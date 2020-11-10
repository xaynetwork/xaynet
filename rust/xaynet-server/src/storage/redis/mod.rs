//! A Redis compatible [`CoordinatorStore`].
//!
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
//!     ],
//!     "latest_global_model_id": global_model_id
//! }
//! ```

mod impls;

use std::collections::HashMap;

use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands, IntoConnectionInfo, Pipeline, Script};
pub use redis::{RedisError, RedisResult};
use tracing::debug;

use self::impls::{
    EncryptedMaskSeedRead,
    LocalSeedDictWrite,
    MaskObjectRead,
    MaskObjectWrite,
    PublicEncryptKeyRead,
    PublicEncryptKeyWrite,
    PublicSigningKeyRead,
    PublicSigningKeyWrite,
};
use crate::{
    state_machine::coordinator::CoordinatorState,
    storage::{
        CoordinatorStorage,
        LocalSeedDictAdd,
        MaskScoreIncr,
        StorageError,
        StorageResult,
        SumPartAdd,
    },
};
use xaynet_core::{
    mask::MaskObject,
    LocalSeedDict,
    SeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

#[derive(Clone)]
pub struct Client {
    connection: ConnectionManager,
}

#[cfg(test)]
impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Redis client").finish()
    }
}

fn to_storage_err(e: RedisError) -> StorageError {
    anyhow::anyhow!(e)
}

impl Client {
    /// Creates a new Redis client.
    ///
    /// `url` to which Redis instance the client should connect to.
    /// The URL format is `redis://[<username>][:<passwd>@]<hostname>[:port][/<db>]`.
    ///
    /// The [`Client`] uses a [`redis::aio::ConnectionManager`] that automatically reconnects
    /// if the connection is dropped.
    pub async fn new<T: IntoConnectionInfo>(url: T) -> Result<Self, RedisError> {
        let client = redis::Client::open(url)?;
        let connection = client.get_tokio_connection_manager().await?;
        Ok(Self { connection })
    }

    async fn create_flush_dicts_pipeline(&mut self) -> RedisResult<Pipeline> {
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
        Ok(pipe)
    }
}

#[async_trait]
impl CoordinatorStorage for Client {
    /// See [`CoordinatorStorage::set_coordinator_state`].
    async fn set_coordinator_state(&mut self, state: &CoordinatorState) -> StorageResult<()> {
        debug!("set coordinator state");
        // https://redis.io/commands/set
        // > Set key to hold the string value. If key already holds a value,
        //   it is overwritten, regardless of its type.
        // Possible return value in our case:
        // > Simple string reply: OK if SET was executed correctly.
        self.connection
            .set("coordinator_state", state)
            .await
            .map_err(to_storage_err)
    }

    /// See [`CoordinatorStorage::coordinator_state`].
    async fn coordinator_state(&mut self) -> StorageResult<Option<CoordinatorState>> {
        // https://redis.io/commands/get
        // > Get the value of key. If the key does not exist the special value nil is returned.
        //   An error is returned if the value stored at key is not a string, because GET only
        //   handles string values.
        // > Return value
        //   Bulk string reply: the value of key, or nil when key does not exist.
        self.connection
            .get("coordinator_state")
            .await
            .map_err(to_storage_err)
    }

    /// See [`CoordinatorStorage::add_sum_participant`].
    async fn add_sum_participant(
        &mut self,
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
    ) -> StorageResult<SumPartAdd> {
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
            .map_err(to_storage_err)
    }

    /// See [`CoordinatorStorage::sum_dict`].
    async fn sum_dict(&mut self) -> StorageResult<Option<SumDict>> {
        debug!("get sum dictionary");
        // https://redis.io/commands/hgetall
        // > Return value
        //   Array reply: list of fields and their values stored in the hash, or an empty
        //   list when key does not exist.
        let reply: Vec<(PublicSigningKeyRead, PublicEncryptKeyRead)> = self
            .connection
            .hgetall("sum_dict")
            .await
            .map_err(to_storage_err)?;

        if reply.is_empty() {
            return Ok(None);
        };

        let sum_dict = reply
            .into_iter()
            .map(|(pk, ephm_pk)| (pk.into(), ephm_pk.into()))
            .collect();

        Ok(Some(sum_dict))
    }

