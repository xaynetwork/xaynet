use crate::{
    coordinator::Coordinator,
    storage::append::types::{CoordinatorPartialState, MaskDictEntry, SeedDictEntry, SumDictEntry},
};
use redis::{aio::MultiplexedConnection, AsyncCommands, Client, RedisError};

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
        coordinator: &Coordinator,
    ) -> Result<(), RedisError> {
        self.connection
            .set_multiple(&CoordinatorPartialState::from(coordinator).to_args())
            .await?;
        Ok(())
    }

    async fn set_sum_dict_entry(&mut self, entry: SumDictEntry) -> Result<(), RedisError> {
        let entry = entry.to_args();
        self.connection.hset("sum_dict", entry.0, entry.1).await?;
        Ok(())
    }

    async fn set_seed_dict_entry(&mut self, entry: SeedDictEntry) -> Result<(), RedisError> {
        let entry = entry.to_args();
        self.connection.sadd("seed_dict", &entry.0).await?;
        self.connection
            .hset_multiple(format!("seed_dict:{}", &entry.0), &entry.1)
            .await?;
        Ok(())
    }

    async fn set_mask_dict_entry(&mut self, entry: MaskDictEntry) -> Result<(), RedisError> {
        self.connection.sadd("mask_dict", &entry.to_args()).await?;
        Ok(())
    }

    async fn schedule_snapshot(&mut self) -> Result<(), RedisError> {
        redis::cmd("BGSAVE")
            .arg("SCHEDULE")
            .query_async(&mut self.connection)
            .await?;
        Ok(())
    }

    async fn clear_all(&mut self) -> Result<(), RedisError> {
        redis::cmd("FLUSHDB")
            .arg("ASYNC")
            .query_async(&mut self.connection)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::coordinator::{Coordinator, EncryptedMaskingSeed, UpdateParticipantPublicKey};
    use redis::RedisResult;
    use sodiumoxide::{crypto::box_, randombytes::randombytes};
    use std::{collections::HashMap, time::Instant};

    #[tokio::test]
    async fn test_basic() {
        // create new coordinator
        let coordinator = Coordinator::new().unwrap();
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();
        store
            .set_partial_coordinator_state(&coordinator)
            .await
            .unwrap();

        let (k, _) = box_::gen_keypair();
        store.set_sum_dict_entry(SumDictEntry(k, k)).await.unwrap();

        let (k, _) = box_::gen_keypair();
        let mut sub_dict: HashMap<UpdateParticipantPublicKey, EncryptedMaskingSeed> =
            HashMap::new();
        sub_dict.insert(k, randombytes(80));
        store
            .set_seed_dict_entry(SeedDictEntry(k, sub_dict))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_10k_loop() {
        let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        let now = Instant::now();
        for _ in 0..10_000 {
            let (k, _) = box_::gen_keypair();
            store.set_sum_dict_entry(SumDictEntry(k, k)).await.unwrap();
        }
        let new_now = Instant::now();
        println!("Add sum dict keys {:?}", new_now.duration_since(now));
    }

    #[tokio::test]
    async fn test_10k_join() {
        let store = RedisStore::new("redis://127.0.0.1/").await.unwrap();

        async fn create(r: &RedisStore) -> RedisResult<()> {
            let mut red = r.clone();
            let (k, _) = box_::gen_keypair();
            red.set_sum_dict_entry(SumDictEntry(k, k)).await
        }
        let cmds = (0..10_000).map(|_| create(&store));

        let now = Instant::now();
        let _ = future::try_join_all(cmds).await.unwrap();
        let new_now = Instant::now();
        println!("Add sum dict keys {:?}", new_now.duration_since(now));
    }
}
