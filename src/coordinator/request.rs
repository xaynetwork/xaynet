use super::client::ClientId;
use super::state_machine::*;
use tokio::sync::oneshot;

pub struct ResponseSender<R>(oneshot::Sender<R>);

impl<R> ResponseSender<R> {
    fn send(self, response: R) {
        self.0.send(response).unwrap_or_else(|_| {
            warn!("failed to send response: receiver shut down");
        })
    }
}

pub type RendezVousRequest = (Option<ClientId>, ResponseSender<RendezVousResponse>);
pub type HeartbeatRequest = (ClientId, ResponseSender<HeartBeatResponse>);

pub enum Request {
    RendezVous(RendezVousRequest),
    Heartbeat(HeartbeatRequest),
}
