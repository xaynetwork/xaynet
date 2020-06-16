use crate::service::Handle;
use crate::ParticipantPublicKey;
use crate::crypto::ByteObject;
use std::net::SocketAddr;
use std::sync::Arc;
use bytes::Bytes;
use bytes::Buf;
use warp::Filter;
use warp::reject::Rejection;
use std::convert::Infallible;
use warp::http::{StatusCode, Response};

async fn handle_whatever(name: String) -> Result<impl warp::Reply, Rejection> {
    match name.len() {
        2 => Ok(warp::reply()),
        _default => Err(warp::reject::not_found()),
    }

}

pub async fn serve(addr: impl Into<SocketAddr> + 'static, handle: Handle) {
    let route = warp::path!("hello" / String)
//      .map(|name| format!("Hello, {}!", name));
        .and_then(handle_whatever);

    let message = warp::path!("message")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(with_hdl(handle.clone()))
        .and_then(handle_message);

    let sum_dict = warp::path!("sums")
        .and(warp::get())
        .and(with_hdl(handle.clone()))
        .and_then(handle_sums);

    // can't take pk as param in the path, as it doesn't impl FromStr...
    let seed_dict = warp::path!("seeds")
        .and(warp::get())
        // ... so let's assume that it's in the request body
        // it's not good HTTP 1.1 practice but what can you do
        .and(warp::body::bytes())
        .and(with_hdl(handle.clone()))
        .and_then(handle_seeds);

    let round_params = warp::path!("params")
        .and(warp::get())
        .and(with_hdl(handle))
        .and_then(handle_params);

    let _routes = message
        .or(sum_dict)
        .or(seed_dict)
        .or(round_params);

    warp::serve(route).run(addr).await
}

async fn handle_message(body: Bytes, handle: Handle) -> Result<impl warp::Reply, Infallible> {
    handle.send_message(body.to_vec()).await;
    Ok(warp::reply())
}

async fn handle_sums(handle: Handle) -> Result<impl warp::Reply, Rejection> {
    let sum_dict = handle
        .get_sum_dict()
        .await
        .ok_or(warp::reject::not_found())?;
    let sum_dict_bytes = Arc::try_unwrap(sum_dict).unwrap(); // HACK
    Ok(warp::reply::with_header(sum_dict_bytes, "Content-Type", "application/octet-stream"))
}

async fn _handle_sums(handle: Handle) -> Result<impl warp::Reply, Infallible> {
    // we'll use a response Builder - the warp::reply convenience functions not
    // so convenient here as the status / header will vary depending on the
    // result of get_sum_dict()
    let builder = Response::builder();
    let response = match handle.get_sum_dict().await {
        None => {
            builder
                .status(StatusCode::NO_CONTENT) // 204
                .body(Vec::with_capacity(0))    // empty body; won't allocate
                .unwrap()
        },
        Some(arc_vec) => {
            // need inner value of Arc for warp::Reply
            let vec = (*arc_vec).clone();
            builder
                .header("Content-Type", "application/octet-stream")
                .body(vec)
                .unwrap()
        },
    };
    Ok(response)
}

// TODO prob want Rejection rather than Infallible since body may not be pk
async fn handle_seeds(body: Bytes, handle: Handle) -> Result<impl warp::Reply, Rejection> {
    // TODO check key higher up and reject if cannot parse
    let pk = ParticipantPublicKey::from_slice(body.bytes()).ok_or(warp::reject::reject())?;
    let seed_dict: Arc<_> = handle.get_seed_dict(pk).await.ok_or(warp::reject::not_found())?;
    let seed_dict_bytes = Arc::try_unwrap(seed_dict).unwrap();
    Ok(warp::reply::with_header(seed_dict_bytes, "Content-Type", "application/octet-stream"))
}

async fn handle_params(handle: Handle) -> Result<impl warp::Reply, Rejection> {
    let round_params = handle.get_round_parameters().await.ok_or(warp::reject::not_found())?;
    let round_params_bytes = Arc::try_unwrap(round_params).unwrap();
    Ok(warp::reply::with_header(round_params_bytes, "Content-Type", "application/octet-stream"))
}

fn with_hdl(hdl: Handle) -> impl Filter<Extract = (Handle,), Error = Infallible> + Clone {
    warp::any().map(move || hdl.clone())
}
