use crate::{
    common::client::ClientId,
    coordinator::{core::CoordinatorHandle, models::json::*},
};
use tokio::net::TcpListener;
use tracing_futures::Instrument;
use warp::Filter;

pub async fn serve(bind_address: &str, handle: CoordinatorHandle) {
    let handle = warp::any().map(move || handle.clone());
    let parent_span = tracing::Span::current();

    let heartbeat = warp::path!("heartbeat" / ClientId)
        .and(warp::get())
        .and(handle.clone())
        .and_then(move |id, mut handle: CoordinatorHandle| {
            let span =
                trace_span!(parent: parent_span.clone(), "api_heartbeat_request", client_id = %id);
            async move {
                match handle.heartbeat(id).await {
                    Ok(response) => Ok(warp::reply::json(&HeartBeatResponseJson::from(response))),
                    Err(_) => Err(warp::reject::not_found()),
                }
            }
            .instrument(span)
        });

    let parent_span = tracing::Span::current();
    let rendez_vous = warp::path!("rendez_vous")
        .and(warp::get())
        .and(handle.clone())
        .and_then(move |mut handle: CoordinatorHandle| {
            let span = trace_span!(parent: parent_span.clone(), "api_rendez_vous_request");
            async move {
                match handle.rendez_vous().await {
                    Ok(response) => Ok(warp::reply::json(&RendezVousResponseJson::from(response))),
                    Err(_) => Err(warp::reject::not_found()),
                }
            }
            .instrument(span)
        });

    let parent_span = tracing::Span::current();
    let start_training = warp::path!("start_training" / ClientId)
        .and(warp::get())
        .and(handle.clone())
        .and_then(move |id, mut handle: CoordinatorHandle| {
            let span =
                trace_span!(parent: parent_span.clone(), "api_start_training_request", client_id = %id);
            async move {
                match handle.start_training(id).await {
                    Ok(response) => Ok(warp::reply::json(&StartTrainingResponseJson::from(
                        response,
                    ))),
                    Err(_) => Err(warp::reject::not_found()),
                }
            }.instrument(span)
        });

    let mut listener = TcpListener::bind(bind_address).await.unwrap();

    info!("starting HTTP server on {}", bind_address);
    let log = warp::log("http");
    warp::serve(heartbeat.or(rendez_vous).or(start_training).with(log))
        .run_incoming(listener.incoming())
        .await
}
