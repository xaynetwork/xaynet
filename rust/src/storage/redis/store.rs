use super::serde::*;
use crate::storage::state::{
    CoordinatorState,
    CoordinatorStateRequest,
    MaskDictEntry,
    MaskDictResult,
    SeedDictEntry,
    SeedDictResult,
    SubSeedDictResult,
    SumDictEntry,
    SumDictResult,
};

use redis::{aio::MultiplexedConnection, AsyncCommands, Client, RedisError, RedisResult};
use std::collections::HashMap;

#[derive(Clone)]
pub struct RedisStore {
    connection: MultiplexedConnection,
}

impl RedisStore {
    pub async fn new<S: Into<String>>(url: S) -> Result<Self, RedisError> {
        let client = Client::open(url.into())?;
        let connection = client.get_multiplexed_tokio_connection().await?;

        Ok(Self { connection })
    }

    pub async fn set_coordinator_state(
        &mut self,
        state: &CoordinatorState,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let (k, v) = serialize_coordinator_state(state)?;
        self.connection.set(k, v).await?;
        Ok(())
    }

    pub async fn set_sum_dict_entry(
        &mut self,
        entry: &SumDictEntry,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let (sum_pk, sum_epk) = serialize_sum_dict_entry(&entry)?;
        self.connection
            .hset(RedisKeys::sum_dict(), sum_pk, sum_epk)
            .await?;
        Ok(())
    }

    pub async fn set_sum_dict_entry_batch(
        &mut self,
        entries: &Vec<SumDictEntry>,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let mut batch: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for entry in entries.iter() {
            batch.push(serialize_sum_dict_entry(entry)?)
        }
        self.connection
            .hset_multiple(RedisKeys::sum_dict(), &batch)
            .await?;
        Ok(())
    }

    pub async fn set_seed_dict_entry(
        &mut self,
        entry: &SeedDictEntry,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let (sum_pk, seeds_map) = serialize_seed_dict_entry(&entry)?;
        // Add sum_pk to a set of sum_pks
        self.connection
            .sadd(RedisKeys::seed_dict(), sum_pk.clone())
            .await?;
        // Add seeds_map to a hashmap with the key seed_dict:<sum_pk>
        self.connection
            .hset_multiple(&RedisKeys::sub_seed_dict_key(&sum_pk), &seeds_map)
            .await?;
        Ok(())
    }

    pub async fn set_mask_dict_entry(
        &mut self,
        entry: &MaskDictEntry,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let mask_hash = serialize_mask_dict_entry(&entry)?;
        self.connection
            .sadd(RedisKeys::mask_dict(), mask_hash)
            .await?;
        Ok(())
    }

