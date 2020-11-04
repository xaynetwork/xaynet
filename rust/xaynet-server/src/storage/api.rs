use crate::{
    settings::{RedisSettings, S3Settings},
    state_machine::coordinator::CoordinatorState,
    storage::{
        impls::{MaskDictIncr, SeedDictUpdate, SumDictAdd},
        redis, s3,
    },
};

use super::{s3::S3Error, RedisError};
use thiserror::Error;
use xaynet_core::{
    common::RoundSeed,
    crypto::ByteObject,
    mask::{MaskObject, Model},
    LocalSeedDict, SeedDict, SumDict, SumParticipantEphemeralPublicKey, SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

// Not so sure about the error type
// On the one hand, it would be cool to have the specific error inside of StorageError like
// StorageError(RedisError)
// On the other hand, it would lead to compiler errors as soon as we swap the storage backend with
// something that produces other error types e.g. `std::io::Error`
// So I decided to only store the error message instead of the error type
// An alternative is to use "dyn std :: error :: Error", but I'm not sure if there is any benefit to it
#[derive(Debug, Error)]
#[error("storage error: {0}")]
// Wrapper that surrounds all types of error types that are not directly related to the actual application logic
// These include, for example: IOErrors (RedisError, FileNotFoundError, Timeouts), ProtocolErrors (RedisError(wrong type)), etc
pub struct StorageError(String);

type StorageResult<T> = Result<T, StorageError>;

#[async_trait]
pub trait VolatileStorage
where
    Self: Clone + Send + Sync + 'static,
{
    async fn set_coordinator_state(&self, state: &CoordinatorState) -> StorageResult<()>;

    async fn get_coordinator_state(&self) -> StorageResult<Option<CoordinatorState>>;

    async fn add_sum_participant(
        &self,
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
    ) -> StorageResult<SumDictAdd>;

    async fn get_sum_dict(&self) -> StorageResult<SumDict>;

    async fn add_local_seed_dict(
        &self,
        update_pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> StorageResult<SeedDictUpdate>;

    async fn get_seed_dict(&self) -> StorageResult<SeedDict>;

    async fn incr_mask_score(
        &self,
        pk: &SumParticipantPublicKey,
        mask: &MaskObject,
    ) -> StorageResult<MaskDictIncr>;

    async fn get_best_masks(&self) -> StorageResult<Vec<(MaskObject, u64)>>;

    async fn get_number_of_unique_masks(&self) -> StorageResult<u64>;

    async fn delete_coordinator_data(&self) -> StorageResult<()>;

    async fn delete_dicts(&self) -> StorageResult<()>;

    async fn set_latest_global_model_id(&self, id: &str) -> StorageResult<()>;

    async fn get_latest_global_model_id(&self) -> StorageResult<Option<String>>;
}

#[async_trait]
pub trait PersistentStorage
where
    Self: Clone + Send + Sync + 'static,
{
    async fn set_global_model(
        &self,
        round_id: u64,
        round_seed: &RoundSeed,
        global_model: &Model,
    ) -> Result<String, StorageError>;

    async fn get_global_model(&self, id: &str) -> StorageResult<Option<Model>>;

    fn format_global_model_id(round_id: u64, round_seed: &RoundSeed) -> String {
        let round_seed = hex::encode(round_seed.as_slice());
        format!("{}_{}", round_id, round_seed)
    }
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct Storage<V, P>
where
    V: VolatileStorage,
    P: PersistentStorage,
{
    volatile: V,
    persistent: P,
}

impl<V, P> Storage<V, P>
where
    V: VolatileStorage,
    P: PersistentStorage,
{
    pub fn new(volatile: V, persistent: P) -> Self {
        Self {
            volatile,
            persistent,
        }
    }
}

#[async_trait]
impl<V: VolatileStorage, P: PersistentStorage> VolatileStorage for Storage<V, P> {
    /// Sets a [`CoordinatorState`].
    ///
    /// Behavior
    /// - if no state has been set yet, set state and return Ok
    /// - if a state already exists, override the state and return OK
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// None
    async fn set_coordinator_state(&self, state: &CoordinatorState) -> StorageResult<()> {
        self.volatile.set_coordinator_state(state).await
    }

    /// Get a [`CoordinatorState`].
    ///
    /// Behavior
    /// - if no state has been set yet, return Ok(None)
    /// - if a state exists, return Ok(state)
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// None
    async fn get_coordinator_state(&self) -> StorageResult<Option<CoordinatorState>> {
        self.volatile.get_coordinator_state().await
    }

    /////////// Sum dict

    /// Adds a sum participant entry to the `SumDict`.
    ///
    /// Behavior
    /// - if sum participant has been added, return Ok
    /// - if pet protocol error, return [`SumDictAddError::`]
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// - [`SumDictAddError::AlreadyExists`]
    async fn add_sum_participant(
        &self,
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
    ) -> StorageResult<SumDictAdd> {
        self.volatile.add_sum_participant(pk, ephm_pk).await
    }

    /// Gets the `SumDict`.
    ///
    /// Behavior
    /// - if sum dict does not exist, return Ok(empty sum dict) or Ok(None)?
    /// - if sum dict exists, return Ok(SumDict)
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// None
    async fn get_sum_dict(&self) -> StorageResult<SumDict> {
        self.volatile.get_sum_dict().await
    }

    /////////// Seed dict

    /// Adds a local [`LocalSeedDict`] of the given ['UpdateParticipantPublicKey'] to the `SeedDict`.
    ///
    /// Behavior
    /// - if local seed dict has been added, return Ok
    /// - if pet protocol error, return [`SeedDictAddError::`]
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// - [`SeedDictAddError::LengthMisMatch`]
    /// - [`SeedDictAddError::UnknownSumParticipant`]
    /// - [`SeedDictAddError::UpdatePkAlreadySubmitted`]
    /// - [`SeedDictAddError::UpdatePkAlreadyExistsInUpdateSeedDict`]
    async fn add_local_seed_dict(
        &self,
        update_pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> StorageResult<SeedDictUpdate> {
        self.volatile
            .add_local_seed_dict(update_pk, local_seed_dict)
            .await
    }

    /// Gets the [`SeedDict`].
    ///
    /// Behavior
    /// - if seed dict does not exist, return Ok(empty seed dict) or Ok(None)?
    /// - if seed dict exists, return Ok(SeedDict)
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// None
    async fn get_seed_dict(&self) -> StorageResult<SeedDict> {
        self.volatile.get_seed_dict().await
    }

    /////////// Mask dict

    /// Increments the mask score with the given [`MaskObject`].
    /// The score of the given mask is incremented by `1`.
    ///
    /// Behavior
    /// - if sum participant has not submitted a mask yet, return Ok
    /// - if pet protocol error, return [`MaskDictIncrError::`]
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// - [`MaskDictIncrError::UnknownSumPk`]
    /// - [`MaskDictIncrError::MaskAlreadySubmitted`]
    async fn incr_mask_score(
        &self,
        pk: &SumParticipantPublicKey,
        mask: &MaskObject,
    ) -> StorageResult<MaskDictIncr> {
        self.volatile.incr_mask_score(pk, mask).await
    }

    /// Gets the two masks with the highest score.
    ///
    /// Behavior
    /// - if no masks exist, return Ok(empty vec) or None?
    /// - if only one mask exists, return the mask
    /// - if two masks exist with the same score, return both
    /// - if two masks exist with the different score, return both (in descending order)
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// None
    async fn get_best_masks(&self) -> StorageResult<Vec<(MaskObject, u64)>> {
        self.volatile.get_best_masks().await
    }

    /// Gets the number of unique masks.
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// None
    async fn get_number_of_unique_masks(&self) -> StorageResult<u64> {
        self.volatile.get_number_of_unique_masks().await
    }

    /////////// Data

    /// Deletes all coordinator data in the current database.
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// None
    async fn delete_coordinator_data(&self) -> StorageResult<()> {
        self.volatile.delete_coordinator_data().await
    }

    /// Deletes the [`SumDict`], [`SeedDict`] and mask dictionary.
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// None
    async fn delete_dicts(&self) -> StorageResult<()> {
        self.volatile.delete_dicts().await
    }

    /// Sets the latest global model id.
    ///
    /// Behavior
    /// - if no global model id has been set yet, set id and return Ok
    /// - if the global model id already exists, override the id and return OK
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// None
    async fn set_latest_global_model_id(&self, id: &str) -> StorageResult<()> {
        self.volatile.set_latest_global_model_id(id).await
    }

    /// Gets the latest global model.
    ///
    /// Behavior
    /// - if the global model id does not exist, return Ok(None)
    /// - if the global model id exists, return Ok(Some(id))
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - RedisError
    ///
    /// ## Protocol
    /// None
    async fn get_latest_global_model_id(&self) -> StorageResult<Option<String>> {
        self.volatile.get_latest_global_model_id().await
    }
}

#[async_trait]
impl<V: VolatileStorage, P: PersistentStorage> PersistentStorage for Storage<V, P> {
    /////////// Global model

    /// Set a global model.
    ///
    /// Behavior
    /// - if the global model already exists (same id),  ?
    /// - if the global model does not exist,  write model and return id
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - S3Error
    ///
    /// ## Protocol
    /// None
    async fn set_global_model(
        &self,
        round_id: u64,
        round_seed: &RoundSeed,
        global_model: &Model,
    ) -> Result<String, StorageError> {
        self.persistent
            .set_global_model(round_id, round_seed, global_model)
            .await
    }

    /// Gets a global model.
    ///
    /// Behavior
    /// - if the global model does not exist, return Ok(None)
    /// - if the global model exists, return Ok(Some(Model))
    ///
    /// # Errors:
    ///
    /// ## IO
    /// - S3Error
    ///
    /// ## Protocol
    /// None
    async fn get_global_model(&self, id: &str) -> StorageResult<Option<Model>> {
        self.persistent.get_global_model(id).await
    }
}

// #[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct RedisStorage {
    redis: redis::Client,
}

impl RedisStorage {
    pub async fn new(redis_settings: RedisSettings) -> Result<Self, ()> {
        Ok(Self {
            redis: redis::Client::new(redis_settings.url, 10).await.unwrap(),
        })
    }
}

impl From<RedisError> for StorageError {
    fn from(e: RedisError) -> Self {
        Self(format!("{}", e))
    }
}

#[async_trait]
impl VolatileStorage for RedisStorage {
    async fn set_coordinator_state(&self, state: &CoordinatorState) -> StorageResult<()> {
        self.redis
            .connection()
            .await
            .set_coordinator_state(state)
            .await
            .map_err(StorageError::from)
    }

    async fn get_coordinator_state(&self) -> StorageResult<Option<CoordinatorState>> {
        self.redis
            .connection()
            .await
            .get_coordinator_state()
            .await
            .map_err(StorageError::from)
    }

    async fn add_sum_participant(
        &self,
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
    ) -> StorageResult<SumDictAdd> {
        self.redis
            .connection()
            .await
            .add_sum_participant(pk, ephm_pk)
            .await
            .map_err(StorageError::from)
    }

    async fn get_sum_dict(&self) -> StorageResult<SumDict> {
        self.redis
            .connection()
            .await
            .get_sum_dict()
            .await
            .map_err(StorageError::from)
    }

    async fn add_local_seed_dict(
        &self,
        update_pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> StorageResult<SeedDictUpdate> {
        self.redis
            .connection()
            .await
            .update_seed_dict(update_pk, local_seed_dict)
            .await
            .map_err(StorageError::from)
    }

    async fn get_seed_dict(&self) -> StorageResult<SeedDict> {
        self.redis
            .connection()
            .await
            .get_seed_dict()
            .await
            .map_err(StorageError::from)
    }

    async fn incr_mask_score(
        &self,
        pk: &SumParticipantPublicKey,
        mask: &MaskObject,
    ) -> StorageResult<MaskDictIncr> {
        self.redis
            .connection()
            .await
            .incr_mask_count(pk, mask)
            .await
            .map_err(StorageError::from)
    }

    async fn get_best_masks(&self) -> StorageResult<Vec<(MaskObject, u64)>> {
        self.redis
            .connection()
            .await
            .get_best_masks()
            .await
            .map_err(StorageError::from)
    }

    async fn get_number_of_unique_masks(&self) -> StorageResult<u64> {
        self.redis
            .connection()
            .await
            .get_number_of_unique_masks()
            .await
            .map_err(StorageError::from)
    }

    async fn delete_coordinator_data(&self) -> StorageResult<()> {
        self.redis
            .connection()
            .await
            .flush_coordinator_data()
            .await
            .map_err(StorageError::from)
    }

    async fn delete_dicts(&self) -> StorageResult<()> {
        self.redis
            .connection()
            .await
            .flush_dicts()
            .await
            .map_err(StorageError::from)
    }

    async fn set_latest_global_model_id(&self, id: &str) -> StorageResult<()> {
        self.redis
            .connection()
            .await
            .set_latest_global_model_id(id)
            .await
            .map_err(StorageError::from)
    }

    async fn get_latest_global_model_id(&self) -> StorageResult<Option<String>> {
        let id = self
            .redis
            .connection()
            .await
            .get_latest_global_model_id()
            .await
            .map_err(StorageError::from)?;
        Ok(id)
    }
}

#[derive(Clone)]
pub struct S3Storage {
    s3: s3::Client,
}

impl From<S3Error> for StorageError {
    fn from(e: S3Error) -> Self {
        Self(format!("{}", e))
    }
}

impl S3Storage {
    pub async fn new(s3_settings: S3Settings) -> Result<Self, ()> {
        Ok(Self {
            s3: s3::Client::new(s3_settings).unwrap(),
        })
    }
}

#[async_trait]
impl PersistentStorage for S3Storage {
    async fn set_global_model(
        &self,
        round_id: u64,
        round_seed: &RoundSeed,
        global_model: &Model,
    ) -> StorageResult<String> {
        let id = Self::format_global_model_id(round_id, round_seed);
        self.s3
            .upload_global_model(&id, global_model)
            .await
            .map_err(StorageError::from)?;
        Ok(id)
    }

    async fn get_global_model(&self, id: &str) -> StorageResult<Option<Model>> {
        let model = self
            .s3
            .download_global_model(&id)
            .await
            .map_err(StorageError::from)?;
        Ok(Some(model))
    }
}

#[async_trait]
pub trait Store
where
    Self: Clone + Send + Sync + 'static + VolatileStorage + PersistentStorage,
{
}

#[async_trait]
impl<V, P> Store for Storage<V, P>
where
    V: VolatileStorage,
    P: PersistentStorage,
{
}
