use super::client::ClientId;
use super::request::{response_channel, Request};
use super::request::{
    EndTrainingResponse, HeartBeatResponse, RendezVousResponse, StartTrainingResponse,
};
use tokio::sync::mpsc;
#[derive(Clone)]
pub struct CoordinatorHandle<T>(mpsc::Sender<Request<T>>);

impl<T> CoordinatorHandle<T> {
    pub fn new(requests_tx: mpsc::Sender<Request<T>>) -> Self {
        Self(requests_tx)
    }
    pub async fn rendez_vous(&mut self, id: Option<ClientId>) -> Result<RendezVousResponse, ()> {
        let (response_tx, response_rx) = response_channel::<RendezVousResponse>();
        let req: Request<T> = Request::RendezVous((id, response_tx));
        self.0.send(req).await.map_err(|_| ())?;
        response_rx.await
    }

    pub async fn heartbeat(&mut self, id: ClientId) -> Result<HeartBeatResponse, ()> {
        let (response_tx, response_rx) = response_channel::<HeartBeatResponse>();
        let req: Request<T> = Request::HeartBeat((id, response_tx));
        self.0.send(req).await.map_err(|_| ())?;
        response_rx.await
    }

    pub async fn start_training(&mut self, id: ClientId) -> Result<StartTrainingResponse<T>, ()> {
        let (response_tx, response_rx) = response_channel::<StartTrainingResponse<T>>();
        let req: Request<T> = Request::StartTraining((id, response_tx));
        self.0.send(req).await.map_err(|_| ())?;
        response_rx.await
    }

    pub async fn end_training(&mut self, id: ClientId) -> Result<EndTrainingResponse, ()> {
        let (response_tx, response_rx) = response_channel::<EndTrainingResponse>();
        let req: Request<T> = Request::EndTraining((id, response_tx));
        self.0.send(req).await.map_err(|_| ())?;
        response_rx.await
    }
}
