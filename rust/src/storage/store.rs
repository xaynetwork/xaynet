use crate::{
    coordinator::CoordinatorState,
    EncrMaskSeed,
    LocalSeedDict,
    MaskHash,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

use redis::{aio::MultiplexedConnection, AsyncCommands, Client, RedisError, RedisResult};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub type MaskDict = HashMap<MaskHash, usize>;

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
        self.connection.get("coordinator_state").await
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
        redis::pipe()
            .hset("sum_dict", pk, ephm_pk)
            .hlen("sum_dict")
            .query_async(&mut self.connection)
            .await
    }

    /// Retrieve [`SeedDict`] entry for the given sum participant
    pub async fn get_seed_dict(
        mut self,
        sum_pk: SumParticipantPublicKey,
    ) -> Result<HashMap<UpdateParticipantPublicKey, EncrMaskSeed>, RedisError> {
        let result: Vec<(UpdateParticipantPublicKey, EncrMaskSeed)> =
            self.connection.hgetall(sum_pk).await?;
        Ok(result.into_iter().collect())
    }

    /// Update the [`SeedDict`] with the seeds from the given update
    /// participant, and return the number of participants that
    /// already submitted an update.
    pub async fn update_seed_dict(
        mut self,
        update_pk: UpdateParticipantPublicKey,
        update: LocalSeedDict,
    ) -> Result<usize, RedisError> {
        let mut pipe = redis::pipe();
        pipe.sadd("update_participants", update_pk);
        for (sum_pk, encr_seed) in update {
            pipe.hset(sum_pk, update_pk, encr_seed);
        }
        pipe.scard("update_participants");
        pipe.atomic().query_async(&mut self.connection).await
    }

    /// Update the [`MaskDict`] with the given mask hash and return
    /// the updated mask dictionary.
    pub async fn incr_mask_count(mut self, mask: MaskHash) -> Result<MaskDict, RedisError> {
        let result: Vec<(MaskHash, usize)> = redis::pipe()
            .atomic()
            .zadd("mask_dict", mask, 1_usize)
            .zrange_withscores("mask_dict", 0, isize::MAX)
            .query_async(&mut self.connection)
            .await?;
        Ok(result.into_iter().collect())
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
