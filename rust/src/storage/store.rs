use crate::{
    coordinator::CoordinatorState,
    mask::Mask,
    EncryptedMaskSeed,
    LocalSeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

use crate::mask::Integers;
use redis::{aio::MultiplexedConnection, AsyncCommands, Client, RedisError, RedisResult};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
pub struct RedisStore {
    raw_connection: MultiplexedConnection,
    semaphore: Arc<Semaphore>,
}

pub struct Connection {
    connection: MultiplexedConnection,
    permit: OwnedSemaphorePermit,
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
        let permit = self.semaphore.acquire_owned().await;
        Connection {
            connection: self.raw_connection,
            permit,
        }
    }
}

impl Connection {
    pub async fn get_coordinator_state(mut self) -> Result<Option<CoordinatorState>, RedisError> {
        self.connection.get("coordinator_round_state").await
    }

    pub async fn set_coordinator_state(
        mut self,
        state: CoordinatorState,
    ) -> Result<(), RedisError> {
        self.connection.set("coordinator_state", state).await
    }

    /// Retrieve the enties [`SumDict`]
    pub async fn get_sum_dict(mut self) -> Result<SumDict, RedisError> {
        let result: Vec<(SumParticipantPublicKey, SumParticipantEphemeralPublicKey)> =
            self.connection.hgetall("sum_dict").await?;
        Ok(result.into_iter().collect())
    }

    /// Store a new [`SumDict`] entry and return the updated number of
    /// entries in the sum dictionary.
    pub async fn add_sum_participant(
        mut self,
        pk: SumParticipantPublicKey,
        ephm_pk: SumParticipantEphemeralPublicKey,
    ) -> Result<usize, RedisError> {
        let result = self.connection.hset_nx("sum_dict", pk, ephm_pk).await;
        result
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

    /// Return the length of the [`SumDict`].
    pub async fn remove_sum_dict_entry(
        mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<usize, RedisError> {
        self.connection.hdel("sum_dict", pk).await
    }

    /// Return the length of the [`SumDict`].
    pub async fn get_sum_dict_len(mut self) -> Result<usize, RedisError> {
        self.connection.hlen("sum_dict").await
    }

    /// Check if all given sum_pk exist in the [`SumDict`].
    pub async fn are_sum_pks_in_sum_dict(
        mut self,
        sum_pks: impl IntoIterator<Item = &SumParticipantPublicKey>,
    ) -> Result<bool, RedisError> {
        let mut pipe = redis::pipe();
        for sum_pk in sum_pks {
            pipe.hexists("sum_dict", sum_pk);
        }
        let result: Vec<u8> = pipe.atomic().query_async(&mut self.connection).await?;
        Ok(result.into_iter().all(|contains| contains == 1))
    }

    /// Update the [`SeedDict`] with the seeds from the given update
    /// participant, and return the number of participants that
    /// already submitted an update.
    pub async fn update_seed_dict(
        mut self,
        update_pk: UpdateParticipantPublicKey,
        update: LocalSeedDict,
    ) -> Result<(), RedisError> {
        let mut pipe = redis::pipe();
        pipe.sadd("update_participants", update_pk);
        for (sum_pk, encr_seed) in update {
            pipe.hset_nx(sum_pk, update_pk, encr_seed);
        }
        pipe.atomic().query_async(&mut self.connection).await
    }

    // Update the [`MaskDict`] with the given mask hash and return
    // the updated mask dictionary.
    pub async fn incr_mask_count(mut self, mask: Mask) -> Result<(), RedisError> {
        redis::pipe()
            .zincr("mask_dict", mask.serialize(), 1_usize)
            .query_async(&mut self.connection)
            .await?;
        Ok(())
    }

    // Update the [`MaskDict`] with the given mask hash and return
    // the updated mask dictionary.
    pub async fn get_best_masks(mut self) -> Result<Vec<(Mask, usize)>, RedisError> {
        let result: Vec<(Vec<u8>, usize)> = redis::pipe()
            // return the two masks with the highest score
            .zrevrange_withscores("mask_dict", 0, 1)
            .query_async(&mut self.connection)
            .await?;
        Ok(result
            .into_iter()
            .map(|(mask, count)| (Mask::deserialize(&mask).unwrap(), count))
            .collect())
    }

    pub async fn schedule_snapshot(mut self) -> RedisResult<()> {
        redis::cmd("BGSAVE")
            .arg("SCHEDULE")
            .query_async(&mut self.connection)
            .await?;
        Ok(())
    }

    pub async fn flushdb(mut self) -> RedisResult<()> {
        redis::cmd("FLUSHDB")
            .arg("ASYNC")
            .query_async(&mut self.connection)
            .await
    }
}
