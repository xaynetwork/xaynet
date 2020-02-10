use super::client::ClientId;
use super::request::{response_channel, Request};
use super::request::{
    EndTrainingResponse, HeartBeatResponse, RendezVousRequest, RendezVousResponse, RequestError,
    StartTrainingResponse,
};
use std::clone::Clone;
use tokio::sync::mpsc;

pub struct CoordinatorHandle<T>(mpsc::Sender<Request<T>>);

impl<T> Clone for CoordinatorHandle<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

type Result<T> = ::std::result::Result<T, RequestError>;

impl<T> CoordinatorHandle<T> {
    pub fn new(requests_tx: mpsc::Sender<Request<T>>) -> Self {
        Self(requests_tx)
    }

    pub async fn rendez_vous(&mut self) -> Result<RendezVousResponse> {
        let (resp_tx, resp_rx) = response_channel::<RendezVousResponse>();
        let req: Request<T> = Request::RendezVous((RendezVousRequest, resp_tx));
        self.0.send(req).await.map_err(|_| RequestError)?;
        resp_rx.await
    }

    pub async fn heartbeat(&mut self, id: ClientId) -> Result<HeartBeatResponse> {
        let (resp_tx, resp_rx) = response_channel::<HeartBeatResponse>();
        let req: Request<T> = Request::HeartBeat((id, resp_tx));
        self.0.send(req).await.map_err(|_| RequestError)?;
        resp_rx.await
    }

    pub async fn start_training(&mut self, id: ClientId) -> Result<StartTrainingResponse<T>> {
        let (resp_tx, resp_rx) = response_channel::<StartTrainingResponse<T>>();
        let req: Request<T> = Request::StartTraining((id, resp_tx));
        self.0.send(req).await.map_err(|_| RequestError)?;
        resp_rx.await
    }

    pub async fn end_training(&mut self, id: ClientId, weights: T) -> Result<EndTrainingResponse> {
        let (resp_tx, resp_rx) = response_channel::<EndTrainingResponse>();
        let req: Request<T> = Request::EndTraining(((id, weights), resp_tx));
        self.0.send(req).await.map_err(|_| RequestError)?;
        resp_rx.await
    }
}
