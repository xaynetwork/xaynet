use crate::{
    aggregator::service::{Aggregator, ServiceHandle},
    common::client::{ClientId, Credentials, Token},
};
use tokio::net::TcpListener;
use tracing_futures::Instrument;
use warp::{
    http::{header::CONTENT_TYPE, method::Method, Response, StatusCode},
    Filter,
};

pub async fn serve<A: Aggregator + 'static>(bind_address: &str, handle: ServiceHandle<A>) {
    let handle = warp::any().map(move || handle.clone());
    let parent_span = tracing::Span::current();

    let download_global_weights = warp::get()
        .and(warp::path::param::<ClientId>())
        .and(warp::path::param::<Token>())
        .and(handle.clone())
        .and_then(move |id, token, handle: ServiceHandle<A>| {
            let span =
                trace_span!(parent: parent_span.clone(), "api_download_request", client_id = %id);
            async move {
                debug!("received download request");
                match handle.download(Credentials(id, token)).await {
                    Ok(weights) => Ok(Response::builder().body(weights)),
                    Err(_) => Err(warp::reject::not_found()),
                }
            }
            .instrument(span)
        })
        .with(warp::cors().allow_any_origin().allow_method(Method::GET))
        // We need to send the this content type back, otherwise the swagger ui does not understand
        // that the data is binary data.
        // Without the "content-type", swagger will show the data as text.
        .with(warp::reply::with::header(
            "Content-Type",
            "application/octet-stream",
        ));

    let parent_span = tracing::Span::current();
    let upload_local_weights = warp::post()
        .and(warp::path::param::<ClientId>())
        .and(warp::path::param::<Token>())
        .and(warp::body::bytes())
        .and(handle.clone())
        .and_then(move |id, token, weights, handle: ServiceHandle<A>| {
            let span =
                trace_span!(parent: parent_span.clone(), "api_upload_request", client_id = %id);

            async move {
                debug!("received upload request");
                match handle.upload(Credentials(id, token), weights).await {
                    Ok(()) => Ok(StatusCode::OK),
                    Err(_) => Err(warp::reject::not_found()),
                }
            }
            .instrument(span)
        })
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_method(Method::POST)
                // Allow the "content-typ" header which is requested in the CORS preflight request.
                // Without this header, we will get an CORS error in the swagger ui.
                .allow_header(CONTENT_TYPE),
        );

    let mut listener = TcpListener::bind(bind_address).await.unwrap();

    info!("starting HTTP server on {}", bind_address);
    let log = warp::log("http");
    warp::serve(download_global_weights.or(upload_local_weights).with(log))
        .run_incoming(listener.incoming())
        .await
}
