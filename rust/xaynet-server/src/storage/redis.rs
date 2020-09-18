//! # Redis Data Model
//!
//!```text
//! {
//!     // Coordinator state
//!     "coordinator_state": "...", // bincode encoded string
//!     // Sum dict
//!     "sum_dict": { // hash
//!         "SumParticipantPublicKey_1": SumParticipantEphemeralPublicKey_1,
//!         "SumParticipantPublicKey_2": SumParticipantEphemeralPublicKey_2
//!     },
//!     // Seed dict
//!     "update_participants": [ // set
//!         UpdateParticipantPublicKey_1,
//!         UpdateParticipantPublicKey_2
//!     ],
//!     "SumParticipantPublicKey_1": { // hash
//!         "UpdateParticipantPublicKey_1": EncryptedMaskSeed,
//!         "UpdateParticipantPublicKey_2": EncryptedMaskSeed
//!     }
//!     "SumParticipantPublicKey_2": {
//!         "UpdateParticipantPublicKey_1": EncryptedMaskSeed,
//!         "UpdateParticipantPublicKey_2": EncryptedMaskSeed
//!     }
//!     // Mask dict
//!     "mask_dict": [ // sorted set
//!         (mask_object_1, 12341), // (mask: bincode encoded string, score/counter: number)
//!         (mask_object_2, 1)
//!     ]
//! }
//! ```
use crate::{
    state_machine::coordinator::CoordinatorState,
    storage::impls::{
        AddSumParticipant,
        DeleteSumParticipant,
        EncryptedMaskSeedRead,
        EncryptedMaskSeedWrite,
        MaskObjectRead,
        MaskObjectWrite,
        PublicEncryptKeyRead,
        PublicEncryptKeyWrite,
        PublicSigningKeyRead,
        PublicSigningKeyWrite,
    },
};
pub use redis::RedisError;
use redis::{aio::ConnectionManager, AsyncCommands, IntoConnectionInfo, RedisResult};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use xaynet_core::{
    mask::{EncryptedMaskSeed, MaskObject},
    LocalSeedDict,
    SeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

#[derive(Clone)]
pub struct Client {
    raw_connection: ConnectionManager,
    semaphore: Arc<Semaphore>,
}

#[cfg(test)]
use std::fmt;
#[cfg(test)]
impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("semaphore", &self.semaphore)
            .finish()
    }
}

pub struct Connection {
    connection: ConnectionManager,
    _permit: OwnedSemaphorePermit,
}

impl Client {
    /// Creates a new Redis client.
    ///
    /// `url` to which Redis instance the client should connect to.
    /// The URL format is `redis://[<username>][:<passwd>@]<hostname>[:port][/<db>]`.
    /// `n` is the maximum number of concurrent uses on a shared connection.
    ///
    /// The [`Client`] uses a [`redis::aio::ConnectionManager`] that automatically reconnects
    /// if the connection is dropped.
    pub async fn new<T: IntoConnectionInfo>(url: T, n: usize) -> Result<Self, RedisError> {
        let client = redis::Client::open(url)?;
        let connection = client.get_tokio_connection_manager().await?;
        Ok(Self {
            raw_connection: connection,
            semaphore: Arc::new(Semaphore::new(n)),
        })
    }

    /// Acquires access to the shared connection.
    ///
    /// If the maximum number of concurrent uses the shared connection is reached,
    /// the method will wait until a pending usage is completed.
    pub async fn connection(&self) -> Connection {
        let Client {
            raw_connection,
            semaphore,
        } = self.clone();

        let _permit = semaphore.acquire_owned().await;
        Connection {
            connection: raw_connection,
            _permit,
        }
    }
}

impl Connection {
    /// Retrieves a [`CoordinatorState`] or `None` when the [`CoordinatorState`] does not exist.
    // currently only used for testing but later required for restoring the coordinator
    #[allow(dead_code)]
    async fn get_coordinator_state(mut self) -> RedisResult<Option<CoordinatorState>> {
        debug!("get coordinator state");
        // https://redis.io/commands/get
        // > Get the value of key. If the key does not exist the special value nil is returned.
        //   An error is returned if the value stored at key is not a string, because GET only
        //   handles string values.
        // > Return value
        //   Bulk string reply: the value of key, or nil when key does not exist.
        self.connection.get("coordinator_state").await
    }

