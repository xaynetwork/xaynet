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
//!     },
//!     "SumParticipantPublicKey_2": {
//!         "UpdateParticipantPublicKey_1": EncryptedMaskSeed,
//!         "UpdateParticipantPublicKey_2": EncryptedMaskSeed
//!     },
//!     // Mask dict
//!     "mask_submitted": [ // set
//!         SumParticipantPublicKey_1,
//!         SumParticipantPublicKey_2
//!     ],
//!     "mask_dict": [ // sorted set
//!         (mask_object_1, 2), // (mask: bincode encoded string, score/counter: number)
//!         (mask_object_2, 1)
//!     ]
//! }
//! ```
use crate::{
    state_machine::coordinator::CoordinatorState,
    storage::impls::{
        EncryptedMaskSeedRead,
        LocalSeedDictWrite,
        MaskDictIncr,
        MaskObjectRead,
        MaskObjectWrite,
        PublicEncryptKeyRead,
        PublicEncryptKeyWrite,
        PublicSigningKeyRead,
        PublicSigningKeyWrite,
        SeedDictUpdate,
        SumDictAdd,
    },
};
use redis::{aio::ConnectionManager, AsyncCommands, IntoConnectionInfo, RedisResult, Script};
use std::{collections::HashMap, sync::Arc};
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
    /// Returns [`Ok(())`] if field is a new or
    /// [`SumDictAddError::AlreadyExists`] if field already exists.
    ///
    /// [`SumDictAddError::AlreadyExists`]: [crate::storage]
    pub async fn add_sum_participant(
        mut self,
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
    ) -> RedisResult<SumDictAdd> {
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
                -- lua lists (tables) start at 1
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
                        -- This condition should never apply.
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
    pub async fn incr_mask_count(
        mut self,
        sum_pk: &SumParticipantPublicKey,
        mask: &MaskObject,
    ) -> RedisResult<MaskDictIncr> {
        debug!("increment mask count");
        let script = Script::new(
            r#"
                -- lua lists (tables) start at 1
                local sum_pk = ARGV[1]

                -- check if the client participated in sum phase
                --
                -- Note: we cannot delete the sum_pk in the sum_dict because we
                -- need the sum_dict later to delete the seed_dict
                local sum_pk_exist = redis.call("HEXISTS", "sum_dict", sum_pk)
                if sum_pk_exist == 0 then
                    return -1
                end

                -- check if sum participant has not already submitted a mask
                local mask_already_submitted = redis.call("SADD", "mask_submitted", sum_pk)
                -- SADD returns 0 if the key already exists
                if mask_already_submitted == 0 then
                    return -2
                end

                redis.call("ZINCRBY", "mask_dict", 1, KEYS[1])

                return 0
            "#,
        );

        script
            .key(MaskObjectWrite::from(mask))
            .arg(PublicSigningKeyWrite::from(sum_pk))
            .invoke_async(&mut self.connection)
            .await
    }

    /// Retrieves the two masks with the highest score.
    pub async fn get_best_masks(mut self) -> RedisResult<Vec<(MaskObject, u64)>> {
        debug!("get best masks");
        // https://redis.io/commands/zrevrangebyscore
        // > Return value:
        //   Array reply: list of elements in the specified range (optionally with their scores,
        //   in case the WITHSCORES option is given).
        let result: Vec<(MaskObjectRead, u64)> = self
            .connection
            .zrevrange_withscores("mask_dict", 0, 1)
            .await?;

        Ok(result
            .into_iter()
            .map(|(mask, count)| (mask.into(), count))
            .collect())
    }

    /// Retrieves the number of unique masks.
    pub async fn get_number_of_unique_masks(mut self) -> RedisResult<u64> {
        debug!("get number of unique masks");
        // https://redis.io/commands/zcount
        // > Return value:
        //   Integer reply: the number of elements in the specified score range.
        self.connection.zcount("mask_dict", "-inf", "+inf").await
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

        // delete seed dict
        pipe.del("update_participants").ignore();
        for sum_pk in sum_pks {
            pipe.del(sum_pk).ignore();
        }

        // delete mask dict
        pipe.del("mask_submitted").ignore();
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
// Functions that are not needed in the state machine but handy for testing.
impl Connection {
    // Retrieves a [`CoordinatorState`] or `None` when the [`CoordinatorState`] does not exist.
    // currently only used for testing but later required for restoring the coordinator
    async fn get_coordinator_state(mut self) -> RedisResult<Option<CoordinatorState>> {
        // https://redis.io/commands/get
        // > Get the value of key. If the key does not exist the special value nil is returned.
        //   An error is returned if the value stored at key is not a string, because GET only
        //   handles string values.
        // > Return value
        //   Bulk string reply: the value of key, or nil when key does not exist.
        self.connection.get("coordinator_state").await
    }

    // Removes an entry in the [`SumDict`].
    //
    // Returns [`SumDictDelete(Ok(()))`] if field was deleted or
    // [`SumDictDelete(Err(SumDictDeleteError::DoesNotExist)`] if field does not exist.
    pub async fn remove_sum_dict_entry(
        mut self,
        pk: &SumParticipantPublicKey,
    ) -> RedisResult<crate::storage::impls::SumDictDelete> {
        // https://redis.io/commands/hdel
        // > Return value
        //   Integer reply: the number of fields that were removed from the hash,
        //   not including specified but non existing fields.
        self.connection
            .hdel("sum_dict", PublicSigningKeyWrite::from(pk))
            .await
    }

    // Retrieves the length of the [`SumDict`].
    pub async fn get_sum_dict_len(mut self) -> RedisResult<u64> {
        // https://redis.io/commands/hlen
        // > Return value
        //   Integer reply: number of fields in the hash, or 0 when key does not exist.
        self.connection.hlen("sum_dict").await
    }

    // Retrieves the [`SumParticipantPublicKey`] of the [`SumDict`] or an empty list when the
    // [`SumDict`] does not exist.
    pub async fn get_sum_pks(
        mut self,
    ) -> RedisResult<std::collections::HashSet<SumParticipantPublicKey>> {
        // https://redis.io/commands/hkeys
        // > Return value:
        //   Array reply: list of fields in the hash, or an empty list when key does not exist.
        let result: std::collections::HashSet<PublicSigningKeyRead> =
            self.connection.hkeys("sum_dict").await?;
        let sum_pks = result.into_iter().map(|pk| pk.into()).collect();

        Ok(sum_pks)
    }

    // Removes an update pk from the the `update_participants` set.
    pub async fn remove_update_participant(
        mut self,
        update_pk: &UpdateParticipantPublicKey,
    ) -> RedisResult<u64> {
        self.connection
            .srem(
                "update_participants",
                PublicSigningKeyWrite::from(update_pk),
            )
            .await
    }

    pub async fn get_mask_submitted_set(mut self) -> RedisResult<Vec<SumParticipantPublicKey>> {
        let result: Vec<PublicSigningKeyRead> =
            self.connection.smembers("update_submitted").await?;
        let sum_pks = result.into_iter().map(|pk| pk.into()).collect();
        Ok(sum_pks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        state_machine::tests::utils::{mask_settings, model_settings, pet_settings},
        storage::{
            impls::{MaskDictIncrError, SeedDictUpdateError, SumDictAddError, SumDictDeleteError},
            tests::*,
        },
    };
    use serial_test::serial;

    async fn create_redis_client() -> Client {
        Client::new("redis://127.0.0.1/", 10).await.unwrap()
    }

    async fn init_client() -> Client {
        let client = create_redis_client().await;
        client.connection().await.flush_db().await.unwrap();
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
    async fn integration_incr_mask_count() {
        // test the the increment of the mask counter
        let client = init_client().await;

        let should_be_empty = client.connection().await.get_best_masks().await.unwrap();
        assert!(should_be_empty.is_empty());

        let sum_pks = create_and_write_sum_participant_entries(&client, 3).await;
        let mask = create_mask_zeroed(10);
        for sum_pk in sum_pks {
            let res = client
                .connection()
                .await
                .incr_mask_count(&sum_pk, &mask)
                .await;
            assert!(res.is_ok())
        }

        let best_masks = client.connection().await.get_best_masks().await.unwrap();
        assert!(best_masks.len() == 1);

        let (best_mask, count) = best_masks.into_iter().next().unwrap();
        assert_eq!(best_mask, mask);
        assert_eq!(count, 3);
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_incr_mask_count_unknown_sum_pk() {
        // test the writing and reading of one mask
        let client = init_client().await;

        let should_be_empty = client.connection().await.get_best_masks().await.unwrap();
        assert!(should_be_empty.is_empty());

        let (sum_pk, _) = create_sum_participant_entry();
        let mask = create_mask_zeroed(10);
        let unknown_sum_pk = client
            .connection()
            .await
            .incr_mask_count(&sum_pk, &mask)
            .await
            .unwrap();

        assert!(matches!(
            unknown_sum_pk.into_inner().unwrap_err(),
            MaskDictIncrError::UnknownSumPk
        ));
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_incr_mask_count_sum_pk_already_submitted() {
        // test the writing and reading of one mask
        let client = init_client().await;

        let should_be_empty = client.connection().await.get_best_masks().await.unwrap();
        assert!(should_be_empty.is_empty());

        let mut sum_pks = create_and_write_sum_participant_entries(&client, 1).await;
        let sum_pk = sum_pks.pop().unwrap();
        let mask = create_mask_zeroed(10);
        let result = client
            .connection()
            .await
            .incr_mask_count(&sum_pk, &mask)
            .await
            .unwrap();
        assert!(result.is_ok());

        let already_submitted = client
            .connection()
            .await
            .incr_mask_count(&sum_pk, &mask)
            .await
            .unwrap();

        assert!(matches!(
            already_submitted.into_inner().unwrap_err(),
            MaskDictIncrError::MaskAlreadySubmitted
        ));
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_best_masks_only_one_mask() {
        // test the writing and reading of one mask
        let client = init_client().await;

        let should_be_empty = client.connection().await.get_best_masks().await.unwrap();
        assert!(should_be_empty.is_empty());

        let sum_pks = create_and_write_sum_participant_entries(&client, 1).await;
        let mask = create_mask_zeroed(10);
        let res = client
            .connection()
            .await
            .incr_mask_count(sum_pks.get(0).unwrap(), &mask)
            .await;
        assert!(res.is_ok());

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

        let should_be_empty = client.connection().await.get_best_masks().await.unwrap();
        assert!(should_be_empty.is_empty());

        let sum_pks = create_and_write_sum_participant_entries(&client, 2).await;
        let mask_1 = create_mask_zeroed(10);
        for sum_pk in sum_pks {
            let res = client
                .connection()
                .await
                .incr_mask_count(&sum_pk, &mask_1)
                .await;
            assert!(res.is_ok())
        }

        let sum_pks = create_and_write_sum_participant_entries(&client, 1).await;
        let mask_2 = create_mask_zeroed(100);
        for sum_pk in sum_pks {
            let res = client
                .connection()
                .await
                .incr_mask_count(&sum_pk, &mask_2)
                .await;
            assert!(res.is_ok())
        }

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
        let client = init_client().await;

        let best_masks = client.connection().await.get_best_masks().await.unwrap();
        assert!(best_masks.is_empty())
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_number_of_unique_masks_empty() {
        // ensure that get_best_masks returns an empty vec if no mask exist
        let client = init_client().await;

        let number_of_unique_masks = client
            .connection()
            .await
            .get_number_of_unique_masks()
            .await
            .unwrap();
        assert_eq!(number_of_unique_masks, 0)
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_number_of_unique_masks() {
        // ensure that get_best_masks returns an empty vec if no mask exist
        let client = init_client().await;

        let should_be_empty = client.connection().await.get_best_masks().await.unwrap();
        assert!(should_be_empty.is_empty());

        let sum_pks = create_and_write_sum_participant_entries(&client, 4).await;
        for (number, sum_pk) in sum_pks.iter().enumerate() {
            let mask_1 = create_mask(10, number as u32);
            let res = client
                .connection()
                .await
                .incr_mask_count(&sum_pk, &mask_1)
                .await;
            assert!(res.is_ok())
        }

        let number_of_unique_masks = client
            .connection()
            .await
            .get_number_of_unique_masks()
            .await
            .unwrap();
        assert_eq!(number_of_unique_masks, 4)
    }

    #[tokio::test]
    #[serial]
    async fn integration_sum_dict() {
        // test multiple sum dict related methods
        let client = init_client().await;

        // create two entries and write them into redis
        let mut entries = vec![];
        for _ in 0..2 {
            let (pk, epk) = create_sum_participant_entry();
            let add_new_key = client
                .connection()
                .await
                .add_sum_participant(&pk, &epk)
                .await
                .unwrap();
            assert!(add_new_key.is_ok());

            entries.push((pk, epk));
        }

        // ensure that add_sum_participant returns SumDictAddError::AlreadyExists if the key already exist
        let (pk, epk) = entries.get(0).unwrap();
        let key_already_exist = client
            .connection()
            .await
            .add_sum_participant(pk, epk)
            .await
            .unwrap();
        assert!(matches!(
            key_already_exist.into_inner().unwrap_err(),
            SumDictAddError::AlreadyExists
        ));

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

            assert!(remove_sum_pk.is_ok());
        }

        // ensure that add_sum_participant returns SumDictDeleteError::DoesNotExist if the key does not exist
        let (sum_pk, _) = entries.get(0).unwrap();
        let key_does_not_exist = client
            .connection()
            .await
            .remove_sum_dict_entry(sum_pk)
            .await
            .unwrap();
        assert!(matches!(
            key_does_not_exist.into_inner().unwrap_err(),
            SumDictDeleteError::DoesNotExist
        ));

        // ensure that get_sum_dict an empty sum dict
        let sum_dict = client.connection().await.get_sum_dict().await.unwrap();
        assert!(sum_dict.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict() {
        let client = init_client().await;

        let sum_pks = create_and_write_sum_participant_entries(&client, 2).await;
        let local_seed_dicts = create_local_seed_entries(&sum_pks);

        let update_result = write_local_seed_entries(&client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let redis_sum_dict = client.connection().await.get_sum_dict().await.unwrap();
        let seed_dict = create_seed_dict(redis_sum_dict, &local_seed_dicts);

        let redis_seed_dict = client.connection().await.get_seed_dict().await.unwrap();
        assert_eq!(seed_dict, redis_seed_dict)
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_len_mis_match() {
        let client = init_client().await;

        let mut sum_pks = create_and_write_sum_participant_entries(&client, 2).await;

        // remove one sum pk to create invalid local seed dicts
        sum_pks.pop();

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = write_local_seed_entries(&client, &local_seed_dicts).await;
        update_result.into_iter().for_each(|res| {
            assert!(matches!(
                res.into_inner().unwrap_err(),
                SeedDictUpdateError::LengthMisMatch
            ))
        });
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_unknown_sum_participant() {
        let client = init_client().await;

        let mut sum_pks = create_and_write_sum_participant_entries(&client, 2).await;

        // replace a known sum_pk with an unknown one
        sum_pks.pop();
        let (pk, _) = create_sum_participant_entry();
        sum_pks.push(pk);

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = write_local_seed_entries(&client, &local_seed_dicts).await;
        update_result.into_iter().for_each(|res| {
            assert!(matches!(
                res.into_inner().unwrap_err(),
                SeedDictUpdateError::UnknownSumParticipant
            ))
        });
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_update_pk_already_submitted() {
        let client = init_client().await;
        let sum_pks = create_and_write_sum_participant_entries(&client, 2).await;

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = write_local_seed_entries(&client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let update_result = write_local_seed_entries(&client, &local_seed_dicts).await;
        update_result.into_iter().for_each(|res| {
            assert!(matches!(
                res.into_inner().unwrap_err(),
                SeedDictUpdateError::UpdatePkAlreadySubmitted
            ))
        });
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_update_pk_already_exists_in_update_seed_dict() {
        let client = init_client().await;
        let sum_pks = create_and_write_sum_participant_entries(&client, 2).await;

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = write_local_seed_entries(&client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let (update_participant, local_seed_dict) = local_seed_dicts.get(0).unwrap().clone();
        let remove_result = client
            .connection()
            .await
            .remove_update_participant(&update_participant)
            .await
            .unwrap();
        assert_eq!(remove_result, 1);

        let update_result =
            write_local_seed_entries(&client, &[(update_participant, local_seed_dict)]).await;
        update_result.into_iter().for_each(|res| {
            assert!(matches!(
                res.into_inner().unwrap_err(),
                SeedDictUpdateError::UpdatePkAlreadyExistsInUpdateSeedDict
            ))
        });
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_get_seed_dict_for_sum_pk() {
        let client = init_client().await;
        let mut sum_pks = create_and_write_sum_participant_entries(&client, 2).await;

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = write_local_seed_entries(&client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let redis_sum_dict = client.connection().await.get_sum_dict().await.unwrap();
        let seed_dict = create_seed_dict(redis_sum_dict, &local_seed_dicts);

        let sum_pk = sum_pks.pop().unwrap();

        let redis_sum_seed_dict = client
            .connection()
            .await
            .get_seed_dict_for_sum_pk(&sum_pk)
            .await
            .unwrap();

        assert_eq!(&redis_sum_seed_dict, seed_dict.get(&sum_pk).unwrap())
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_get_seed_dict_for_sum_pk_empty() {
        let client = init_client().await;
        let (sum_pk, _) = create_sum_participant_entry();

        let result = client
            .connection()
            .await
            .get_seed_dict_for_sum_pk(&sum_pk)
            .await
            .unwrap();
        assert!(result.is_empty())
    }

    #[tokio::test]
    #[serial]
    async fn integration_flush_dicts_return() {
        let client = init_client().await;

        // write some data into redis
        let set_state = CoordinatorState::new(pet_settings(), mask_settings(), model_settings());
        let res = client
            .connection()
            .await
            .set_coordinator_state(&set_state)
            .await;
        assert!(res.is_ok());

        let sum_pks = create_and_write_sum_participant_entries(&client, 2).await;

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = write_local_seed_entries(&client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let mask = create_mask_zeroed(10);
        client
            .connection()
            .await
            .incr_mask_count(sum_pks.get(0).unwrap(), &mask)
            .await
            .unwrap();

        // remove dicts
        let res = client.connection().await.flush_dicts().await;
        assert!(res.is_ok());

        // ensure that only the coordinator state exists
        let res = client.connection().await.get_coordinator_state().await;
        assert!(res.unwrap().is_some());

        let res = client.connection().await.get_sum_dict().await;
        assert!(res.unwrap().is_empty());

        let res = client.connection().await.get_seed_dict().await;
        assert!(res.unwrap().is_empty());

        let res = client.connection().await.get_mask_submitted_set().await;
        assert!(res.unwrap().is_empty());

        let res = client.connection().await.get_best_masks().await;
        assert!(res.unwrap().is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn integration_ping() {
        // test ping command
        let client = init_client().await;

        let res = client.connection().await.ping().await;
        assert!(res.is_ok())
    }
}