    /// See [`CoordinatorStorage::add_local_seed_dict`].
    async fn add_local_seed_dict(
        &mut self,
        update_pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> StorageResult<LocalSeedDictAdd> {
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
            .map_err(to_storage_err)
    }

    /// See [`CoordinatorStorage::seed_dict`].
    ///
    /// # Note
    /// This method is **not** an atomic operation.
    async fn seed_dict(&mut self) -> StorageResult<Option<SeedDict>> {
        debug!("get seed dictionary");
        // https://redis.io/commands/hkeys
        // > Return value:
        //   Array reply: list of fields in the hash, or an empty list when key does not exist.
        let sum_pks: Vec<PublicSigningKeyRead> = self.connection.hkeys("sum_dict").await?;

        if sum_pks.is_empty() {
            return Ok(None);
        };

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

        Ok(Some(seed_dict))
    }

    /// See [`CoordinatorStorage::incr_mask_score`].
    ///
    /// The maximum length of a serialized mask is 512 Megabytes.
    async fn incr_mask_score(
        &mut self,
        sum_pk: &SumParticipantPublicKey,
        mask: &MaskObject,
    ) -> StorageResult<MaskScoreIncr> {
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
            .map_err(to_storage_err)
    }

    /// See [`CoordinatorStorage::best_masks`].
    async fn best_masks(&mut self) -> StorageResult<Option<Vec<(MaskObject, u64)>>> {
        debug!("get best masks");
        // https://redis.io/commands/zrevrangebyscore
        // > Return value:
        //   Array reply: list of elements in the specified range (optionally with their scores,
        //   in case the WITHSCORES option is given).
        let reply: Vec<(MaskObjectRead, u64)> = self
            .connection
            .zrevrange_withscores("mask_dict", 0, 1)
            .await?;

        let result = match reply.is_empty() {
            true => None,
            _ => {
                let masks = reply
                    .into_iter()
                    .map(|(mask, count)| (mask.into(), count))
                    .collect();

                Some(masks)
            }
        };

        Ok(result)
    }

    /// See [`CoordinatorStorage::number_of_unique_masks`].
    async fn number_of_unique_masks(&mut self) -> StorageResult<u64> {
        debug!("get number of unique masks");
        // https://redis.io/commands/zcount
        // > Return value:
        //   Integer reply: the number of elements in the specified score range.
        self.connection
            .zcount("mask_dict", "-inf", "+inf")
            .await
            .map_err(to_storage_err)
    }

    /// See [`CoordinatorStorage::delete_coordinator_data`].
    ///
    /// # Note
    /// This method is **not** an atomic operation.
    async fn delete_coordinator_data(&mut self) -> StorageResult<()> {
        debug!("flush coordinator data");
        let mut pipe = self.create_flush_dicts_pipeline().await?;
        pipe.del("coordinator_state").ignore();
        pipe.del("latest_global_model_id").ignore();
        pipe.atomic()
            .query_async(&mut self.connection)
            .await
            .map_err(to_storage_err)
    }

    /// See [`CoordinatorStorage::delete_dicts`].
    ///
    /// # Note
    /// This method is **not** an atomic operation.
    async fn delete_dicts(&mut self) -> StorageResult<()> {
        debug!("flush all dictionaries");
        let mut pipe = self.create_flush_dicts_pipeline().await?;
        pipe.atomic()
            .query_async(&mut self.connection)
            .await
            .map_err(to_storage_err)
    }

    /// See [`CoordinatorStorage::set_latest_global_model_id`].
    async fn set_latest_global_model_id(&mut self, global_model_id: &str) -> StorageResult<()> {
        debug!("set latest global model with id {}", global_model_id);
        // https://redis.io/commands/set
        // > Set key to hold the string value. If key already holds a value,
        //   it is overwritten, regardless of its type.
        // Possible return value in our case:
        // > Simple string reply: OK if SET was executed correctly.
        self.connection
            .set("latest_global_model_id", global_model_id)
            .await
            .map_err(to_storage_err)
    }

    /// See [`CoordinatorStorage::latest_global_model_id`].
    async fn latest_global_model_id(&mut self) -> StorageResult<Option<String>> {
        debug!("get latest global model id");
        // https://redis.io/commands/get
        // > Get the value of key. If the key does not exist the special value nil is returned.
        //   An error is returned if the value stored at key is not a string, because GET only
        //   handles string values.
        // > Return value
        //   Bulk string reply: the value of key, or nil when key does not exist.
        self.connection
            .get("latest_global_model_id")
            .await
            .map_err(to_storage_err)
    }
}

#[cfg(test)]
// Functions that are not needed in the state machine but handy for testing.
impl Client {
    // Removes an entry in the [`SumDict`].
    //
    // Returns [`SumDictDelete(Ok(()))`] if field was deleted or
    // [`SumDictDelete(Err(SumDictDeleteError::DoesNotExist)`] if field does not exist.
    pub async fn remove_sum_dict_entry(
        &mut self,
        pk: &SumParticipantPublicKey,
    ) -> RedisResult<self::impls::SumDictDelete> {
        // https://redis.io/commands/hdel
        // > Return value
        //   Integer reply: the number of fields that were removed from the hash,
        //   not including specified but non existing fields.
        self.connection
            .hdel("sum_dict", PublicSigningKeyWrite::from(pk))
            .await
    }

