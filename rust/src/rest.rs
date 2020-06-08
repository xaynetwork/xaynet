use crate::service::Handle;
use crate::ParticipantPublicKey;
use crate::crypto::ByteObject;
use std::net::SocketAddr;
use std::sync::Arc;
use bytes::Bytes;
use bytes::Buf;
use warp::Filter;
use warp::http::{StatusCode, Response};
use warp::reject::Rejection;
use std::convert::Infallible;

pub async fn serve(addr: impl Into<SocketAddr> + 'static, handle: Handle) {
    let _route = warp::path!("hello" / String)
        .map(|name| format!("Hello, {}!", name));

    let message = warp::path!("message")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(with_hdl(handle.clone()))
        .and_then(send_message);

    let sum_dict = warp::path!("sums")
        .and(warp::get())
        .and(with_hdl(handle.clone()))
        .and_then(get_sums);

    // can't take pk as param in the path, as it doesn't impl FromStr...
    let seed_dict = warp::path!("seeds")
        .and(warp::get())
        // ... so let's assume that it's in the request body
        // it's not good HTTP 1.1 practice but what can you do
        .and(warp::body::bytes())
        .and(with_hdl(handle.clone()))
        .and_then(get_seeds);

    let round_params = warp::path!("params")
        .and(warp::get())
        .and(with_hdl(handle))
        .and_then(get_params);

    let routes = message
        .or(sum_dict)
        .or(seed_dict)
        .or(round_params);

    warp::serve(routes).run(addr).await
}

async fn send_message(body: Bytes, handle: Handle) -> Result<impl warp::Reply, Infallible> {
    handle.send_message(body.to_vec()).await;
    Ok(warp::reply())
}

async fn get_sums(handle: Handle) -> Result<impl warp::Reply, Rejection> {
    let sum_dict = handle
        .get_sum_dict()
        .await
        .ok_or(warp::reject::not_found())?;
    let sum_dict_bytes = Arc::try_unwrap(sum_dict).unwrap();
    Ok(warp::reply::with_header(sum_dict_bytes, "Content-Type", "application/octet-stream"))
}

// TODO prob want Rejection rather than Infallible since body may not be pk
async fn get_seeds(body: Bytes, handle: Handle) -> Result<impl warp::Reply, Infallible> {
    // TODO check key higher up and reject if cannot parse
    let pk = ParticipantPublicKey::from_slice(body.bytes()).unwrap();
    let seed_dict: Arc<_> = handle.get_seed_dict(pk).await.unwrap();
    let seed_dict_bytes = Arc::try_unwrap(seed_dict).unwrap();
    Ok(warp::reply::with_header(seed_dict_bytes, "Content-Type", "application/octet-stream"))
}

async fn get_params(handle: Handle) -> Result<impl warp::Reply, Infallible> {
    handle.get_round_parameters().await;
    Ok(warp::reply()) // TODO
}

fn with_hdl(hdl: Handle) -> impl Filter<Extract = (Handle,), Error = Infallible> + Clone {
    warp::any().map(move || hdl.clone())
}
