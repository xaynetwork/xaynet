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
        EncryptedMaskSeedRead,
        LocalSeedDictWrite,
        MaskObjectRead,
        MaskObjectWrite,
        PublicEncryptKeyRead,
        PublicEncryptKeyWrite,
        PublicSigningKeyRead,
        PublicSigningKeyWrite,
        SeedDictUpdate,
    },
};
use redis::{aio::ConnectionManager, AsyncCommands, IntoConnectionInfo, RedisResult, Script};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use xaynet_core::{
    mask::{EncryptedMaskSeed, MaskMany},
    LocalSeedDict,
    SeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

pub use redis::RedisError;

#[derive(Clone)]
pub struct Client {
    raw_connection: ConnectionManager,
    semaphore: Arc<Semaphore>,
}

#[cfg(test)]
impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

    /// Updates the [`SeedDict`] with the seeds from the given ['UpdateParticipantPublicKey'].
    pub async fn update_seed_dict(
        mut self,
        update_pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> RedisResult<SeedDictUpdate> {
        debug!(
            "update seed dictionary for update participant with pk {:?}",
            update_pk
        );
        let script = Script::new(
            r#"
                local update_pk = ARGV[1]

                -- check if the local seed dict has the same length as the sum_dict

                -- KEYS is a list (table) of key value pairs ([update_pk_1, seed_1, update_pk_2, seed_2])
                local seed_dict_len = #KEYS / 2
                local sum_dict_len = redis.call("HLEN", "sum_dict")
                if seed_dict_len ~= sum_dict_len then
                    return -1
                end

                -- check if all pks of the local seed dict exists in sum_dict
                for i = 1, #KEYS, 2 do
                    local exist_in_sum_dict = redis.call("HEXISTS", "sum_dict", KEYS[i])
                    if exist_in_sum_dict == 0 then
                        return -2
                    end
                end

                -- check if one pk of the local seed dict already exists in seed_dict
                local exist_in_seed_dict = redis.call("SADD", "update_participants", update_pk)
                -- SADD returns 0 if the key already exists
                if exist_in_seed_dict == 0 then
                    return -3
                end

                -- update the seed dict
                for i = 1, #KEYS, 2 do
                    local exist_in_update_seed_dict = redis.call("HSETNX", KEYS[i], update_pk, KEYS[i + 1])
                    -- HSETNX returns 0 if the key already exists
                    if exist_in_update_seed_dict == 0 then
                        -- This condition and should never apply.
                        -- If this condition is true, it is an indication that the data in redis is corrupted.
                        return -4
                    end
                end

                return 0
            "#,
        );

        script
            .key(LocalSeedDictWrite::from(local_seed_dict))
            .arg(PublicSigningKeyWrite::from(update_pk))
            .invoke_async(&mut self.connection)
            .await
    }

    /// Retrieves the [`SeedDict`] or an empty [`SeedDict`] when the [`SumDict`] does not exist.
    ///
    /// # Note
    /// This method is **not** an atomic operation.
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

    /// Updates the mask dictionary with the given [`MaskObject`].
    ///
    /// The score/counter of the given mask is incremented by `1`.
    /// The maximum length of a serialized mask is 512 Megabytes.
    pub async fn incr_mask_count(mut self, mask: &MaskMany) -> RedisResult<()> {
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
    pub async fn get_best_masks(mut self) -> RedisResult<Vec<(MaskMany, usize)>> {
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
    ///
    /// # Note
    /// This method is **not** an atomic operation.
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
        mask::{BoundType, DataType, GroupType, MaskConfig, MaskMany, ModelType},
    };

    fn create_mask(byte_size: usize) -> MaskMany {
        let config = MaskConfig {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };

        MaskMany::new(config, vec![BigUint::zero(); byte_size])
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
