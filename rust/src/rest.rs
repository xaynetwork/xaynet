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

    let scalar = warp::path!("scalar")
        .and(warp::get())
        .and(with_hdl(handle.clone()))
        .and_then(handle_scalar);

    let seed_dict = warp::path!("seeds")
        .and(warp::get())
        .and(part_pk())
        .and(with_hdl(handle.clone()))
        .and_then(handle_seeds);

    let length = warp::path!("length")
        .and(warp::get())
        .and(with_hdl(handle.clone()))
        .and_then(handle_length);

    let round_params = warp::path!("params")
        .and(warp::get())
        .and(with_hdl(handle))
        .and_then(handle_params);

    let routes = message
        .or(round_params)
        .or(sum_dict)
        .or(scalar)
        .or(seed_dict)
        .or(length)
        .recover(handle_reject);

    warp::serve(routes).run(addr).await
}

async fn handle_message(body: Bytes, handle: Handle) -> Result<impl warp::Reply, Infallible> {
    handle.send_message(body.to_vec()).await;
    Ok(warp::reply())
}

async fn handle_sums(handle: Handle) -> Result<impl warp::Reply, Infallible> {
    let sums = handle.get_sum_dict().await;
    Ok(build_byte_response(sums))
}

async fn handle_scalar(handle: Handle) -> Result<impl warp::Reply, Infallible> {
    let scalar = handle.get_scalar().await.map(|s| s.to_string());
    Ok(build_text_response(scalar))
}

async fn handle_seeds(
    pk: ParticipantPublicKey,
    handle: Handle,
) -> Result<impl warp::Reply, Infallible> {
    let seeds = handle.get_seed_dict(pk).await;
    Ok(build_byte_response(seeds))
}

async fn handle_length(handle: Handle) -> Result<impl warp::Reply, Infallible> {
    let length = handle.get_length().await.map(|l| l.to_string());
    Ok(build_text_response(length))
}

async fn handle_params(handle: Handle) -> Result<impl warp::Reply, Infallible> {
    let params = handle.get_round_parameters().await;
    Ok(build_byte_response(params))
}

fn with_hdl(hdl: Handle) -> impl Filter<Extract = (Handle,), Error = Infallible> + Clone {
    warp::any().map(move || hdl.clone())
}

fn build_byte_response(data: Option<Arc<Vec<u8>>>) -> Response<Vec<u8>> {
    let builder = Response::builder();
    match data {
        None => {
            builder
                .status(StatusCode::NO_CONTENT) // 204
                .body(Vec::new()) // empty body; won't allocate
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

fn build_text_response(text: Option<String>) -> Response<String> {
    let builder = Response::builder();
    match text {
        None => {
            builder
                .status(StatusCode::NO_CONTENT) // 204
                .body(String::new()) // empty body; won't allocate
                .unwrap()
        }
        Some(string) => builder.body(string).unwrap(),
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
    let code = if err.is_not_found() {
        StatusCode::NOT_FOUND
    } else if let Some(InvalidPublicKey) = err.find() {
        StatusCode::BAD_REQUEST
    } else {
        error!("unhandled rejection: {:?}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    };
    // reply with empty body; the status code is the interesting part
    Ok(warp::reply::with_status(Vec::new(), code))
}
