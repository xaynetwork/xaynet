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
    pub async fn get_coordinator_state(mut self) -> Result<CoordinatorState, RedisError> {
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
        let result = self.connection.hset_nx("sum_dict", pk, ephm_pk).await;
        result
    }

    /// Remove a entry in the [`SumDict`].
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
        update: LocalSeedDict,
    ) -> Result<(), RedisError> {
        let mut pipe = redis::pipe();
        pipe.sadd("update_participants", update_pk);
        for (sum_pk, encr_seed) in update {
            pipe.hset_nx(sum_pk, update_pk, encr_seed);
        }
        pipe.atomic().query_async(&mut self.connection).await
    }

    /// Update the [`MaskDict`] with the given mask hash and return
    /// the updated mask dictionary.
    pub async fn incr_mask_count(mut self, mask: Mask) -> Result<(), RedisError> {
        redis::pipe()
            .zincr("mask_dict", mask.serialize(), 1_usize)
            .query_async(&mut self.connection)
            .await?;
        Ok(())
    }

    /// Update the [`MaskDict`] with the given mask hash and return
    /// the updated mask dictionary.
    // return the two masks with the highest score
    pub async fn get_best_masks(mut self) -> Result<Vec<(Mask, usize)>, RedisError> {
        let result: Vec<(Vec<u8>, usize)> = self
            .connection
            .zrevrange_withscores("mask_dict", 0, 1)
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
        pipe.del("sum_dict");

        //delete seed_dict
        pipe.del("update_participants");
        for sum_pk in sum_pks {
            pipe.del(sum_pk);
        }

        //delete mask_dict
        pipe.del("mask_dict");
        pipe.atomic().query_async(&mut self.connection).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        coordinator::{CoordinatorState, RoundSeed},
        crypto::{
            generate_encrypt_key_pair,
            generate_integer,
            generate_signing_key_pair,
            ByteObject,
        },
        mask::{
            config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
            Mask,
            MaskedModel,
        },
        model::Model,
    };
    use futures::stream::{FuturesUnordered, StreamExt};
    use num::{bigint::BigUint, traits::identities::Zero};
    use sodiumoxide::randombytes::randombytes;
    use std::{convert::TryFrom, iter, time::Instant};
    use tokio::task::JoinHandle;

    fn create_mask(byte_size: usize) -> Mask {
        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();

        Mask::from_parts(vec![BigUint::zero(); byte_size], config.clone()).unwrap()
    }

    async fn create_store() -> RedisStore {
        RedisStore::new("redis://127.0.0.1/", 10).await.unwrap()
    }

    #[tokio::test]
    #[ignore]
    async fn test_increment_mask() {
        let store = create_store().await;
        store.clone().connection().await.flushdb().await.unwrap();

        let mask = create_mask(10);
        let res = store.connection().await.incr_mask_count(mask).await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    #[ignore]
    async fn test_set_and_get_coordinator_state() {
        let store = create_store().await;
        store.clone().connection().await.flushdb().await.unwrap();

        let cs = CoordinatorState::default();

        store
            .clone()
            .connection()
            .await
            .set_coordinator_state(cs.clone())
            .await
            .unwrap();

        let res = store
            .connection()
            .await
            .get_coordinator_state()
            .await
            .unwrap();
        assert_eq!(cs, res)
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_best_masks_one_mask() {
        let store = create_store().await;
        store.clone().connection().await.flushdb().await.unwrap();

        let mask = create_mask(10);
        store
            .clone()
            .connection()
            .await
            .incr_mask_count(mask.clone())
            .await
            .unwrap();

        let res = store.connection().await.get_best_masks().await.unwrap();
        assert!(res.len() == 1)
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_best_masks_two_masks() {
        let store = create_store().await;
        store.clone().connection().await.flushdb().await.unwrap();

        let mask = create_mask(10);
        store
            .clone()
            .connection()
            .await
            .incr_mask_count(mask.clone())
            .await
            .unwrap();
        store
            .clone()
            .connection()
            .await
            .incr_mask_count(mask)
            .await
            .unwrap();

        let mask = create_mask(100);
        store
            .clone()
            .connection()
            .await
            .incr_mask_count(mask)
            .await
            .unwrap();

        let res = store.connection().await.get_best_masks().await.unwrap();
        assert!(res.len() == 2)
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_best_masks_none_mask() {
        let store = create_store().await;
        store.clone().connection().await.flushdb().await.unwrap();

        let res = store.connection().await.get_best_masks().await.unwrap();
        assert!(res.is_empty())
    }

    #[tokio::test]
    #[ignore]
    async fn test_sum_dict() {
        let store = create_store().await;
        store.clone().connection().await.flushdb().await.unwrap();

        let mut entries = vec![];
        for _ in 0..2 {
            let (pk, _) = generate_signing_key_pair();
            let (epk, _) = generate_encrypt_key_pair();
            entries.push((pk.clone(), epk.clone()));

            store
                .clone()
                .connection()
                .await
                .add_sum_participant(pk, epk)
                .await
                .unwrap();
        }

        let len = store
            .clone()
            .connection()
            .await
            .get_sum_dict_len()
            .await
            .unwrap();
        assert_eq!(len, 2);

        let sum_pks = store
            .clone()
            .connection()
            .await
            .get_sum_pks()
            .await
            .unwrap();
        assert_eq!(sum_pks.len(), 2);

        store
            .clone()
            .connection()
            .await
            .remove_sum_dict_entry(sum_pks.into_iter().next().unwrap())
            .await
            .unwrap();

        let sum_dict = store
            .clone()
            .connection()
            .await
            .get_sum_dict()
            .await
            .unwrap();
        assert_eq!(sum_dict.len(), 1);
    }
}
