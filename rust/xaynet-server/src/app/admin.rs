use crate::app::drain::Watch;
use crate::app::Readiness;
use crate::app::Tracing;
use futures::Future;
use warp::{
    http::{Response, StatusCode},
    reply::Reply,
    Filter,
};

// Settings
// bindaddress
// enable

pub fn build(
    shutdown: Watch,
    ready: Readiness,
    trace: Tracing,
) -> impl Future<Output = ()> + 'static {
    tracing::debug!("initialize");
    let ready = warp::path("ready")
        .and(warp::get())
        .and(warp::any().map(move || ready.clone()))
        .map(|ready: Readiness| match ready.is_ready() {
            true => StatusCode::OK,
            false => StatusCode::SERVICE_UNAVAILABLE,
        });

    let live = warp::path("live").and(warp::get()).map(|| StatusCode::OK);
    let filter = warp::path("filter")
        .and(warp::put())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::bytes())
        .and(warp::any().map(move || trace.clone()))
        .map(|body, trace: Tracing| match trace.set_from(body) {
            Ok(()) => StatusCode::OK,
            Err(_) => StatusCode::BAD_REQUEST,
        });

    let routes = ready.or(live).or(filter).with(warp::log("http"));

    warp::serve(routes)
        .bind_with_graceful_shutdown(([127, 0, 0, 1], 3000), async move {
            let release = shutdown.signaled().await;
            tracing::debug!("Shutdown");
            drop(release)
        })
        .1
}
