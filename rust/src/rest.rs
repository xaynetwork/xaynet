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
    let _route = warp::path!("hello" / String)
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

    let seed_dict = warp::path!("seeds")
        .and(warp::get())
//      .and(warp::body::bytes())
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

fn build_response(data: Option<Arc<Vec<u8>>>) -> Response<Vec<u8>> {
    let builder = Response::builder();
    match data {
        None => {
            builder
                .status(StatusCode::NO_CONTENT) // 204
                .body(Vec::with_capacity(0))    // empty body; won't allocate
                .unwrap()
        },
        Some(arc_vec) => {
            let vec = (*arc_vec).clone(); // need inner value for warp::Reply
            builder
                .header("Content-Type", "application/octet-stream")
                .body(vec)
                .unwrap()
        },
    }
}

async fn handle_seeds(pk: ParticipantPublicKey, handle: Handle) -> Result<impl warp::Reply, Infallible> {
    let seeds = handle.get_seed_dict(pk).await;
    Ok(build_response(seeds))
}

async fn handle_params(handle: Handle) -> Result<impl warp::Reply, Infallible> {
    let params = handle.get_round_parameters().await;
    Ok(build_response(params))
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

fn with_hdl(hdl: Handle) -> impl Filter<Extract = (Handle,), Error = Infallible> + Clone {
    warp::any().map(move || hdl.clone())
}

// TODO prob want Rejection rather than Infallible since body may not be pk
// async fn handle_seeds(body: Bytes, handle: Handle) -> Result<impl warp::Reply, Rejection> {
//     // TODO check key higher up and reject if cannot parse
//     let pk = ParticipantPublicKey::from_slice(body.bytes()).ok_or(warp::reject::reject())?;
//     let seed_dict: Arc<_> = handle.get_seed_dict(pk).await.ok_or(warp::reject::not_found())?;
//     let seed_dict_bytes = Arc::try_unwrap(seed_dict).unwrap();
//     Ok(warp::reply::with_header(seed_dict_bytes, "Content-Type", "application/octet-stream"))
// }

// async fn handle_params(handle: Handle) -> Result<impl warp::Reply, Rejection> {
//     let round_params = handle.get_round_parameters().await.ok_or(warp::reject::not_found())?;
//     let round_params_bytes = Arc::try_unwrap(round_params).unwrap();
//     Ok(warp::reply::with_header(round_params_bytes, "Content-Type", "application/octet-stream"))
// }

// async fn handle_sums(handle: Handle) -> Result<impl warp::Reply, Rejection> {
//     let sum_dict = handle
//         .get_sum_dict()
//         .await
//         .ok_or(warp::reject::not_found())?;
//     let sum_dict_bytes = Arc::try_unwrap(sum_dict).unwrap(); // HACK
//     Ok(warp::reply::with_header(sum_dict_bytes, "Content-Type", "application/octet-stream"))
// }
