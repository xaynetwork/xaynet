use crate::{
    mask::{Integers, Mask, MaskedModel},
    storage::s3::store::{S3Store, StorageError},
    PetError,
};
use sodiumoxide::crypto::hash::sha256;
use std::convert::Infallible;
use tokio::net::TcpListener;
use warp::{
    http::{header::CONTENT_TYPE, method::Method, Response, StatusCode},
    reject::{Reject, Rejection},
    reply::Reply,
    Filter,
};

pub mod models;
// API Handlers

async fn handle_upload_mask_request(
    store: S3Store,
    se_mask: Vec<u8>,
) -> Result<impl Reply, Rejection> {
    debug!("handling mask upload request");

    // calculate hash so that we don't save same the mask more then one time
    let mask_hash = sha256::hash(&se_mask).as_ref().to_vec();
    let key = hex::encode(mask_hash);

    let mask = Mask::deserialize(&se_mask).map_err(warp::reject::custom)?;

    store
        .upload_mask(&key, &mask)
        .await
        .map(|_| warp::reply::json(&models::MaskResponse { key }))
        .map_err(warp::reject::custom)
}

async fn handle_upload_masked_model_request(
    store: S3Store,
    se_masked_model: Vec<u8>,
) -> Result<impl Reply, Rejection> {
    debug!("handling masked model upload request");

    // calculate hash so that we don't save same the masked model more then one time
    let masked_model_hash = sha256::hash(&se_masked_model).as_ref().to_vec();
    let key = hex::encode(masked_model_hash);

    let masked_model = MaskedModel::deserialize(&se_masked_model).map_err(warp::reject::custom)?;

    store
        .upload_masked_model(&key, &masked_model)
        .await
        .map(|_| warp::reply::json(&models::MaskedModelResponse { key }))
        .map_err(warp::reject::custom)
}

async fn handle_download_global_model_request<N>(
    store: S3Store,
    round_seed: String,
) -> Result<impl Reply, Rejection>
where
    N: for<'de> serde::Deserialize<'de> + serde::Serialize,
{
    debug!("handling global model download request");
    store
        .download_global_model::<N>(&round_seed)
        .await
        .map_err(warp::reject::custom)
        .map(|global_model| match global_model {
            Some(model) => Ok(warp::reply::json(&models::GlobalModelResponse { model })),
            // TODO: if the model does not exist the API returns an S3error because the non
            // existence of the model is treated as an error in the S3 API
            None => Err(warp::reject::not_found()),
        })?
}

pub async fn serve<N>(bind_address: &str, store: S3Store)
where
    N: for<'de> serde::Deserialize<'de> + serde::Serialize,
{
    let store = warp::any().map(move || store.clone());

    let upload_mask = warp::path!("upload" / "mask")
        .and(warp::post())
        .and(warp::body::json())
        .and(store.clone())
        .and_then(move |mask_json: models::MaskRequest, store: S3Store| {
            handle_upload_mask_request(store, mask_json.mask)
        })
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_method(Method::POST)
                .allow_header(CONTENT_TYPE),
        );

    let upload_masked_model = warp::path!("upload" / "masked_model")
        .and(warp::post())
        .and(warp::body::json())
        .and(store.clone())
        .and_then(
            move |masked_model_json: models::MaskedModelRequest, store: S3Store| {
                handle_upload_masked_model_request(store, masked_model_json.model)
            },
        )
        .recover(handle_pet_rejection)
        .recover(handle_storage_rejection)
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_method(Method::POST)
                .allow_header(CONTENT_TYPE),
        );

    let download_global_model = warp::path!("download" / "global_model" / String)
        .and(warp::get())
        .and(store.clone())
        .and_then(move |round_seed, store: S3Store| {
            handle_download_global_model_request::<N>(store, round_seed)
        })
        .recover(handle_storage_rejection)
        .with(warp::cors().allow_any_origin().allow_method(Method::GET));

    let mut listener = TcpListener::bind(bind_address).await.unwrap();

    let log = warp::log("http");
    warp::serve(
        upload_mask
            .or(upload_masked_model)
            .or(download_global_model)
            //.or(download_masked_model)
            .with(log),
    )
    .run_incoming(listener.incoming())
    .await
}

// Error handling

impl Reject for StorageError {}
impl Reject for PetError {}

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: &'static str,
}

type JsonErrorReply = warp::reply::WithStatus<warp::reply::Json>;

/// Create a JSON response from a status code and an error message
fn error(code: StatusCode, message: &'static str) -> JsonErrorReply {
    let msg = ErrorResponse {
        code: code.into(),
        message,
    };
    let json = warp::reply::json(&msg);
    warp::reply::with_status(json, code)
}

fn service_unavailable() -> JsonErrorReply {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Service temporarily un-available",
    )
}

fn bad_request() -> JsonErrorReply {
    error(
        StatusCode::BAD_REQUEST,
        StatusCode::BAD_REQUEST.canonical_reason().unwrap(),
    )
}

/// Create a json response from a `Rejection`
async fn handle_rejection(e: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if e.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if e.find::<warp::reject::MethodNotAllowed>().is_some() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        // oops... Just log and say its a 500
        error!("Unhandled rejection: {:?}", e);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }
    Ok(error(code, message))
}

async fn handle_pet_rejection(e: Rejection) -> Result<impl Reply, Rejection> {
    e.find::<PetError>()
        .map(|_| Ok(bad_request()))
        .ok_or_else(|| e)
}

async fn handle_storage_rejection(e: Rejection) -> Result<impl Reply, Rejection> {
    e.find::<StorageError>()
        .map(|_| Ok(service_unavailable()))
        .ok_or_else(|| e)
}

fn create_masked_model(byte_size: usize) -> (String, MaskedModel) {
    use crate::{
        crypto::generate_integer,
        mask::config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
    };

    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use sodiumoxide::randombytes::randombytes;
    use std::iter;

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
// example:
// "masked_model": [1,0,0,3,49,237,31,40,81,10,30,81,80,215,178,4,25,212,240,145,2,18,179,35,15,254,219,11,227,179,209,4,13,11,214,151,191,94,166,1,228,28,235,89,4,18,196,123,223,241,42,6,184,158,34,241,26,8,196,255,240,184,108,2]

fn create_mask(byte_size: usize) -> (String, Mask) {
    use crate::mask::config::{BoundType, DataType, GroupType, MaskConfigs, ModelType};
    use num::{bigint::BigUint, traits::identities::Zero};
    use sodiumoxide::randombytes::randombytes;

    let config = MaskConfigs::from_parts(
        GroupType::Prime,
        DataType::F32,
        BoundType::B0,
        ModelType::M3,
    )
    .config();

    (
        hex::encode(randombytes(32)),
        Mask::from_parts(vec![BigUint::zero(); byte_size], config.clone()).unwrap(),
    )
}
// example:
// "mask": [1,0,0,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]

// example key global model
// 38c040498eaf4829ead65289b5d28549cb9f5ec33a70082f08188a480fb05f22
// println!("{:?}", hex::encode(RoundSeed::generate().as_slice()));
