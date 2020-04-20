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
        partial_state: CoordinatorPartialState,
    ) -> RedisResult<()> {
        self.connection
            .set_multiple(&partial_state.to_args())
            .await?;
        Ok(())
    }

    async fn set_sum_dict_entry(&mut self, entry: SumDictEntry) -> RedisResult<()> {
        let entry = entry.to_args();
        self.connection.hset("sum_dict", entry.0, entry.1).await?;
        Ok(())
    }

    async fn set_seed_dict_entry(&mut self, entry: SeedDictEntry) -> RedisResult<()> {
        let entry = entry.to_args();
        self.connection.sadd("seed_dict", &entry.0).await?;
        self.connection
            .hset_multiple(format!("seed_dict:{}", &entry.0), &entry.1)
            .await?;
        Ok(())
    }

    async fn set_mask_dict_entry(&mut self, entry: MaskDictEntry) -> RedisResult<()> {
        self.connection.sadd("mask_dict", &entry.to_args()).await?;
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
        let result: SumDictResult = SumDictResult(self.connection.hgetall("sum_dict").await?);
        Ok(SumDict::try_from(result)?)
    }

    async fn get_seed_dict(&mut self) -> Result<SeedDict, Box<dyn std::error::Error + 'static>> {
        let seed_dict_keys: Vec<String> = self.connection.smembers("seed_dict").await?;

        let mut seed_dict: SeedDict = HashMap::new();
        for sum_key in seed_dict_keys.into_iter() {
            let seed_dict_fields: Vec<(String, String)> = self
                .connection
                .hgetall(format!("seed_dict:{}", &sum_key))
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
        let result = MaskDictResult(self.connection.smembers("mask_dict").await?);
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
            Coordinator, EncryptedMaskingSeed, SeedDict, SumDict, SumParticipantEphemeralPublicKey,
            SumParticipantPublicKey, UpdateParticipantPublicKey,
        },
        storage::append::types::*,
    };
    use redis::RedisResult;
    use sodiumoxide::{crypto::box_, randombytes::randombytes};
    use std::{collections::HashMap, time::Instant};

    #[tokio::test]
    async fn test_basic() {
        // create new coordinator
        let coordinator = Coordinator::new().unwrap();
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        store.clear_all().await.unwrap();

        store
            .set_partial_coordinator_state(CoordinatorPartialState::from(&coordinator))
            .await
            .unwrap();

        let read = store.get_partial_coordinator_state().await.unwrap();
        println!("Coordinator: {:?}", read);

        let sum_participant_pk = box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let sum_participant_epk = box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let mut result: SumDict = HashMap::new();
        result.insert(sum_participant_pk, sum_participant_epk);

        store
            .set_sum_dict_entry(SumDictEntry(sum_participant_pk, sum_participant_epk))
            .await
            .unwrap();
        let sum_dict = store.get_sum_dict().await.unwrap();

        println!("Sum dict: {:?}", &sum_dict);
        assert_eq!(result, sum_dict);

        let mut result_sub: HashMap<SumParticipantPublicKey, EncryptedMaskingSeed> = HashMap::new();
        result_sub.insert(sum_participant_pk, randombytes(80));
        let mut result: SeedDict = HashMap::new();
        result.insert(sum_participant_pk, result_sub.clone());

        store
            .set_seed_dict_entry(SeedDictEntry(sum_participant_pk, result_sub))
            .await
            .unwrap();

        let seed_dict = store.get_seed_dict().await.unwrap();
        println!("Seed dict: {:?}", seed_dict);
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
