use crate::{
    aggregator::service::AggregatorServiceHandle,
    common::{ClientId, Token},
};

use tokio::net::TcpListener;
use warp::{
    http::{Response, StatusCode},
    Filter,
};

pub async fn serve(bind_address: &str, handle: AggregatorServiceHandle) {
    let handle = warp::any().map(move || handle.clone());

    let download_global_weights = warp::get()
        .and(warp::path::param::<ClientId>())
        .and(warp::path::param::<Token>())
        .and(handle.clone())
        .and_then(|id, token, handle: AggregatorServiceHandle| async move {
            debug!("received download request for {}", id);
            match handle.download(id, token).await {
                Some(weights) => Ok(Response::builder().body(weights)),
                None => Err(warp::reject::not_found()),
            }
        });

    let upload_local_weights = warp::post()
        .and(warp::path::param::<ClientId>())
        .and(warp::path::param::<Token>())
        .and(warp::body::bytes())
        .and(handle.clone())
        .and_then(
            |id, token, weights, handle: AggregatorServiceHandle| async move {
                debug!("received upload request for {}", id);
                handle.upload(id, token, weights).await;
                Ok(StatusCode::OK) as Result<_, warp::reject::Rejection>
            },
        );

    let mut listener = TcpListener::bind(bind_address).await.unwrap();

    info!("starting HTTP server on {}", bind_address);
    let log = warp::log("http");
    warp::serve(download_global_weights.or(upload_local_weights).with(log))
        .run_incoming(listener.incoming())
        .await
}