    /// Stores a [`CoordinatorState`].
    ///
    /// If the coordinator state already exists, it is overwritten.
    pub async fn set_coordinator_state(mut self, state: &CoordinatorState) -> RedisResult<()> {
        debug!("set coordinator state");
        // https://redis.io/commands/set
        // > Set key to hold the string value. If key already holds a value,
        //   it is overwritten, regardless of its type.
        // Possible return value in our case:
        // > Simple string reply: OK if SET was executed correctly.
        self.connection.set("coordinator_state", state).await
    }

    /// Retrieves the [`SumDict`].
    pub async fn get_sum_dict(mut self) -> RedisResult<SumDict> {
        debug!("get sum dictionary");
        // https://redis.io/commands/hgetall
        // > Return value
        //   Array reply: list of fields and their values stored in the hash, or an empty
        //   list when key does not exist.
        let result: Vec<(PublicSigningKeyRead, PublicEncryptKeyRead)> =
            self.connection.hgetall("sum_dict").await?;
        let sum_dict = result
            .into_iter()
            .map(|(pk, ephm_pk)| (pk.into(), ephm_pk.into()))
            .collect();

        Ok(sum_dict)
    }

    /// Stores a new [`SumDict`] entry.
    ///
    /// Returns [`AddSumParticipant::Ok`] if field is a new or
    /// [`AddSumParticipant::AlreadyExists`] if field already exists.
    pub async fn add_sum_participant(
        mut self,
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
    ) -> RedisResult<AddSumParticipant> {
        debug!("add sum participant with pk {:?}", pk);
        // https://redis.io/commands/hsetnx
        // > If field already exists, this operation has no effect.
        // > Return value
        //   Integer reply, specifically:
        //   1 if field is a new field in the hash and value was set.
        //   0 if field already exists in the hash and no operation was performed.
        self.connection
            .hset_nx(
                "sum_dict",
                PublicSigningKeyWrite::from(pk),
                PublicEncryptKeyWrite::from(ephm_pk),
            )
            .await
    }

    /// Removes an entry in the [`SumDict`].
    ///
    /// Returns [`DeleteSumParticipant::Ok`] if field was deleted or
    /// [`DeleteSumParticipant::DoesNotExist`] if field does not exist.
    pub async fn remove_sum_dict_entry(
        mut self,
        pk: &SumParticipantPublicKey,
    ) -> RedisResult<DeleteSumParticipant> {
        debug!(
            "remove sum dictionary entry for sum participant with pk {:?}",
            pk
        );
        // https://redis.io/commands/hdel
        // > Return value
        //   Integer reply: the number of fields that were removed from the hash,
        //   not including specified but non existing fields.
        self.connection
            .hdel("sum_dict", PublicSigningKeyWrite::from(pk))
            .await
    }

    /// Retrieves the length of the [`SumDict`].
    pub async fn get_sum_dict_len(mut self) -> RedisResult<u64> {
        debug!("get length of sum dictionary");
        // https://redis.io/commands/hlen
        // > Return value
        //   Integer reply: number of fields in the hash, or 0 when key does not exist.
        self.connection.hlen("sum_dict").await
    }

    /// Retrieves the [`SumParticipantPublicKey`] of the [`SumDict`] or an empty list when the
    /// [`SumDict`] does not exist.
    pub async fn get_sum_pks(mut self) -> RedisResult<HashSet<SumParticipantPublicKey>> {
        debug!("get public keys of all sum participants");
        // https://redis.io/commands/hkeys
        // > Return value:
        //   Array reply: list of fields in the hash, or an empty list when key does not exist.
        let result: HashSet<PublicSigningKeyRead> = self.connection.hkeys("sum_dict").await?;
        let sum_pks = result.into_iter().map(|pk| pk.into()).collect();

        Ok(sum_pks)
    }

    /// Retrieves the [`SeedDict`] entry for the given ['SumParticipantPublicKey'] or an empty map
    /// when a [`SeedDict`] entry does not exist.
    pub async fn get_seed_dict_for_sum_pk(
        mut self,
        sum_pk: &SumParticipantPublicKey,
    ) -> RedisResult<HashMap<UpdateParticipantPublicKey, EncryptedMaskSeed>> {
        debug!(
            "get seed dictionary for sum participant with pk {:?}",
            sum_pk
        );
        // https://redis.io/commands/hgetall
        // > Return value
        //   Array reply: list of fields and their values stored in the hash, or an empty
        //   list when key does not exist.
        let result: Vec<(PublicSigningKeyRead, EncryptedMaskSeedRead)> = self
            .connection
            .hgetall(PublicSigningKeyWrite::from(sum_pk))
            .await?;
        let seed_dict = result
            .into_iter()
            .map(|(pk, seed)| (pk.into(), seed.into()))
            .collect();

        Ok(seed_dict)
    }