    // Returns the length of the [`SumDict`].
    pub async fn sum_dict_len(&mut self) -> RedisResult<u64> {
        // https://redis.io/commands/hlen
        // > Return value
        //   Integer reply: number of fields in the hash, or 0 when key does not exist.
        self.connection.hlen("sum_dict").await
    }

    // Returns the [`SumParticipantPublicKey`] of the [`SumDict`] or an empty list when the
    // [`SumDict`] does not exist.
    pub async fn sum_pks(
        &mut self,
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
        &mut self,
        update_pk: &UpdateParticipantPublicKey,
    ) -> RedisResult<u64> {
        self.connection
            .srem(
                "update_participants",
                PublicSigningKeyWrite::from(update_pk),
            )
            .await
    }

    pub async fn mask_submitted_set(&mut self) -> RedisResult<Vec<SumParticipantPublicKey>> {
        let result: Vec<PublicSigningKeyRead> =
            self.connection.smembers("update_submitted").await?;
        let sum_pks = result.into_iter().map(|pk| pk.into()).collect();
        Ok(sum_pks)
    }

    // Returns all keys in the current database
    pub async fn keys(&mut self) -> RedisResult<Vec<String>> {
        self.connection.keys("*").await
    }

    /// Returns the [`SeedDict`] entry for the given ['SumParticipantPublicKey'] or an empty map
    /// when a [`SeedDict`] entry does not exist.
    pub async fn seed_dict_for_sum_pk(
        &mut self,
        sum_pk: &SumParticipantPublicKey,
    ) -> RedisResult<HashMap<UpdateParticipantPublicKey, xaynet_core::mask::EncryptedMaskSeed>>
    {
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

    /// Deletes all data in the current database.
    pub async fn flush_db(&mut self) -> RedisResult<()> {
        debug!("flush current database");
        // https://redis.io/commands/flushdb
        // > This command never fails.
        redis::cmd("FLUSHDB")
            .arg("ASYNC")
            .query_async(&mut self.connection)
            .await
    }
}

#[cfg(test)]
pub(in crate) mod tests {
    use self::impls::SumDictDeleteError;
    use super::*;
    use crate::{
        state_machine::tests::utils::{mask_settings, model_settings, pet_settings},
        storage::{tests::*, LocalSeedDictAddError, MaskScoreIncrError, SumPartAddError},
    };
    use serial_test::serial;

    async fn create_redis_client() -> Client {
        Client::new("redis://127.0.0.1/").await.unwrap()
    }

    pub async fn init_client() -> Client {
        let mut client = create_redis_client().await;
        client.flush_db().await.unwrap();
        client
    }

