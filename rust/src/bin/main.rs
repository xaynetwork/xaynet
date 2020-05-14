use futures::stream::{FuturesUnordered, StreamExt};
use rayon::prelude::*;
use sodiumoxide::randombytes::randombytes;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::task::JoinHandle;
use xain_fl::{
    crypto::*,
    mask::seed::MaskSeed,
    storage::store::RedisStore,
    LocalSeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

#[tokio::main]
async fn main() {
    let n_sum_participants = 1_000;
    let n_update_participants = 1_000;
    let n_concurrent_requests = 1000;

    println!(
        "generating {} keys for the sum participants",
        n_sum_participants
    );
    let sum_dict_entries: Vec<(SumParticipantPublicKey, SumParticipantEphemeralPublicKey)> = (0
        ..n_sum_participants)
        .map(|_| {
            let (pk, _) = generate_signing_key_pair();
            let (epk, _) = generate_encrypt_key_pair();
            (pk, epk)
        })
        .collect();

    println!("connecting to redis");
    let store = RedisStore::new("redis://127.0.0.1/", n_concurrent_requests)
        .await
        .unwrap();

    println!("clearing redis");
    store.clone().connection().await.flushdb().await.unwrap();
    println!("done");

    println!(
        "creating a {} sum dict, {} requests at a time",
        n_sum_participants, n_concurrent_requests
    );

    // The FuturesUnordered is basically a big "join" for a bunch of
    // futures: it waits for them to complete, in any order. Note that
    // we don't run the actual requests in FuturesUnordered. We first
    // spawn all the requests, and give the corresponding JoinHandles
    // to FuturesUnordered. This is because FuturesUnordered runs all
    // its futures in a single thread, whereas `tokio::spawn` makes
    // use of the whole threadpool.
    let mut futures = FuturesUnordered::<JoinHandle<Result<usize, redis::RedisError>>>::new();

    let now = Instant::now();

    for (pk, epk) in sum_dict_entries {
        let fut = store
            .clone()
            .connection()
            .await
            .add_sum_participant(pk, epk);
        // spawn the future, and give the handle to FuturesUnordered
        futures.push(tokio::spawn(fut));
    }

    // wait for all the requests to finish
    loop {
        if futures.next().await.is_none() {
            break;
        }
    }

    let new_now = Instant::now();
    println!("done in {:?}", new_now.duration_since(now));

    println!("retrieving the sum dictionary");
    let now = Instant::now();
    let sum_dict = store
        .clone()
        .connection()
        .await
        .get_sum_dict()
        .await
        .unwrap();
    let new_now = Instant::now();
    println!("done in {:?}", new_now.duration_since(now));

    println!("generating {} updates", n_update_participants);
    let updates = Arc::new(Mutex::new(vec![]));
    rayon::iter::repeat(())
        .take(n_update_participants)
        .for_each(|_| {
            let update = generate_update(&sum_dict);
            updates.clone().lock().unwrap().push(update);
        });
    println!("done");

    println!("creating the seed dictionary");
    let now = Instant::now();
    for (pk, update) in Arc::try_unwrap(updates).unwrap().into_inner().unwrap() {
        store
            .clone()
            .connection()
            .await
            .update_seed_dict(pk, update)
            .await
            .unwrap();
    }
    let new_now = Instant::now();
    println!("done in {:?}", new_now.duration_since(now));
}

fn generate_update(sum_dict: &SumDict) -> (UpdateParticipantPublicKey, LocalSeedDict) {
    let seed = MaskSeed::generate();
    let pk = UpdateParticipantPublicKey::from_slice(&randombytes(32)).unwrap();
    let local_seed_dict = sum_dict
        .iter()
        .map(|(sum_pk, sum_ephm_pk)| (*sum_pk, seed.encrypt(sum_ephm_pk)))
        .collect::<LocalSeedDict>();
    (pk, local_seed_dict)
}
