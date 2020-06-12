use crate::{
    crypto::ByteObject,
    services::{Fetcher, PetMessageHandler},
    ParticipantPublicKey,
};
use bytes::{Buf, Bytes};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use warp::{
    http::{Response, StatusCode},
    Filter,
};

pub async fn serve<F, MH>(
    addr: impl Into<SocketAddr> + 'static,
    fetcher: F,
    pet_message_handler: MH,
) where
    F: Fetcher + Sync + Send + 'static,
    MH: PetMessageHandler + Sync + Send + 'static,
{
    let fetcher = Arc::new(fetcher);
    let message_handler = Arc::new(pet_message_handler);
    let message = warp::path!("message")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(with_message_handler(message_handler.clone()))
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

    let scalar = warp::path!("scalar")
        .and(warp::get())
        .and(with_fetcher(fetcher.clone()))
        .and_then(handle_scalar);

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
        .or(scalar)
        .or(seed_dict)
        .or(length)
        .or(model)
        .recover(handle_reject);

    warp::serve(routes).run(addr).await
}

async fn handle_message<MH: PetMessageHandler>(
    body: Bytes,
    handler: Arc<MH>,
) -> Result<impl warp::Reply, Infallible> {
    let _ = handler
        .as_ref()
        .handle_message(body.to_vec())
        .await
        .map_err(|e| {
            warn!("failed to handle message: {:?}", e);
        });
    Ok(warp::reply())
}

async fn handle_sums<F: Fetcher>(fetcher: Arc<F>) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.as_ref().sum_dict().await {
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

async fn handle_seeds<F: Fetcher>(
    pk: ParticipantPublicKey,
    fetcher: Arc<F>,
) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.as_ref().seed_dict().await {
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

async fn handle_scalar<F: Fetcher>(fetcher: Arc<F>) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.as_ref().scalar().await {
        Ok(Some(scalar)) => Response::builder()
            .status(StatusCode::OK)
            .body(scalar.to_string())
            .unwrap(),
        Ok(None) => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(String::new())
            .unwrap(),
        Err(e) => {
            warn!("failed to handle scalar request: {:?}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(String::new())
                .unwrap()
        }
    })
}

async fn handle_length<F: Fetcher>(fetcher: Arc<F>) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.as_ref().mask_length().await {
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

async fn handle_model<F: Fetcher>(fetcher: Arc<F>) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.as_ref().model().await {
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
async fn handle_params<F: Fetcher>(fetcher: Arc<F>) -> Result<impl warp::Reply, Infallible> {
    Ok(match fetcher.as_ref().round_params().await {
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

fn with_message_handler<MH: PetMessageHandler + Send + Sync + 'static>(
    handler: Arc<MH>,
) -> impl Filter<Extract = (Arc<MH>,), Error = Infallible> + Clone {
    warp::any().map(move || handler.clone())
}

fn with_fetcher<F: Fetcher + Sync + Send + 'static>(
    fetcher: Arc<F>,
) -> impl Filter<Extract = (Arc<F>,), Error = Infallible> + Clone {
    warp::any().map(move || fetcher.clone())
}

/// Extract a participant public key from a request body
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
