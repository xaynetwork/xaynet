//! A HTTP API for the PET protocol interactions.

use std::{convert::Infallible, path::PathBuf};

use bytes::{Buf, Bytes};
use warp::{
    http::{Response, StatusCode},
    Filter,
};

use crate::services::{fetchers::Fetcher, messages::PetMessageHandler};
use crate::settings::ApiSettings;
use xaynet_core::{crypto::ByteObject, ParticipantPublicKey};

/// Starts a HTTP server at the given address, listening to GET requests for
/// data and POST requests containing PET messages.
///
/// * `api_settings`: address of the server and optional certificate and key for TLS server
///   authentication.
/// * `fetcher`: fetcher for responding to data requests.
/// * `pet_message_handler`: handler for responding to PET messages.
pub async fn serve<F>(api_settings: ApiSettings, fetcher: F, pet_message_handler: PetMessageHandler)
where
    F: Fetcher + Sync + Send + 'static + Clone,
{
    let message = warp::path!("message")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(with_message_handler(pet_message_handler.clone()))
        .and_then(handle_message);

    let sum_dict = warp::path!("sums")
        .and(warp::get())
        .and(with_fetcher(fetcher.clone()))
        .and_then(handle_sums);

    let seed_dict = warp::path!("seeds")
        .and(warp::get())
        .and(part_pk())
        .and(with_fetcher(fetcher.clone()))
        .and_then(handle_seeds);

    let length = warp::path!("length")
        .and(warp::get())
        .and(with_fetcher(fetcher.clone()))
        .and_then(handle_length);

    let round_params = warp::path!("params")
        .and(warp::get())
        .and(with_fetcher(fetcher.clone()))
        .and_then(handle_params);

    let model = warp::path!("model")
        .and(warp::get())
        .and(with_fetcher(fetcher.clone()))
        .and_then(handle_model);

    let routes = message
        .or(round_params)
        .or(sum_dict)
        .or(seed_dict)
        .or(length)
        .or(model)
        .recover(handle_reject)
        .with(warp::log("http"));

    if let (Some(cert), Some(key)) = (api_settings.tls_certificate, api_settings.tls_key) {
        // mixed cases are prevented by validation
        warp::serve(routes)
            .tls()
            .cert_path(PathBuf::from(cert))
            .key_path(PathBuf::from(key))
            .run(api_settings.bind_address)
            .await
    } else {
        warp::serve(routes).run(api_settings.bind_address).await
    }
}

/// Handles and responds to a PET message.
async fn handle_message(
    body: Bytes,
    mut handler: PetMessageHandler,
) -> Result<impl warp::Reply, Infallible> {
    let _ = handler.handle_message(body.to_vec()).await.map_err(|e| {
        warn!("failed to handle message: {:?}", e);
    });
    Ok(warp::reply())
}

/// Handles and responds to a request for the sum dictionary.
async fn handle_sums<F: Fetcher>(mut fetcher: F) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.sum_dict().await {
        Err(e) => {
            warn!("failed to handle sum dict request: {:?}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Vec::new())
                .unwrap()
        }
        Ok(None) => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Vec::new())
            .unwrap(),
        Ok(Some(dict)) => {
            let bytes = bincode::serialize(dict.as_ref()).unwrap();
            Response::builder()
                .header("Content-Type", "application/octet-stream")
                .status(StatusCode::OK)
                .body(bytes)
                .unwrap()
        }
    })
}

/// Handles and responds to a request for the seed dictionary.
async fn handle_seeds<F: Fetcher>(
    pk: ParticipantPublicKey,
    mut fetcher: F,
) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.seed_dict().await {
        Err(e) => {
            warn!("failed to handle seed dict request: {:?}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Vec::new())
                .unwrap()
        }
        Ok(Some(dict)) if dict.get(&pk).is_some() => {
            let bytes = bincode::serialize(dict.as_ref().get(&pk).unwrap()).unwrap();
            Response::builder()
                .header("Content-Type", "application/octet-stream")
                .status(StatusCode::OK)
                .body(bytes)
                .unwrap()
        }
        _ => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Vec::new())
            .unwrap(),
    })
}

/// Handles and responds to a request for mask / model length.
async fn handle_length<F: Fetcher>(mut fetcher: F) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.mask_length().await {
        Ok(Some(mask_length)) => Response::builder()
            .status(StatusCode::OK)
            .body(mask_length.to_string())
            .unwrap(),
        Ok(None) => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(String::new())
            .unwrap(),
        Err(e) => {
            warn!("failed to handle mask_length request: {:?}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(String::new())
                .unwrap()
        }
    })
}

/// Handles and responds to a request for the global model.
async fn handle_model<F: Fetcher>(mut fetcher: F) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.model().await {
        Ok(Some(model)) => Response::builder()
            .status(StatusCode::OK)
            .body(bincode::serialize(model.as_ref()).unwrap())
            .unwrap(),
        Ok(None) => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Vec::new())
            .unwrap(),
        Err(e) => {
            warn!("failed to handle model request: {:?}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Vec::new())
                .unwrap()
        }
    })
}

/// Handles and responds to a request for the round parameters.
async fn handle_params<F: Fetcher>(mut fetcher: F) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.round_params().await {
        Ok(params) => Response::builder()
            .status(StatusCode::OK)
            .body(bincode::serialize(&params).unwrap())
            .unwrap(),
        Err(e) => {
            warn!("failed to handle round parameters request: {:?}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Vec::new())
                .unwrap()
        }
    })
}

/// Converts a PET message handler into a `warp` filter.
fn with_message_handler(
    handler: PetMessageHandler,
) -> impl Filter<Extract = (PetMessageHandler,), Error = Infallible> + Clone {
    warp::any().map(move || handler.clone())
}

/// Converts a data fetcher into a `warp` filter.
fn with_fetcher<F: Fetcher + Sync + Send + 'static + Clone>(
    fetcher: F,
) -> impl Filter<Extract = (F,), Error = Infallible> + Clone {
    warp::any().map(move || fetcher.clone())
}

/// Extracts a participant public key from a request body
fn part_pk() -> impl Filter<Extract = (ParticipantPublicKey,), Error = warp::Rejection> + Clone {
    warp::body::bytes().and_then(|body: Bytes| async move {
        if let Some(pk) = ParticipantPublicKey::from_slice(body.bytes()) {
            Ok(pk)
        } else {
            Err(warp::reject::custom(InvalidPublicKey))
        }
    })
}

#[derive(Debug)]
struct InvalidPublicKey;

impl warp::reject::Reject for InvalidPublicKey {}

/// Handles `warp` rejections of bad requests.
async fn handle_reject(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    let code = if err.is_not_found() {
        StatusCode::NOT_FOUND
    } else if let Some(InvalidPublicKey) = err.find() {
        StatusCode::BAD_REQUEST
    } else {
        error!("unhandled rejection: {:?}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    };
    // reply with empty body; the status code is the interesting part
    Ok(warp::reply::with_status(Vec::new(), code))
}
