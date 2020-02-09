use super::request::{response_channel, Request};
use super::state_machine::{HeartBeatResponse, RendezVousResponse};
use super::client::ClientId;
use tokio::sync::mpsc;
#[derive(Clone)]
pub struct CoordinatorHandle(mpsc::Sender<Request>);

impl CoordinatorHandle {
    pub async fn rendez_vous(&self, id: Option<ClientId>) -> Result<RendezVousResponse, ()> {
        let (response_tx, response_rx) = response_channel::<RendezVousResponse>();
        let req = Request::RendezVous((id, response_tx));
        response_rx.await
    }

    pub async fn heartbeat(&self, id: ClientId) -> Result<HeartBeatResponse, ()> {
        let (response_tx, response_rx) = response_channel::<HeartBeatResponse>();
        let req = Request::HeartBeat((id, response_tx));
        response_rx.await
    }

}