    #[tokio::test]
    #[serial]
    async fn integration_set_and_get_coordinator_state() {
        // test the writing and reading of the coordinator state
        let mut client = init_client().await;

        let set_state = CoordinatorState::new(pet_settings(), mask_settings(), model_settings());
        client.set_coordinator_state(&set_state).await.unwrap();

        let get_state = client.coordinator_state().await.unwrap().unwrap();

        assert_eq!(set_state, get_state)
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_coordinator_empty() {
        // test the reading of a non existing coordinator state
        let mut client = init_client().await;

        let get_state = client.coordinator_state().await.unwrap();

        assert_eq!(None, get_state)
    }

    #[tokio::test]
    #[serial]
    async fn integration_incr_mask_score() {
        // test the increment of the mask counter
        let mut client = init_client().await;

        let should_be_none = client.best_masks().await.unwrap();
        assert!(should_be_none.is_none());

        let sum_pks = create_and_add_sum_participant_entries(&mut client, 3).await;
        let mask = create_mask_zeroed(10);
        for sum_pk in sum_pks {
            let res = client.incr_mask_score(&sum_pk, &mask).await;
            assert!(res.is_ok())
        }

        let best_masks = client.best_masks().await.unwrap().unwrap();
        assert!(best_masks.len() == 1);

        let (best_mask, count) = best_masks.into_iter().next().unwrap();
        assert_eq!(best_mask, mask);
        assert_eq!(count, 3);
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_incr_mask_count_unknown_sum_pk() {
        // test the writing and reading of one mask
        let mut client = init_client().await;

        let should_be_none = client.best_masks().await.unwrap();
        assert!(should_be_none.is_none());

        let (sum_pk, _) = create_sum_participant_entry();
        let mask = create_mask_zeroed(10);
        let unknown_sum_pk = client.incr_mask_score(&sum_pk, &mask).await.unwrap();

        assert!(matches!(
            unknown_sum_pk.into_inner().unwrap_err(),
            MaskScoreIncrError::UnknownSumPk
        ));
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_incr_mask_score_sum_pk_already_submitted() {
        // test the writing and reading of one mask
        let mut client = init_client().await;

        let should_be_none = client.best_masks().await.unwrap();
        assert!(should_be_none.is_none());

        let mut sum_pks = create_and_add_sum_participant_entries(&mut client, 1).await;
        let sum_pk = sum_pks.pop().unwrap();
        let mask = create_mask_zeroed(10);
        let result = client.incr_mask_score(&sum_pk, &mask).await.unwrap();
        assert!(result.is_ok());

        let already_submitted = client.incr_mask_score(&sum_pk, &mask).await.unwrap();

        assert!(matches!(
            already_submitted.into_inner().unwrap_err(),
            MaskScoreIncrError::MaskAlreadySubmitted
        ));
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_best_masks_only_one_mask() {
        // test the writing and reading of one mask
        let mut client = init_client().await;

        let should_be_none = client.best_masks().await.unwrap();
        assert!(should_be_none.is_none());

        let sum_pks = create_and_add_sum_participant_entries(&mut client, 1).await;
        let mask = create_mask_zeroed(10);
        let res = client.incr_mask_score(sum_pks.get(0).unwrap(), &mask).await;
        assert!(res.is_ok());

        let best_masks = client.best_masks().await.unwrap().unwrap();
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
        let mut client = init_client().await;

        let should_be_none = client.best_masks().await.unwrap();
        assert!(should_be_none.is_none());

        let sum_pks = create_and_add_sum_participant_entries(&mut client, 2).await;
        let mask_1 = create_mask_zeroed(10);
        for sum_pk in sum_pks {
            let res = client.incr_mask_score(&sum_pk, &mask_1).await;
            assert!(res.is_ok())
        }

        let sum_pks = create_and_add_sum_participant_entries(&mut client, 1).await;
        let mask_2 = create_mask_zeroed(100);
        for sum_pk in sum_pks {
            let res = client.incr_mask_score(&sum_pk, &mask_2).await;
            assert!(res.is_ok())
        }

        let best_masks = client.best_masks().await.unwrap().unwrap();
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
        let mut client = init_client().await;

        let best_masks = client.best_masks().await.unwrap();
        assert!(best_masks.is_none())
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_number_of_unique_masks_empty() {
        // ensure that get_best_masks returns an empty vec if no mask exist
        let mut client = init_client().await;

        let number_of_unique_masks = client.number_of_unique_masks().await.unwrap();
        assert_eq!(number_of_unique_masks, 0)
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_number_of_unique_masks() {
        // ensure that get_best_masks returns an empty vec if no mask exist
        let mut client = init_client().await;

        let should_be_none = client.best_masks().await.unwrap();
        assert!(should_be_none.is_none());

        let sum_pks = create_and_add_sum_participant_entries(&mut client, 4).await;
        for (number, sum_pk) in sum_pks.iter().enumerate() {
            let mask_1 = create_mask(10, number as u32);
            let res = client.incr_mask_score(&sum_pk, &mask_1).await;
            assert!(res.is_ok())
        }

        let number_of_unique_masks = client.number_of_unique_masks().await.unwrap();
        assert_eq!(number_of_unique_masks, 4)
    }

    #[tokio::test]
    #[serial]
    async fn integration_sum_dict() {
        // test multiple sum dict related methods
        let mut client = init_client().await;

        // create two entries and write them into redis
        let mut entries = vec![];
        for _ in 0..2 {
            let (pk, epk) = create_sum_participant_entry();
            let add_new_key = client.add_sum_participant(&pk, &epk).await.unwrap();
            assert!(add_new_key.is_ok());

            entries.push((pk, epk));
        }

        // ensure that add_sum_participant returns SumPartAddError::AlreadyExists if the key already exist
        let (pk, epk) = entries.get(0).unwrap();
        let key_already_exist = client.add_sum_participant(pk, epk).await.unwrap();
        assert!(matches!(
            key_already_exist.into_inner().unwrap_err(),
            SumPartAddError::AlreadyExists
        ));

        // ensure that get_sum_dict_len returns 2
        let len_of_sum_dict = client.sum_dict_len().await.unwrap();
        assert_eq!(len_of_sum_dict, 2);

        // read the written sum keys
        // ensure they are equal
        let sum_pks = client.sum_pks().await.unwrap();
        for (sum_pk, _) in entries.iter() {
            assert!(sum_pks.contains(sum_pk));
        }

        // remove both sum entries
        for (sum_pk, _) in entries.iter() {
            let remove_sum_pk = client.remove_sum_dict_entry(sum_pk).await.unwrap();

            assert!(remove_sum_pk.is_ok());
        }

        // ensure that add_sum_participant returns SumDictDeleteError::DoesNotExist if the key does not exist
        let (sum_pk, _) = entries.get(0).unwrap();
        let key_does_not_exist = client.remove_sum_dict_entry(sum_pk).await.unwrap();
        assert!(matches!(
            key_does_not_exist.into_inner().unwrap_err(),
            SumDictDeleteError::DoesNotExist
        ));

        // ensure that get_sum_dict an empty sum dict
        let sum_dict = client.sum_dict().await.unwrap();
        assert!(sum_dict.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict() {
        let mut client = init_client().await;

        let sum_pks = create_and_add_sum_participant_entries(&mut client, 2).await;
        let local_seed_dicts = create_local_seed_entries(&sum_pks);

        let update_result = add_local_seed_entries(&mut client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let redis_sum_dict = client.sum_dict().await.unwrap().unwrap();
        let seed_dict = create_seed_dict(redis_sum_dict, &local_seed_dicts);

        let redis_seed_dict = client.seed_dict().await.unwrap().unwrap();
        assert_eq!(seed_dict, redis_seed_dict)
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_len_mis_match() {
        let mut client = init_client().await;

        let mut sum_pks = create_and_add_sum_participant_entries(&mut client, 2).await;

        // remove one sum pk to create invalid local seed dicts
        sum_pks.pop();

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = add_local_seed_entries(&mut client, &local_seed_dicts).await;
        update_result.into_iter().for_each(|res| {
            assert!(matches!(
                res.into_inner().unwrap_err(),
                LocalSeedDictAddError::LengthMisMatch
            ))
        });
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_unknown_sum_participant() {
        let mut client = init_client().await;

        let mut sum_pks = create_and_add_sum_participant_entries(&mut client, 2).await;

        // replace a known sum_pk with an unknown one
        sum_pks.pop();
        let (pk, _) = create_sum_participant_entry();
        sum_pks.push(pk);

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = add_local_seed_entries(&mut client, &local_seed_dicts).await;
        update_result.into_iter().for_each(|res| {
            assert!(matches!(
                res.into_inner().unwrap_err(),
                LocalSeedDictAddError::UnknownSumParticipant
            ))
        });
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_update_pk_already_submitted() {
        let mut client = init_client().await;
        let sum_pks = create_and_add_sum_participant_entries(&mut client, 2).await;

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = add_local_seed_entries(&mut client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let update_result = add_local_seed_entries(&mut client, &local_seed_dicts).await;
        update_result.into_iter().for_each(|res| {
            assert!(matches!(
                res.into_inner().unwrap_err(),
                LocalSeedDictAddError::UpdatePkAlreadySubmitted
            ))
        });
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_update_pk_already_exists_in_update_seed_dict() {
        let mut client = init_client().await;
        let sum_pks = create_and_add_sum_participant_entries(&mut client, 2).await;

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = add_local_seed_entries(&mut client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let (update_participant, local_seed_dict) = local_seed_dicts.get(0).unwrap().clone();
        let remove_result = client
            .remove_update_participant(&update_participant)
            .await
            .unwrap();
        assert_eq!(remove_result, 1);

        let update_result =
            add_local_seed_entries(&mut client, &[(update_participant, local_seed_dict)]).await;
        update_result.into_iter().for_each(|res| {
            assert!(matches!(
                res.into_inner().unwrap_err(),
                LocalSeedDictAddError::UpdatePkAlreadyExistsInUpdateSeedDict
            ))
        });
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_get_seed_dict_for_sum_pk() {
        let mut client = init_client().await;
        let mut sum_pks = create_and_add_sum_participant_entries(&mut client, 2).await;

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = add_local_seed_entries(&mut client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let redis_sum_dict = client.sum_dict().await.unwrap().unwrap();
        let seed_dict = create_seed_dict(redis_sum_dict, &local_seed_dicts);

        let sum_pk = sum_pks.pop().unwrap();

        let redis_sum_seed_dict = client.seed_dict_for_sum_pk(&sum_pk).await.unwrap();

        assert_eq!(&redis_sum_seed_dict, seed_dict.get(&sum_pk).unwrap())
    }

    #[tokio::test]
    #[serial]
    async fn integration_seed_dict_get_seed_dict_for_sum_pk_empty() {
        let mut client = init_client().await;
        let (sum_pk, _) = create_sum_participant_entry();

        let result = client.seed_dict_for_sum_pk(&sum_pk).await.unwrap();
        assert!(result.is_empty())
    }

    #[tokio::test]
    #[serial]
    async fn integration_flush_dicts() {
        let mut client = init_client().await;

        // write some data into redis
        let set_state = CoordinatorState::new(pet_settings(), mask_settings(), model_settings());
        let res = client.set_coordinator_state(&set_state).await;
        assert!(res.is_ok());

        let res = client.set_latest_global_model_id("global_model_id").await;
        assert!(res.is_ok());

        let sum_pks = create_and_add_sum_participant_entries(&mut client, 2).await;

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = add_local_seed_entries(&mut client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let mask = create_mask_zeroed(10);
        client
            .incr_mask_score(sum_pks.get(0).unwrap(), &mask)
            .await
            .unwrap();

        // remove dicts
        let res = client.delete_dicts().await;
        assert!(res.is_ok());

        // ensure that only the coordinator state and latest global model id exists
        let res = client.coordinator_state().await;
        assert!(res.unwrap().is_some());

        let res = client.latest_global_model_id().await;
        assert!(res.unwrap().is_some());

        let res = client.sum_dict().await;
        assert!(res.unwrap().is_none());

        let res = client.seed_dict().await;
        assert!(res.unwrap().is_none());

        let res = client.mask_submitted_set().await;
        assert!(res.unwrap().is_empty());

        let res = client.best_masks().await;
        assert!(res.unwrap().is_none());
    }

    #[tokio::test]
    #[serial]
    async fn integration_flush_coordinator_data() {
        let mut client = init_client().await;

        // write some data into redis
        let set_state = CoordinatorState::new(pet_settings(), mask_settings(), model_settings());
        let res = client.set_coordinator_state(&set_state).await;
        assert!(res.is_ok());

        let res = client.set_latest_global_model_id("global_model_id").await;
        assert!(res.is_ok());

        let sum_pks = create_and_add_sum_participant_entries(&mut client, 2).await;

        let local_seed_dicts = create_local_seed_entries(&sum_pks);
        let update_result = add_local_seed_entries(&mut client, &local_seed_dicts).await;
        update_result.iter().for_each(|res| assert!(res.is_ok()));

        let mask = create_mask_zeroed(10);
        client
            .incr_mask_score(sum_pks.get(0).unwrap(), &mask)
            .await
            .unwrap();

        // remove all coordinator data
        let res = client.delete_coordinator_data().await;
        assert!(res.is_ok());

        let keys = client.keys().await.unwrap();
        assert!(keys.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn integration_set_and_get_latest_global_model_id() {
        // test the writing and reading of the global model id
        let mut client = init_client().await;

        let set_id = "global_model_id";
        client.set_latest_global_model_id(set_id).await.unwrap();

        let get_id = client.latest_global_model_id().await.unwrap().unwrap();

        assert_eq!(set_id, get_id)
    }

    #[tokio::test]
    #[serial]
    async fn integration_get_latest_global_model_id_empty() {
        // test the reading of a non existing global model id
        let mut client = init_client().await;

        let get_id = client.latest_global_model_id().await.unwrap();

        assert_eq!(None, get_id)
    }
}
