use crate::{
    aggregator::service::ServiceHandle,
    common::client::{ClientId, Credentials, Token},
};
use tokio::net::TcpListener;
use tracing_futures::Instrument;
use warp::{
    http::{Response, StatusCode},
    Filter,
};

pub async fn serve(bind_address: &str, handle: ServiceHandle) {
    let handle = warp::any().map(move || handle.clone());
    let parent_span = tracing::Span::current();

    let download_global_weights = warp::get()
        .and(warp::path::param::<ClientId>())
        .and(warp::path::param::<Token>())
        .and(handle.clone())
        .and_then(move |id, token, handle: ServiceHandle| {
            let span =
                trace_span!(parent: parent_span.clone(), "api_download_request", client_id = %id);
            async move {
                debug!("received download request");
                match handle.download(Credentials(id, token)).await {
                    Some(weights) => Ok(Response::builder().body(weights)),
                    None => Err(warp::reject::not_found()),
                }
            }
            .instrument(span)
        });

    let parent_span = tracing::Span::current();
    let upload_local_weights = warp::post()
        .and(warp::path::param::<ClientId>())
        .and(warp::path::param::<Token>())
        .and(warp::body::bytes())
        .and(handle.clone())
        .and_then(move |id, token, weights, handle: ServiceHandle| {
            let span =
                trace_span!(parent: parent_span.clone(), "api_upload_request", client_id = %id);

            async move {
                debug!("received upload request");
                handle.upload(Credentials(id, token), weights).await;
                Ok(StatusCode::OK) as Result<_, warp::reject::Rejection>
            }
            .instrument(span)
        });

    let mut listener = TcpListener::bind(bind_address).await.unwrap();

    info!("starting HTTP server on {}", bind_address);
    let log = warp::log("http");
    warp::serve(download_global_weights.or(upload_local_weights).with(log))
        .run_incoming(listener.incoming())
        .await
}