    /// Retrieves the [`SeedDict`] or an empty [`SeedDict`] when the [`SumDict`] does not exist.
    pub async fn get_seed_dict(mut self) -> RedisResult<SeedDict> {
        debug!("get seed dictionary");
        // https://redis.io/commands/hkeys
        // > Return value:
        //   Array reply: list of fields in the hash, or an empty list when key does not exist.
        let sum_pks: Vec<PublicSigningKeyRead> = self.connection.hkeys("sum_dict").await?;

        let mut seed_dict: SeedDict = SeedDict::new();
        for sum_pk in sum_pks {
            // https://redis.io/commands/hgetall
            // > Return value
            //   Array reply: list of fields and their values stored in the hash, or an empty
            //   list when key does not exist.
            let sum_pk_seed_dict: HashMap<PublicSigningKeyRead, EncryptedMaskSeedRead> =
                self.connection.hgetall(&sum_pk).await?;
            seed_dict.insert(
                sum_pk.into(),
                sum_pk_seed_dict
                    .into_iter()
                    .map(|(pk, seed)| (pk.into(), seed.into()))
                    .collect(),
            );
        }

        Ok(seed_dict)
    }

    /// Updates the [`SeedDict`] with the seeds from the given ['UpdateParticipantPublicKey'].
    pub async fn update_seed_dict(
        mut self,
        update_pk: &UpdateParticipantPublicKey,
        update: &LocalSeedDict,
    ) -> RedisResult<()> {
        debug!(
            "update seed dictionary for update participant with pk {:?}",
            update_pk
        );
        // Note:
        // Redis supports transactions, but not rollbacks. This means, that if a Redis command fails
        // in a multi-command transaction, Redis will still execute the rest of the transaction.
        // It is important to understand under which circumstances a Redis command can fail:
        //
        // https://redis.io/topics/transactions
        // > Redis commands can fail only if called with a wrong syntax (and the problem is not detectable
        //   during the command queueing), or against keys holding the wrong data type: this means that in
        //   practical terms a failing command is the result of a programming errors, and a kind of error
        //   that is very likely to be detected during development, and not in production.
        //
        // Side note: In addition to the `TypeError`, the crate `redis` adds further error kinds.
        // https://docs.rs/redis/0.17.0/redis/enum.ErrorKind.html
        // Mostly related to IO, connection issues and redis cluster features.
        //
        // This means, that Redis does not abort a transaction if for example the
        // command "hset_nx" returns `0` (value already exists).
        //
        // As a result, this method is successful even though one or all of
        // the following points apply:
        // - `sum_pk` is not in the `sum_dict`
        // - `update_pk` already exists (however, in this case the seed is not updated)
        // - the `LocalSeedDict` has a different length/update_pks than the `sum_dict`.
        //
        // We can create our own transactions via lua scripts. In a script we can
        // perform all the checks first and only write the data if the checks were successful.
        // However, in our case, the checks can be quite expensive.
        // Therefore we should agree on which checks are necessary.

        let mut pipe = redis::pipe();

        // https://redis.io/commands/sadd
        // > Specified members that are already a member of this set are ignored.
        //   If key does not exist, a new set is created before adding the specified members.
        //   An error is returned when the value stored at key is not a set.
        // > Return value
        //   Integer reply: the number of elements that were added to the set, not including all the
        //   elements already present into the set.
        //
        // TODO: not sure if we need this here. We used the Set in #394 to count and return
        // the number of update participants. However, we can not rely on this returned
        // number when we later process the updates in parallel.
        // (the responses will likely be received by the coordinator in a different order than they were sent)
        // We can add a separate method that returns the number of update participants and check at
        // the end of the update phase if this number (number of update participants) is equal to
        // the number (number of successful update messages) in the coordinator.
        pipe.sadd(
            "update_participants",
            PublicSigningKeyWrite::from(update_pk),
        )
        .ignore();

        // https://redis.io/commands/hsetnx
        // > Sets field in the hash stored at key to value, only if field does not yet exist.
        //   If key does not exist, a new key holding a hash is created. If field already exists,
        //   this operation has no effect.
        // > Return value
        //   Integer reply, specifically:
        //   1 if field is a new field in the hash and value was set.
        //   0 if field already exists in the hash and no operation was performed.
        //
        // The return value `0` is not interpreted as error in Redis.
        // TODO: Is it ok to ignore the returned value?
        for (sum_pk, encr_seed) in update {
            pipe.hset_nx(
                PublicSigningKeyWrite::from(sum_pk),
                PublicSigningKeyWrite::from(update_pk),
                EncryptedMaskSeedWrite::from(encr_seed),
            )
            .ignore();
        }
        pipe.atomic().query_async(&mut self.connection).await
    }