    pub async fn get_coordinator_state(
        &mut self,
        request: &CoordinatorStateRequest,
    ) -> Result<CoordinatorState, Box<dyn std::error::Error + 'static>> {
        let key = RedisKeys::from_coordinator_state_request(request);
        let result = self.connection.get(key).await?;
        Ok(deserialize_coordinator_state(&request, &result)?)
    }

    pub async fn get_sum_dict(
        &mut self,
    ) -> Result<SumDictResult, Box<dyn std::error::Error + 'static>> {
        let result = self.connection.hgetall(RedisKeys::sum_dict()).await?;
        Ok(deserialize_sum_dict(&result)?)
    }

    pub async fn get_seed_dict(
        &mut self,
    ) -> Result<SeedDictResult, Box<dyn std::error::Error + 'static>> {
        let seed_dict_keys: Vec<Vec<u8>> = self.connection.smembers(RedisKeys::seed_dict()).await?;

        let mut seed_dict: SeedDictResult = HashMap::new();
        for sum_pk_as_bin in seed_dict_keys.into_iter() {
            let sub_seed_dict = self.get_sub_seed_dict(&sum_pk_as_bin).await?;

            let sum_pk = deserialize_sum_pk(&sum_pk_as_bin)?;
            seed_dict.insert(sum_pk, sub_seed_dict);
        }

        Ok(seed_dict)
    }

    async fn get_sub_seed_dict(
        &mut self,
        sum_pk: &Vec<u8>,
    ) -> Result<SubSeedDictResult, Box<dyn std::error::Error + 'static>> {
        let result = self
            .connection
            .hgetall(&RedisKeys::sub_seed_dict_key(&sum_pk))
            .await?;
        Ok(deserialize_seed_dict_entry(&result)?)
    }

    async fn get_mask_dict(
        &mut self,
    ) -> Result<MaskDictResult, Box<dyn std::error::Error + 'static>> {
        let result = self.connection.smembers(RedisKeys::mask_dict()).await?;
        Ok(deserialize_mask_dict(&result)?)
    }

    async fn schedule_snapshot(&mut self) -> RedisResult<()> {
        redis::cmd("BGSAVE")
            .arg("SCHEDULE")
            .query_async(&mut self.connection)
            .await?;
        Ok(())
    }

    async fn clear_all_async(&mut self) -> RedisResult<()> {
        redis::cmd("FLUSHDB")
            .arg("ASYNC")
            .query_async(&mut self.connection)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::RedisStore;
    use crate::{
        coordinator::Phase,
        storage::state::{
            CoordinatorState,
            CoordinatorStateRequest,
            MaskDictEntry,
            MaskDictResult,
            SeedDictEntry,
            SeedDictResult,
            SubSeedDictResult,
            SumDictEntry,
            SumDictResult,
        },
        EncrMaskSeed,
    };
    use counter::Counter;
    use futures::*;
    use sodiumoxide::{
        crypto::{box_, hash::sha256, sign},
        randombytes::randombytes,
    };
    use std::{collections::HashMap, convert::TryFrom, iter, time::Instant};

    #[tokio::test]
    async fn test_set_get_sum_dict() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();
        //store.clear_all().await.unwrap();

        let sum_participant_pk = sign::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let sum_participant_epk = box_::PublicKey([1_u8; box_::PUBLICKEYBYTES]);
        let mut expect: SumDictResult = HashMap::new();
        expect.insert(sum_participant_pk, sum_participant_epk);

        store
            .set_sum_dict_entry(&SumDictEntry(sum_participant_pk, sum_participant_epk))
            .await
            .unwrap();
        let get = store.get_sum_dict().await.unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_seed_dict() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();
        //store.clear_all().await.unwrap();

        let sum_participant_pk = sign::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let update_participant_pk = sign::PublicKey([1_u8; box_::PUBLICKEYBYTES]);
        let seed = EncrMaskSeed::try_from(randombytes(80)).unwrap();

        let mut seeds_map: SubSeedDictResult = HashMap::new();
        seeds_map.insert(update_participant_pk, seed);

        let mut expect: SeedDictResult = HashMap::new();
        expect.insert(sum_participant_pk, seeds_map.clone());

        store
            .set_seed_dict_entry(&SeedDictEntry(sum_participant_pk, seeds_map))
            .await
            .unwrap();
        let get = store.get_seed_dict().await.unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_mask_dict() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();
        //store.clear_all().await.unwrap();

        let mask_hash = sha256::hash(&[0_u8; 100]);
        let expect: MaskDictResult = Counter::init(iter::once(mask_hash.clone()));

        store
            .set_mask_dict_entry(&MaskDictEntry(mask_hash))
            .await
            .unwrap();
        let get = store.get_mask_dict().await.unwrap();

        assert_eq!(expect, get);
    }

    // #[tokio::test(max_threads = 4)]
    // async fn test_set_100k_seed_dict_join() {
    //     let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();
    //     //store.clear_all().await.unwrap();

    //     let keys: Vec<(sign::PublicKey, box_::PublicKey)> = (0..100_000)
    //         .map(|_| {
    //             let (pk, _) = sign::gen_keypair();
    //             let (epk, _) = box_::gen_keypair();
    //             (pk, epk)
    //         })
    //         .collect();

    //     async fn gen_set_fut(
    //         rs: &RedisStore,
    //         pk: sign::PublicKey,
    //         epk: box_::PublicKey,
    //     ) -> Result<(), Box<dyn std::error::Error + 'static>> {
    //         let mut red = rs.clone();

    //         red.set_sum_dict_entry(&SumDictEntry(pk, epk)).await
    //     }
    //     let set_fut = keys
    //         .into_iter()
    //         .map(|(pk, epk)| gen_set_fut(&store, pk, epk));

    //     let now = Instant::now();
    //     let _ = future::try_join_all(set_fut).await.unwrap();
    //     let new_now = Instant::now();
    //     println!(
    //         "Time writing 10k seed dict entries {:?}",
    //         new_now.duration_since(now)
    //     );

    //     let now = Instant::now();
    //     store.get_seed_dict().await.unwrap();
    //     let new_now = Instant::now();
    //     println!(
    //         "Time reading 10k seed dict entries {:?}",
    //         new_now.duration_since(now)
    //     );
    // }

    #[tokio::test]
    async fn test_set_get_coord_pk() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        let pk = box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let expect = CoordinatorState::CoordPk(pk);

        store.set_coordinator_state(&expect).await.unwrap();
        let get = store
            .get_coordinator_state(&CoordinatorStateRequest::CoordPk)
            .await
            .unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_coord_sk() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        let (_, sk) = box_::gen_keypair();
        let expect = CoordinatorState::CoordSk(sk);

        store.set_coordinator_state(&expect).await.unwrap();
        let get = store
            .get_coordinator_state(&CoordinatorStateRequest::CoordSk)
            .await
            .unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_sum() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        let sum = 0.01_f64;
        let expect = CoordinatorState::Sum(sum);

        store.set_coordinator_state(&expect).await.unwrap();
        let get = store
            .get_coordinator_state(&CoordinatorStateRequest::Sum)
            .await
            .unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_update() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        let update = 0.01_f64;
        let expect = CoordinatorState::Update(update);

        store.set_coordinator_state(&expect).await.unwrap();
        let get = store
            .get_coordinator_state(&CoordinatorStateRequest::Update)
            .await
            .unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_seed() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        let seed = randombytes(80);
        let expect = CoordinatorState::Seed(seed);

        store.set_coordinator_state(&expect).await.unwrap();
        let get = store
            .get_coordinator_state(&CoordinatorStateRequest::Seed)
            .await
            .unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_min_sum() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        let min_sum: usize = 10;
        let expect = CoordinatorState::MinSum(min_sum);

        store.set_coordinator_state(&expect).await.unwrap();
        let get = store
            .get_coordinator_state(&CoordinatorStateRequest::MinSum)
            .await
            .unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_min_update() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        let min_update: usize = 10;
        let expect = CoordinatorState::MinUpdate(min_update);

        store.set_coordinator_state(&expect).await.unwrap();
        let get = store
            .get_coordinator_state(&CoordinatorStateRequest::MinUpdate)
            .await
            .unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_phase() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        let phase = Phase::Idle;
        let expect = CoordinatorState::Phase(phase);

        store.set_coordinator_state(&expect).await.unwrap();
        let get = store
            .get_coordinator_state(&CoordinatorStateRequest::Phase)
            .await
            .unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_round() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        let round = 100;
        let expect = CoordinatorState::Round(round);

        store.set_coordinator_state(&expect).await.unwrap();
        let get = store
            .get_coordinator_state(&CoordinatorStateRequest::Round)
            .await
            .unwrap();

        assert_eq!(expect, get);
    }
}
