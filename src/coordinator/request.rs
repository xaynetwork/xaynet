use super::client::ClientId;
use super::state_machine::*;
use tokio::sync::oneshot;

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub struct ResponseReceiver<R>(oneshot::Receiver<R>);

pub fn response_channel<R>() -> (ResponseSender<R>, ResponseReceiver<R>) {
    let (tx, rx) = oneshot::channel::<R>();
    (ResponseSender(tx), ResponseReceiver(rx))
}

impl<R> Future for ResponseReceiver<R> {
    type Output = Result<R, ()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0).as_mut().poll(cx).map_err(|_| ())
    }
}

pub struct ResponseSender<R>(oneshot::Sender<R>);

impl<R> ResponseSender<R> {
    pub fn send(self, response: R) {
        self.0.send(response).unwrap_or_else(|_| {
            warn!("failed to send response: receiver shut down");
        })
    }
}

pub type RequestMessage<P, R> = (P, ResponseSender<R>);

pub type RendezVousRequest = Option<ClientId>;
pub type HeartBeatRequest = ClientId;

pub enum Request {
    RendezVous(RequestMessage<RendezVousRequest, RendezVousResponse>),
    HeartBeat(RequestMessage<HeartBeatRequest, HeartBeatResponse>),
}