    /// Updates the mask dictionary with the given [`MaskObject`].
    ///
    /// The score/counter of the given mask is incremented by `1`.
    /// The maximum length of a serialized mask is 512 Megabytes.
    pub async fn incr_mask_count(mut self, mask: &MaskObject) -> RedisResult<()> {
        debug!("increment mask count");
        // https://redis.io/commands/zincrby
        // > Return value
        //   Bulk string reply: the new score of member (a double precision floating point number),
        //   represented as string.
        //
        // We ignore the return value because we are not interested in it. We will use the method
        // `get_best_masks` instead.
        self.connection
            .zincr("mask_dict", MaskObjectWrite::from(mask), 1_usize)
            .await
    }

    /// Retrieves the two masks with the highest score.
    pub async fn get_best_masks(mut self) -> RedisResult<Vec<(MaskObject, usize)>> {
        debug!("get best masks");
        // https://redis.io/commands/zrevrangebyscore
        // > Return value:
        //   Array reply: list of elements in the specified range (optionally with their scores,
        //   in case the WITHSCORES option is given).
        let result: Vec<(MaskObjectRead, usize)> = self
            .connection
            .zrevrange_withscores("mask_dict", 0, 1)
            .await?;

        Ok(result
            .into_iter()
            .map(|(mask, count)| (mask.into(), count))
            .collect())
    }

    /// Deletes all data in the current database.
    pub async fn flush_db(mut self) -> RedisResult<()> {
        debug!("flush current database");
        // https://redis.io/commands/flushdb
        // > This command never fails.
        redis::cmd("FLUSHDB")
            .arg("ASYNC")
            .query_async(&mut self.connection)
            .await
    }

    /// Deletes the dictionaries [`SumDict`], [`SeedDict`] and mask dictionary.
    pub async fn flush_dicts(mut self) -> RedisResult<()> {
        debug!("flush all dictionaries");
        // https://redis.io/commands/hkeys
        // > Return value:
        //   Array reply: list of fields in the hash, or an empty list when key does not exist.
        let sum_pks: Vec<PublicSigningKeyRead> = self.connection.hkeys("sum_dict").await?;
        let mut pipe = redis::pipe();

        // https://redis.io/commands/del
        // > Return value:
        //   The number of keys that were removed.
        //
        // Returns `0` if the key does not exist.
        // We ignore the return value because we are not interested in it.

        // delete sum dict
        pipe.del("sum_dict").ignore();

        //delete seed dict
        pipe.del("update_participants").ignore();
        for sum_pk in sum_pks {
            pipe.del(sum_pk).ignore();
        }

        //delete mask dict
        pipe.del("mask_dict").ignore();
        pipe.atomic().query_async(&mut self.connection).await
    }

