use crate::{
    mask::{Integers, Mask, MaskedModel},
    storage::s3::store::S3Store,
};
use bytes::Bytes;
use sodiumoxide::crypto::hash::sha256;

use tokio::net::TcpListener;
use warp::{
    http::{header::CONTENT_TYPE, method::Method, Response, StatusCode},
    reject::{Reject, Rejection},
    reply::Reply,
    Filter,
};

pub mod models;

async fn handle_upload_mask_request(
    handle: S3Store,
    se_mask: Bytes,
) -> Result<impl Reply, Rejection> {
    debug!("handling mask upload request");
    // calculate hash so that we don't save same the mask more then one time
    let mask_hash = sha256::hash(&se_mask).as_ref().to_vec();
    let key = hex::encode(mask_hash);

    let mask = Mask::deserialize(&se_mask.to_vec());
    if mask.is_err() {
        return Err(warp::reject::not_found());
    }

    match handle.upload_mask(&key, &mask.unwrap()).await {
        Ok(_) => Ok(warp::reply::json(&models::MaskResponse { key: Some(key) })),
        Err(_) => Err(warp::reject::not_found()),
    }
}

async fn handle_upload_masked_model_request(
    handle: S3Store,
    se_masked_model: Bytes,
) -> Result<impl Reply, Rejection> {
    debug!("handling masked model upload request");
    // calculate hash so that we don't save same the masked model more then one time
    let masked_model_hash = sha256::hash(&se_masked_model).as_ref().to_vec();
    let key = hex::encode(masked_model_hash);

    let masked_model = MaskedModel::deserialize(&se_masked_model.to_vec());
    if masked_model.is_err() {
        return Err(warp::reject::not_found());
    }

    match handle
        .upload_masked_model(&key, &masked_model.unwrap())
        .await
    {
        Ok(_) => Ok(warp::reply::json(&models::MaskedModelResponse {
            key: Some(key),
        })),
        Err(_) => Err(warp::reject::not_found()),
    }
}

async fn handle_download_global_model_request<N>(
    handle: S3Store,
    round_seed: String,
) -> Result<impl Reply, Rejection>
where
    N: for<'de> serde::Deserialize<'de> + serde::Serialize,
{
    debug!("handling global model download request");
    match handle.download_global_model::<N>(&round_seed).await {
        Ok(global_model) => Ok(warp::reply::json(&models::GlobalModelResponse {
            global_model: global_model,
        })),
        Err(_) => Err(warp::reject::not_found()),
    }
}

pub async fn serve<N>(bind_address: &str, store: S3Store)
where
    N: for<'de> serde::Deserialize<'de> + serde::Serialize,
{
    let handle = warp::any().map(move || store.clone());

    let upload_mask = warp::path!("upload" / "mask")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(handle.clone())
        .and_then(move |mask, handle: S3Store| handle_upload_mask_request(handle, mask))
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_method(Method::POST)
                .allow_header(CONTENT_TYPE),
        );

    let upload_masked_model = warp::path!("upload" / "masked_model")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(handle.clone())
        .and_then(move |masked_model, handle: S3Store| {
            handle_upload_masked_model_request(handle, masked_model)
        })
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_method(Method::POST)
                .allow_header(CONTENT_TYPE),
        );

    let download_global_model = warp::path!("download" / "global_model" / String)
        .and(warp::get())
        .and(handle.clone())
        .and_then(move |round_seed, handle: S3Store| {
            handle_download_global_model_request::<N>(handle, round_seed)
        })
        .with(warp::cors().allow_any_origin().allow_method(Method::GET));

    let mut listener = TcpListener::bind(bind_address).await.unwrap();

    let log = warp::log("http");
    warp::serve(
        upload_mask
            .or(upload_masked_model)
            .or(download_global_model)
            .with(log),
    )
    .run_incoming(listener.incoming())
    .await
}

// 38c040498eaf4829ead65289b5d28549cb9f5ec33a70082f08188a480fb05f22
// println!("{:?}", hex::encode(RoundSeed::generate().as_slice()));
