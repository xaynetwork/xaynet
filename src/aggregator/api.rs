use crate::{
    aggregator::service::AggregatorServiceHandle,
    common::{ClientId, Token},
};

use tokio::net::TcpListener;
use warp::{
    http::{Response, StatusCode},
    Filter,
};

async fn serve(handle: AggregatorServiceHandle) {
    let handle = warp::any().map(move || handle.clone());

    let download_global_weights = warp::get()
        .and(warp::path::param::<ClientId>())
        .and(warp::path::param::<Token>())
        .and(handle.clone())
        .and_then(|id, token, handle: AggregatorServiceHandle| async move {
            match handle.get_global_weights(id, token).await {
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
                handle.set_local_weights(id, token, weights).await;
                Ok(StatusCode::OK) as Result<_, warp::reject::Rejection>
            },
        );

    let mut listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    warp::serve(download_global_weights.or(upload_local_weights))
        .run_incoming(listener.incoming())
        .await
}
