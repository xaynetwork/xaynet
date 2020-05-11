use futures::stream::{FuturesUnordered, StreamExt};
use hex;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rusoto_core::Region;
use sodiumoxide::{crypto::hash::sha256, randombytes::randombytes};
use std::{convert::TryFrom, iter, time::Instant};
use tokio::task::JoinHandle;
use xain_fl::{
    coordinator::RoundSeed,
    crypto::{generate_integer, ByteObject},
    mask::{
        config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
        Integers,
        Mask,
        MaskedModel,
    },
    model::Model,
    proxy::serve,
    storage::s3::store::S3Store,
    MaskHash,
};

#[tokio::main]
async fn main() {
    let store = create_client().await;
    let (r, m) = create_global_model(10);
    let _ = store.upload_global_model(&r, &m).await;
    println!("{:?}", r);
    serve::<i32>("localhost:7325", store).await;
}

fn create_minio_setup() -> Region {
    Region::Custom {
        name: String::from("eu-east-3"),
        endpoint: String::from("http://127.0.0.1:9000"),
    }
}

async fn create_client() -> S3Store {
    let region = create_minio_setup();
    let s3_store = S3Store::new(region);
    s3_store.clear_all().await.unwrap();
    s3_store.create_buckets().await.unwrap();
    s3_store
}

fn create_global_model(byte_size: usize) -> (String, Model<i32>) {
    let mut rng = rand::thread_rng();
    (
        hex::encode(RoundSeed::generate().as_slice()),
        Model::try_from(
            (0..byte_size)
                .map(|_| rng.gen_range(1, 21))
                .collect::<Vec<i32>>(),
        )
        .unwrap(),
    )
}

fn create_masked_model(byte_size: usize) -> (String, MaskedModel) {
    let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
    let config = MaskConfigs::from_parts(
        GroupType::Prime,
        DataType::F32,
        BoundType::B0,
        ModelType::M3,
    )
    .config();
    let integers = iter::repeat_with(|| generate_integer(&mut prng, config.order()))
        .take(byte_size)
        .collect();
    (
        hex::encode(randombytes(32)),
        MaskedModel::from_parts(integers, config.clone()).unwrap(),
    )
}
