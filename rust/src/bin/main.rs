use futures::*;
use sodiumoxide::crypto::box_;
use std::time::Instant;
use xain_fl::storage::{redis::store::RedisStore, state::*};

#[tokio::main]
async fn main() {
    let mut store = RedisStore::new("redis://127.0.0.1/").await.unwrap();
    //store.clear_all().await.unwrap();

    let keys: Vec<box_::PublicKey> = (0..100_000)
        .map(|_| {
            let (pk, _) = box_::gen_keypair();
            pk
        })
        .collect();

    async fn gen_set_fut(
        rs: &RedisStore,
        pk: box_::PublicKey,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let mut red = rs.clone();

        red.set_sum_dict_entry(&SumDictEntry(pk, pk)).await
    }
    //let set_fut = keys.into_iter().map(|pk| gen_set_fut(&store, pk));

    let sum_dict_entries: Vec<SumDictEntry> = (0..100_000)
        .map(|_| {
            let (pk, _) = box_::gen_keypair();
            SumDictEntry(pk, pk)
        })
        .collect();

    let now = Instant::now();
    //let _ = future::try_join_all(set_fut).await.unwrap();
    store
        .set_sum_dict_entry_batch(&sum_dict_entries)
        .await
        .unwrap();
    let new_now = Instant::now();
    println!(
        "Time writing {:?} seed dict entries {:?}",
        sum_dict_entries.len(),
        new_now.duration_since(now)
    );

    let now = Instant::now();
    let map = store.get_sum_dict().await.unwrap();
    let new_now = Instant::now();
    println!(
        "Time reading {:?} seed dict entries {:?}",
        map.len(),
        new_now.duration_since(now)
    );
}
