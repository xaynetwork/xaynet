use crate::{
    coordinator_async::CoordinatorState,
    mask::{EncryptedMaskSeed, MaskObject},
    LocalSeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

use redis::{aio::MultiplexedConnection, AsyncCommands, Client, RedisError, RedisResult};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
pub struct RedisStore {
    raw_connection: MultiplexedConnection,
    semaphore: Arc<Semaphore>,
}

pub struct Connection {
    connection: MultiplexedConnection,
    _permit: OwnedSemaphorePermit,
}

impl RedisStore {
    /// Create a new store. `url` is the URL to connect to the redis
    /// instance, and `n` is the maximum number of concurrent
    /// connections to the store.
    pub async fn new<S: Into<String>>(url: S, n: usize) -> Result<Self, RedisError> {
        let client = Client::open(url.into())?;
        let connection = client.get_multiplexed_tokio_connection().await?;
        Ok(Self {
            raw_connection: connection,
            semaphore: Arc::new(Semaphore::new(n)),
        })
    }

    pub async fn connection(self) -> Connection {
        let _permit = self.semaphore.acquire_owned().await;
        Connection {
            connection: self.raw_connection,
            _permit,
        }
    }
}

impl Connection {
    pub async fn get_coordinator_state(mut self) -> Result<CoordinatorState, RedisError> {
        self.connection.get("coordinator_state").await
    }

    pub async fn set_coordinator_state(
        mut self,
        state: &CoordinatorState,
    ) -> Result<(), RedisError> {
        self.connection.set("coordinator_state", state).await
    }

    /// Retrieve the entries [`SumDict`]
    pub async fn get_sum_dict(mut self) -> Result<SumDict, RedisError> {
        let result: Vec<(SumParticipantPublicKey, SumParticipantEphemeralPublicKey)> =
            self.connection.hgetall("sum_dict").await?;
        Ok(result.into_iter().collect())
    }

    /// Store a new [`SumDict`] entry.
    /// Returns `1` if field is a new and `0` if field already exists.
    pub async fn add_sum_participant(
        mut self,
        pk: SumParticipantPublicKey,
        ephm_pk: SumParticipantEphemeralPublicKey,
    ) -> Result<usize, RedisError> {
        let result = self.connection.hset_nx("sum_dict", pk, ephm_pk).await;
        result
    }

    /// Remove an entry in the [`SumDict`].
    /// Returns `1` if field was deleted and `0` if field does not exists.
    pub async fn remove_sum_dict_entry(
        mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<usize, RedisError> {
        self.connection.hdel("sum_dict", pk).await
    }

    /// Retrieve the length of the [`SumDict`].
    pub async fn get_sum_dict_len(mut self) -> Result<usize, RedisError> {
        self.connection.hlen("sum_dict").await
    }

    /// Retrieve the sum_pks of the [`SumDict`].
    pub async fn get_sum_pks(mut self) -> Result<HashSet<SumParticipantPublicKey>, RedisError> {
        self.connection.hkeys("sum_dict").await
    }

    /// Retrieve [`SeedDict`] entry for the given sum participant
    pub async fn get_seed_dict(
        mut self,
        sum_pk: SumParticipantPublicKey,
    ) -> Result<HashMap<UpdateParticipantPublicKey, EncryptedMaskSeed>, RedisError> {
        let result: Vec<(UpdateParticipantPublicKey, EncryptedMaskSeed)> =
            self.connection.hgetall(sum_pk).await?;
        Ok(result.into_iter().collect())
    }

    /// Update the [`SeedDict`] with the seeds from the given update
    /// participant, and return the number of participants that
    /// already submitted an update.
    pub async fn update_seed_dict(
        mut self,
        update_pk: UpdateParticipantPublicKey,
        update: &LocalSeedDict,
    ) -> Result<(), RedisError> {
        let mut pipe = redis::pipe();
        pipe.sadd("update_participants", update_pk).ignore();
        for (sum_pk, encr_seed) in update {
            pipe.hset_nx(sum_pk, update_pk, encr_seed).ignore();
        }
        pipe.atomic().query_async(&mut self.connection).await
    }

    /// Update the [`MaskDict`] with the given mask.
    /// The score/counter of the given mask is incremented by `1`.
    pub async fn incr_mask_count(mut self, mask: &MaskObject) -> Result<(), RedisError> {
        redis::pipe()
            .zincr("mask_dict", bincode::serialize(mask).unwrap(), 1_usize)
            .query_async(&mut self.connection)
            .await?;
        Ok(())
    }

    /// Retrieve the two masks with the highest score.
    pub async fn get_best_masks(mut self) -> Result<Vec<(MaskObject, usize)>, RedisError> {
        let result: Vec<(Vec<u8>, usize)> = self
            .connection
            .zrevrange_withscores("mask_dict", 0, 1)
            .await?;

        Ok(result
            .into_iter()
            .map(|(mask, count)| (bincode::deserialize(&mask).unwrap(), count))
            .collect())
    }

    pub async fn schedule_snapshot(mut self) -> RedisResult<()> {
        redis::cmd("BGSAVE")
            .arg("SCHEDULE")
            .query_async(&mut self.connection)
            .await?;
        Ok(())
    }

    /// Delete all data in the current database.
    pub async fn flushdb(mut self) -> RedisResult<()> {
        redis::cmd("FLUSHDB")
            .arg("ASYNC")
            .query_async(&mut self.connection)
            .await
    }

    /// Delete the dictionaries `sum_dict`, `seed_dict` and `mask_dict`.
    pub async fn flush_dicts(mut self) -> RedisResult<()> {
        let sum_pks: Vec<SumParticipantPublicKey> = self.connection.hkeys("sum_dict").await?;
        let mut pipe = redis::pipe();

        // delete sum_dict
        pipe.del("sum_dict").ignore();

        //delete seed_dict
        pipe.del("update_participants").ignore();
        for sum_pk in sum_pks {
            pipe.del(sum_pk).ignore();
        }

        //delete mask_dict
        pipe.del("mask_dict").ignore();
        pipe.atomic().query_async(&mut self.connection).await
    }
}
