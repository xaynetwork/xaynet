use crate::{crypto::ByteObject, service::Handle, ParticipantPublicKey};
use bytes::{Buf, Bytes};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use warp::{
    http::{Response, StatusCode},
    Filter,
};

#[rustfmt::skip]
pub async fn serve(addr: impl Into<SocketAddr> + 'static, handle: Handle) {

    let message = warp::path!("message")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(with_hdl(handle.clone()))
        .and_then(handle_message);

    let sum_dict = warp::path!("sums")
        .and(warp::get())
        .and(with_hdl(handle.clone()))
        .and_then(handle_sums);

    let seed_dict = warp::path!("seeds")
        .and(warp::get())
        .and(part_pk())
        .and(with_hdl(handle.clone()))
        .and_then(handle_seeds)
        .recover(handle_reject);

    let round_params = warp::path!("params")
        .and(warp::get())
        .and(with_hdl(handle))
        .and_then(handle_params);

    let routes = message
        .or(sum_dict)
        .or(round_params)
        .or(seed_dict);

    warp::serve(routes).run(addr).await
}

async fn handle_message(body: Bytes, handle: Handle) -> Result<impl warp::Reply, Infallible> {
    handle.send_message(body.to_vec()).await;
    Ok(warp::reply())
}

async fn handle_sums(handle: Handle) -> Result<impl warp::Reply, Infallible> {
    let sums = handle.get_sum_dict().await;
    Ok(build_response(sums))
}

async fn handle_seeds(
    pk: ParticipantPublicKey,
    handle: Handle,
) -> Result<impl warp::Reply, Infallible> {
    let seeds = handle.get_seed_dict(pk).await;
    Ok(build_response(seeds))
}

async fn handle_params(handle: Handle) -> Result<impl warp::Reply, Infallible> {
    let params = handle.get_round_parameters().await;
    Ok(build_response(params))
}

fn with_hdl(hdl: Handle) -> impl Filter<Extract = (Handle,), Error = Infallible> + Clone {
    warp::any().map(move || hdl.clone())
}

fn build_response(data: Option<Arc<Vec<u8>>>) -> Response<Vec<u8>> {
    let builder = Response::builder();
    match data {
        None => {
            builder
                .status(StatusCode::NO_CONTENT) // 204
                .body(Vec::with_capacity(0)) // empty body; won't allocate
                .unwrap()
        }
        Some(arc_vec) => {
            let vec = (*arc_vec).clone(); // need inner value for warp::Reply
            builder
                .header("Content-Type", "application/octet-stream")
                .body(vec)
                .unwrap()
        }
    }
}

fn part_pk() -> impl Filter<Extract = (ParticipantPublicKey,), Error = warp::Rejection> + Clone {
    warp::body::bytes().and_then(|body: Bytes| async move {
        if let Some(pk) = ParticipantPublicKey::from_slice(body.bytes()) {
            Ok(pk)
        } else {
            Err(warp::reject::custom(InvalidPublicKey))
        }
    })
}

#[derive(Debug)]
struct InvalidPublicKey;

impl warp::reject::Reject for InvalidPublicKey {}

async fn handle_reject(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    let code;
    if err.is_not_found() {
        code = StatusCode::NOT_FOUND
    } else if let Some(InvalidPublicKey) = err.find() {
        code = StatusCode::BAD_REQUEST
    } else {
        error!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR
    };
    // reply with empty body; the status code is the interesting part
    Ok(warp::reply::with_status(Vec::with_capacity(0), code))
}
