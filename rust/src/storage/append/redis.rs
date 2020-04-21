use crate::{
    coordinator::{
        EncryptedMaskingSeed, MaskHash, SeedDict, SumDict, SumParticipantPublicKey,
        UpdateParticipantPublicKey,
    },
    storage::append::types::{
        CoordinatorPartialState, CoordinatorPartialStateResult, MaskDictEntry, MaskDictResult,
        SeedDictEntry, SeedDictKeyResult, SeedDictValueEntryResult, SumDictEntry, SumDictResult,
    },
};
use counter::Counter;
use redis::{aio::MultiplexedConnection, AsyncCommands, Client, RedisError, RedisResult};
use std::{collections::HashMap, convert::TryFrom};

#[derive(Clone)]
pub struct RedisStore {
    connection: MultiplexedConnection,
}

impl RedisStore {
    async fn new<S: Into<String>>(url: S) -> Result<Self, RedisError> {
        let client = Client::open(url.into())?;
        let connection = client.get_multiplexed_tokio_connection().await?;

        Ok(Self { connection })
    }

    async fn set_partial_coordinator_state(
        &mut self,
        partial_state: &CoordinatorPartialState,
    ) -> RedisResult<()> {
        self.connection
            .set_multiple(&partial_state.to_args())
            .await?;
        Ok(())
    }

    async fn set_sum_dict_entry(&mut self, entry: &SumDictEntry) -> RedisResult<()> {
        let (sum_pk, sum_epk) = entry.to_args();
        self.connection
            .hset(SumDictEntry::key(), sum_pk, sum_epk)
            .await?;
        Ok(())
    }

    async fn set_seed_dict_entry(&mut self, entry: &SeedDictEntry) -> RedisResult<()> {
        let (sum_pk, seeds_map) = entry.to_args();
        // Add sum_pk to a set of sum_pks
        self.connection.sadd(SeedDictEntry::key(), &sum_pk).await?;
        // Add seeds_map to a hashmap with the key seed_dict:<sum_pk>
        self.connection
            .hset_multiple(format!("{}:{}", &SeedDictEntry::key(), &sum_pk), &seeds_map)
            .await?;
        Ok(())
    }

    async fn set_mask_dict_entry(&mut self, entry: &MaskDictEntry) -> RedisResult<()> {
        self.connection
            .sadd(MaskDictEntry::key(), &entry.to_args())
            .await?;
        Ok(())
    }

    async fn get_partial_coordinator_state(
        &mut self,
    ) -> Result<CoordinatorPartialState, Box<dyn std::error::Error + 'static>> {
        let result = CoordinatorPartialStateResult(
            self.connection.get(CoordinatorPartialState::keys()).await?,
        );

        Ok(CoordinatorPartialState::try_from(result)?)
    }

    async fn get_sum_dict(&mut self) -> Result<SumDict, Box<dyn std::error::Error + 'static>> {
        let result: SumDictResult =
            SumDictResult(self.connection.hgetall(SumDictEntry::key()).await?);
        Ok(SumDict::try_from(result)?)
    }

    async fn get_seed_dict(&mut self) -> Result<SeedDict, Box<dyn std::error::Error + 'static>> {
        let seed_dict_keys: Vec<String> = self.connection.smembers(SeedDictEntry::key()).await?;

        let mut seed_dict: SeedDict = HashMap::new();
        for sum_key in seed_dict_keys.into_iter() {
            let seed_dict_fields: Vec<(String, String)> = self
                .connection
                .hgetall(format!("{}:{}", &SeedDictEntry::key(), &sum_key))
                .await?;
            let sub_dict: HashMap<UpdateParticipantPublicKey, EncryptedMaskingSeed> =
                HashMap::try_from(SeedDictValueEntryResult(seed_dict_fields))?;
            let key = SumParticipantPublicKey::try_from(SeedDictKeyResult(sum_key))?;
            seed_dict.insert(key, sub_dict);
        }

        Ok(seed_dict)
    }

    async fn get_mask_dict(
        &mut self,
    ) -> Result<Counter<MaskHash>, Box<dyn std::error::Error + 'static>> {
        let result = MaskDictResult(self.connection.smembers(MaskDictEntry::key()).await?);
        Ok(Counter::<MaskHash>::try_from(result)?)
    }

    async fn schedule_snapshot(&mut self) -> RedisResult<()> {
        redis::cmd("BGSAVE")
            .arg("SCHEDULE")
            .query_async(&mut self.connection)
            .await?;
        Ok(())
    }

    async fn clear_all(&mut self) -> RedisResult<()> {
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
        coordinator::{
            Coordinator, EncryptedMaskingSeed, MaskHash, SeedDict, SumDict,
            SumParticipantEphemeralPublicKey, SumParticipantPublicKey, UpdateParticipantPublicKey,
        },
        storage::append::types::*,
    };
    use counter::Counter;
    use redis::RedisResult;
    use sodiumoxide::{crypto::box_, randombytes::randombytes};
    use std::{collections::HashMap, iter, time::Instant};

    #[tokio::test]
    async fn test_set_get_partial_coordinator_state() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();
        store.clear_all().await.unwrap();

        // create new coordinator
        let coordinator = Coordinator::new().unwrap();
        let expect = CoordinatorPartialState::from(&coordinator);

        store.set_partial_coordinator_state(&expect).await.unwrap();
        let get = store.get_partial_coordinator_state().await.unwrap();

        assert_eq!(expect, get);
    }

    #[tokio::test]
    async fn test_set_get_sum_dict() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();
        store.clear_all().await.unwrap();

        let sum_participant_pk = box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let sum_participant_epk = box_::PublicKey([1_u8; box_::PUBLICKEYBYTES]);
        let mut expect: SumDict = HashMap::new();
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
        store.clear_all().await.unwrap();

        let sum_participant_pk = box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let update_participant_pk = box_::PublicKey([1_u8; box_::PUBLICKEYBYTES]);
        let seed = randombytes(80);

        let mut seeds_map: HashMap<UpdateParticipantPublicKey, EncryptedMaskingSeed> =
            HashMap::new();
        seeds_map.insert(update_participant_pk, seed);

        let mut expect: SeedDict = HashMap::new();
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
        store.clear_all().await.unwrap();

        let mask_hash = randombytes(80);
        let expect: Counter<MaskHash> = Counter::init(iter::once(mask_hash.clone()));

        store
            .set_mask_dict_entry(&MaskDictEntry(mask_hash))
            .await
            .unwrap();
        let get = store.get_mask_dict().await.unwrap();

        assert_eq!(expect, get);
    }

    // #[tokio::test]
    // async fn test_10k_loop() {
    //     let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

    //     let now = Instant::now();
    //     for _ in 0..10_000 {
    //         let (k, _) = box_::gen_keypair();
    //         store.set_sum_dict_entry(SumDictEntry(k, k)).await.unwrap();
    //     }
    //     let new_now = Instant::now();
    //     println!("Add sum dict keys {:?}", new_now.duration_since(now));
    // }

    // #[tokio::test]
    // async fn test_10k_join() {
    //     let store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

    //     async fn create(r: &RedisStore) -> RedisResult<()> {
    //         let mut red = r.clone();
    //         let (k, _) = box_::gen_keypair();
    //         red.set_sum_dict_entry(SumDictEntry(k, k)).await
    //     }
    //     let cmds = (0..10_000).map(|_| create(&store));

    //     let now = Instant::now();
    //     let _ = future::try_join_all(cmds).await.unwrap();
    //     let new_now = Instant::now();
    //     println!("Add sum dict keys {:?}", new_now.duration_since(now));
    // }
}
