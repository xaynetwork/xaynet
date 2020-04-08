use crate::{
    aggregator::service::{Aggregator, DownloadError, ServiceError, ServiceHandle, UploadError},
    common::client::{ClientId, Credentials, Token},
};
use bytes::Bytes;
use std::{convert::Infallible, error::Error};
use tokio::net::TcpListener;
use tracing_futures::Instrument;
use warp::{
    http::{header::CONTENT_TYPE, method::Method, Response, StatusCode},
    reject::{Reject, Rejection},
    reply::Reply,
    Filter,
};

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: &'static str,
}

// Make it possible to turn a ServiceError into a Rejection
impl<E> Reject for ServiceError<E> where E: Error + Sized + Send + Sync + 'static {}

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

fn unauthorized(message: &'static str) -> JsonErrorReply {
    error(StatusCode::UNAUTHORIZED, message)
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

/// Handler for global weights download requests
async fn handle_download_request<A: Aggregator + 'static>(
    id: ClientId,
    token: Token,
    handle: ServiceHandle<A>,
) -> Result<impl Reply, Rejection> {
    debug!("handling download request");
    handle
        .download(Credentials(id, token))
        .await
        .map(|weights| {
            Response::builder()
                .header("Content-Type", "application/octet-stream")
                .body(weights)
        })
        .map_err(warp::reject::custom)
}

/// Handler for local weights upload requests
async fn handle_upload_request<A: Aggregator + 'static>(
    id: ClientId,
    token: Token,
    weights: Bytes,
    handle: ServiceHandle<A>,
) -> Result<impl Reply, Rejection> {
    debug!("handling upload request");
    handle
        .upload(Credentials(id, token), weights)
        .await
        .map(|()| StatusCode::OK)
        .map_err(warp::reject::custom)
}

async fn handle_upload_rejection(e: Rejection) -> Result<impl Reply, Rejection> {
    e.find::<ServiceError<UploadError>>()
        .map(|e| {
            Ok(match e {
                ServiceError::Handle(_) => service_unavailable(),
                ServiceError::Request(UploadError::Unauthorized) => {
                    unauthorized("Not authorized to upload local model weights")
                }
            })
        })
        .ok_or_else(|| e)
}

async fn handle_download_rejection(e: Rejection) -> Result<impl Reply, Rejection> {
    e.find::<ServiceError<DownloadError>>()
        .map(|e| {
            Ok(match e {
                ServiceError::Handle(_) => service_unavailable(),
                ServiceError::Request(DownloadError::Unauthorized) => {
                    unauthorized("Not authorized to retrieve the global model weights")
                }
            })
        })
        .ok_or_else(|| e)
}

pub async fn serve<A: Aggregator + 'static>(bind_address: &str, handle: ServiceHandle<A>) {
    let handle = warp::any().map(move || handle.clone());
    let parent_span = tracing::Span::current();

    let download_global_weights = warp::get()
        .and(warp::path::param::<ClientId>())
        .and(warp::path::param::<Token>())
        .and(handle.clone())
        .and_then(move |id, token, handle| {
            let span =
                trace_span!(parent: parent_span.clone(), "api_download_request", client_id = %id);
            handle_download_request(id, token, handle).instrument(span)
        })
        .recover(handle_download_rejection)
        .with(warp::cors().allow_any_origin().allow_method(Method::GET));

    let parent_span = tracing::Span::current();
    let upload_local_weights = warp::post()
        .and(warp::path::param::<ClientId>())
        .and(warp::path::param::<Token>())
        .and(warp::body::bytes())
        .and(handle.clone())
        .and_then(move |id, token, weights, handle: ServiceHandle<A>| {
            let span =
                trace_span!(parent: parent_span.clone(), "api_upload_request", client_id = %id);
            handle_upload_request(id, token, weights, handle).instrument(span)
        })
        .recover(handle_upload_rejection)
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_method(Method::POST)
                // Allow the "content-type" header which is requested
                // in the CORS preflight request. Without this header,
                // we will get an CORS error in the swagger ui.
                .allow_header(CONTENT_TYPE),
        );

    let mut listener = TcpListener::bind(bind_address).await.unwrap();

    info!("starting HTTP server on {}", bind_address);
    let log = warp::log("http");
    warp::serve(
        download_global_weights
            .or(upload_local_weights)
            .recover(handle_rejection)
            .with(log),
    )
    .run_incoming(listener.incoming())
    .await
}
