//! A HTTP API for the PET protocol interactions.

use std::convert::Infallible;
#[cfg(feature = "tls")]
use std::path::PathBuf;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, warn};
use warp::{
    http::{Response, StatusCode},
    reply::Reply,
    Filter,
};
#[cfg(feature = "tls")]
use warp::{Server, TlsServer};

use crate::{
    services::{fetchers::Fetcher, messages::PetMessageHandler},
    settings::ApiSettings,
};
use xaynet_core::{crypto::ByteObject, ParticipantPublicKey};

#[derive(Deserialize, Serialize)]
struct SeedDictQuery {
    pk: String,
}

/// Starts a HTTP server at the given address, listening to GET requests for
/// data and POST requests containing PET messages.
///
/// * `api_settings`: address of the server and optional certificate and key for TLS server
///   authentication as well as trusted anchors for TLS client authentication.
/// * `fetcher`: fetcher for responding to data requests.
/// * `pet_message_handler`: handler for responding to PET messages.
///
/// # Errors
/// Fails if the TLS settings are invalid.
pub async fn serve<F>(
    api_settings: ApiSettings,
    fetcher: F,
    pet_message_handler: PetMessageHandler,
) -> Result<(), RestError>
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
        .and(warp::query::<SeedDictQuery>())
        .and_then(part_pk)
        .and(with_fetcher(fetcher.clone()))
        .and_then(handle_seeds);

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
        .or(model)
        .recover(handle_reject)
        .with(warp::log("http"));

    #[cfg(not(feature = "tls"))]
    return run_http(routes, api_settings)
        .await
        .map_err(RestError::from);
    #[cfg(feature = "tls")]
    return run_https(routes, api_settings).await;
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

/// Extracts a participant public key from the url query string
async fn part_pk(query: SeedDictQuery) -> Result<ParticipantPublicKey, warp::Rejection> {
    match base64::decode(query.pk.as_bytes()) {
        Ok(bytes) => {
            if let Some(pk) = ParticipantPublicKey::from_slice(&bytes[..]) {
                Ok(pk)
            } else {
                Err(warp::reject::custom(InvalidPublicKey))
            }
        }
        Err(_) => Err(warp::reject::custom(InvalidPublicKey)),
    }
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

#[derive(Debug, Error)]
/// Errors of the rest server.
pub enum RestError {
    #[error("invalid TLS configuration was provided")]
    InvalidTlsConfig,
}

impl From<Infallible> for RestError {
    fn from(infallible: Infallible) -> RestError {
        match infallible {}
    }
}

#[cfg(feature = "tls")]
/// Configures a server for TLS server and client authentication.
///
/// # Errors
/// Fails if the TLS settings are invalid.
fn configure_tls<F>(
    server: Server<F>,
    tls_certificate: Option<PathBuf>,
    tls_key: Option<PathBuf>,
    tls_client_auth: Option<PathBuf>,
) -> Result<TlsServer<F>, RestError>
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: Reply,
{
    if tls_certificate.is_none() && tls_key.is_none() && tls_client_auth.is_none() {
        return Err(RestError::InvalidTlsConfig);
    }

    let mut server = server.tls();
    match (tls_certificate, tls_key) {
        (Some(cert), Some(key)) => server = server.cert_path(cert).key_path(key),
        (None, None) => {}
        _ => return Err(RestError::InvalidTlsConfig),
    }
    // if let Some(trust_anchor) = tls_client_auth {
    //     server = server.client_auth_required_path(trust_anchor);
    // }
    Ok(server)
}

#[cfg(not(feature = "tls"))]
/// Runs a server with the provided filter routes.
async fn run_http<F>(filter: F, api_settings: ApiSettings) -> Result<(), Infallible>
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: Reply,
{
    warp::serve(filter).run(api_settings.bind_address).await;
    Ok(())
}

#[cfg(feature = "tls")]
/// Runs a TLS server with the provided filter routes.
///
/// # Errors
/// Fails if the TLS settings are invalid.
async fn run_https<F>(filter: F, api_settings: ApiSettings) -> Result<(), RestError>
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: Reply,
{
    configure_tls(
        warp::serve(filter),
        api_settings.tls_certificate,
        api_settings.tls_key,
        api_settings.tls_client_auth,
    )?
    .run(api_settings.bind_address)
    .await;
    Ok(())
}
