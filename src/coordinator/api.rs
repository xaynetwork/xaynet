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

async fn serve(handle: CoordinatorHandle) {
    let handle = warp::any().map(move || handle.clone());

    let heartbeat = warp::get()
        .and(warp::path::param::<ClientId>())
        .and(handle.clone())
        .and_then(|id, mut handle: CoordinatorHandle| async move {
            match handle.heartbeat(id).await {
                Ok(response) => Ok(warp::reply::json(&response)),
                Err(_) => Err(warp::reject::not_found()),
            }
        });

    let rendez_vous =
        warp::get()
            .and(handle.clone())
            .and_then(|mut handle: CoordinatorHandle| async move {
                match handle.rendez_vous().await {
                    Ok(response) => Ok(warp::reply::json(&response)),
                    Err(_) => Err(warp::reject::not_found()),
                }
            });

    let start_training = warp::get()
        .and(warp::path::param::<ClientId>())
        .and(handle.clone())
        .and_then(|id, mut handle: CoordinatorHandle| async move {
            match handle.start_training(id).await {
                Ok(response) => Ok(warp::reply::json(&response)),
                Err(_) => Err(warp::reject::not_found()),
            }
        });

    let mut listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    warp::serve(heartbeat.or(rendez_vous).or(start_training))
        .run_incoming(listener.incoming())
        .await
}
