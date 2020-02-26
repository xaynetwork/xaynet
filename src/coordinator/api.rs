use crate::{common::ClientId, coordinator::core::CoordinatorHandle};

use tokio::net::TcpListener;
use warp::Filter;

pub mod models {
    use crate::common::{ClientId, Token};
    /// Response to a heartbeat
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum HeartBeatResponse {
        /// The client should stand by in its current state
        StandBy,

        /// The coordinator has finished, and the client should disconnect
        Finish,

        /// The client has been selected for the given round and should
        /// start or continue training
        Round(u32),

        /// The client has not been accepted by the coordinator yet and
        /// should not send heartbeats
        Reject,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum RendezVousResponse {
        Accept(ClientId),
        Reject,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum StartTrainingResponse {
        Accept(String, Token),
        Reject,
    }
}

pub async fn serve(bind_address: &str, handle: CoordinatorHandle) {
    let handle = warp::any().map(move || handle.clone());

    let heartbeat = warp::path!("heartbeat" / ClientId)
        .and(warp::get())
        .and(handle.clone())
        .and_then(|id, mut handle: CoordinatorHandle| async move {
            match handle.heartbeat(id).await {
                Ok(response) => Ok(warp::reply::json(&response)),
                Err(_) => Err(warp::reject::not_found()),
            }
        });

    let rendez_vous = warp::path!("rendez_vous")
        .and(warp::get())
        .and(handle.clone())
        .and_then(|mut handle: CoordinatorHandle| async move {
            match handle.rendez_vous().await {
                Ok(response) => Ok(warp::reply::json(&response)),
                Err(_) => Err(warp::reject::not_found()),
            }
        });

    let start_training = warp::path!("start_training" / ClientId)
        .and(warp::get())
        .and(handle.clone())
        .and_then(|id, mut handle: CoordinatorHandle| async move {
            match handle.start_training(id).await {
                Ok(response) => Ok(warp::reply::json(&response)),
                Err(_) => Err(warp::reject::not_found()),
            }
        });

    let mut listener = TcpListener::bind(bind_address).await.unwrap();

    info!("starting HTTP server on {}", bind_address);
    let log = warp::log("http");
    warp::serve(heartbeat.or(rendez_vous).or(start_training).with(log))
        .run_incoming(listener.incoming())
        .await
}