    /// Pings the Redis server. Useful for checking whether there is a connection
    /// between the client and Redis.
    pub async fn ping(mut self) -> RedisResult<()> {
        // https://redis.io/commands/ping
        redis::cmd("PING").query_async(&mut self.connection).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_machine::tests::utils::{mask_settings, model_settings, pet_settings};
    use num::{bigint::BigUint, traits::identities::Zero};
    use serial_test::serial;
    use xaynet_core::{
        crypto::{EncryptKeyPair, SigningKeyPair},
        mask::{BoundType, DataType, GroupType, MaskConfig, MaskObject, ModelType},
    };

    fn create_mask(byte_size: usize) -> MaskObject {
        let config = MaskConfig {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };

        MaskObject::new(config, vec![BigUint::zero(); byte_size])
    }

    async fn flush_db(client: &Client) {
        client.connection().await.flush_db().await.unwrap();
    }

    async fn create_redis_client() -> Client {
        Client::new("redis://127.0.0.1/", 10).await.unwrap()
    }

    async fn init_client() -> Client {
        let client = create_redis_client().await;
        flush_db(&client).await;
        client
    }

    #[tokio::test]
    #[serial]
    async fn integration_set_and_get_coordinator_state() {
        // test the writing and reading of the coordinator state
        let client = init_client().await;

        let set_state = CoordinatorState::new(pet_settings(), mask_settings(), model_settings());
        client
            .connection()
            .await
            .set_coordinator_state(&set_state)
            .await
            .unwrap();

        let get_state = client
            .connection()
            .await
            .get_coordinator_state()
            .await
            .unwrap()
            .unwrap();

        assert_eq!(set_state, get_state)
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_best_masks_one_mask() {
        // test the writing and reading of one mask
        let client = init_client().await;

        let mask = create_mask(10);
        client
            .connection()
            .await
            .incr_mask_count(&mask)
            .await
            .unwrap();

        let best_masks = client.connection().await.get_best_masks().await.unwrap();
        assert!(best_masks.len() == 1);

        let (best_mask, count) = best_masks.into_iter().next().unwrap();
        assert_eq!(best_mask, mask);
        assert_eq!(count, 1);
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_best_masks_two_masks() {
        // test the writing and reading of two masks
        // the first mask is incremented twice
        let client = init_client().await;

        let mask_1 = create_mask(10);
        client
            .connection()
            .await
            .incr_mask_count(&mask_1)
            .await
            .unwrap();
        client
            .connection()
            .await
            .incr_mask_count(&mask_1)
            .await
            .unwrap();

        let mask_2 = create_mask(100);
        client
            .connection()
            .await
            .incr_mask_count(&mask_2)
            .await
            .unwrap();

        let best_masks = client.connection().await.get_best_masks().await.unwrap();
        assert!(best_masks.len() == 2);
        let mut best_masks_iter = best_masks.into_iter();

        let (first_mask, count) = best_masks_iter.next().unwrap();
        assert_eq!(first_mask, mask_1);
        assert_eq!(count, 2);
        let (second_mask, count) = best_masks_iter.next().unwrap();
        assert_eq!(second_mask, mask_2);
        assert_eq!(count, 1);
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_best_masks_no_mask() {
        // ensure that get_best_masks returns an empty vec if no mask exist
        let client = create_redis_client().await;
        flush_db(&client).await;

        let best_masks = client.connection().await.get_best_masks().await.unwrap();
        assert!(best_masks.is_empty())
    }

    #[tokio::test]
    #[serial]
    async fn integration_sum_dict() {
        // test multiple sum dict related methods
        let client = init_client().await;

        // create two entries and write them into redis
        let mut entries = vec![];
        for _ in 0..2 {
            let SigningKeyPair { public: pk, .. } = SigningKeyPair::generate();
            let EncryptKeyPair { public: epk, .. } = EncryptKeyPair::generate();
            entries.push((pk.clone(), epk.clone()));

            let add_new_key = client
                .connection()
                .await
                .add_sum_participant(&pk, &epk)
                .await
                .unwrap();
            assert_eq!(add_new_key, AddSumParticipant::Ok)
        }

        // ensure that add_sum_participant returns AddSumParticipant::AlreadyExists if the key already exist
        let (pk, epk) = entries.get(0).unwrap();
        let key_already_exist = client
            .connection()
            .await
            .add_sum_participant(pk, epk)
            .await
            .unwrap();
        assert_eq!(key_already_exist, AddSumParticipant::AlreadyExists);

        // ensure that get_sum_dict_len returns 2
        let len_of_sum_dict = client.connection().await.get_sum_dict_len().await.unwrap();
        assert_eq!(len_of_sum_dict, 2);

        // read the written sum keys
        // ensure they are equal
        let sum_pks = client.connection().await.get_sum_pks().await.unwrap();
        for (sum_pk, _) in entries.iter() {
            assert!(sum_pks.contains(sum_pk));
        }

        // remove both sum entries
        for (sum_pk, _) in entries.iter() {
            let remove_sum_pk = client
                .connection()
                .await
                .remove_sum_dict_entry(sum_pk)
                .await
                .unwrap();
            assert_eq!(remove_sum_pk, DeleteSumParticipant::Ok);
        }

        // ensure that add_sum_participant returns AddSumParticipant::AlreadyExists if the key already exist
        let (sum_pk, _) = entries.get(0).unwrap();
        let key_does_not_exist = client
            .connection()
            .await
            .remove_sum_dict_entry(sum_pk)
            .await
            .unwrap();
        assert_eq!(key_does_not_exist, DeleteSumParticipant::DoesNotExist);

        // ensure that get_sum_dict an empty sum dict
        let sum_dict = client.connection().await.get_sum_dict().await.unwrap();
        assert_eq!(sum_dict.len(), 0);
    }

    #[tokio::test]
    #[serial]
    async fn integration_flush_dicts_return() {
        let client = init_client().await;

        let res = client.connection().await.flush_db().await;
        assert!(res.is_ok())
    }
}
