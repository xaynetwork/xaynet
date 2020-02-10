use super::client::ClientId;
use super::protocol;
use derive_more::Display;
use tokio::sync::oneshot;

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// Error returned when a request fails due to the coordinator having shut down.
#[derive(Debug, Display)]
pub struct RequestError;

impl ::std::error::Error for RequestError {}
pub struct ResponseReceiver<R>(oneshot::Receiver<R>);

pub fn response_channel<R>() -> (ResponseSender<R>, ResponseReceiver<R>) {
    let (tx, rx) = oneshot::channel::<R>();
    (ResponseSender(tx), ResponseReceiver(rx))
}

impl<R> Future for ResponseReceiver<R> {
    type Output = Result<R, RequestError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0)
            .as_mut()
            .poll(cx)
            .map_err(|_| RequestError)
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

// rendez-vous
#[derive(Debug)]
pub struct RendezVousRequest;
#[derive(Debug)]
pub enum RendezVousResponse {
    Accept(ClientId),
    Reject,
}

// heartbeat
pub type HeartBeatRequest = ClientId;
pub use protocol::HeartBeatResponse;

// start training
pub type StartTrainingRequest = ClientId;
pub enum StartTrainingResponse<T> {
    Accept(StartTrainingPayload<T>),
    Reject,
}

pub struct StartTrainingPayload<T> {
    pub global_weights: T,
    // more stuff...
}

impl<T> StartTrainingPayload<T> {
    pub fn new(global_weights: T) -> Self {
        Self { global_weights }
    }
}

impl<T> From<StartTrainingPayload<T>> for StartTrainingResponse<T> {
    fn from(value: StartTrainingPayload<T>) -> Self {
        Self::Accept(value)
    }
}

// end training
pub type EndTrainingRequest<T> = (ClientId, T);
pub use protocol::EndTrainingResponse;

pub enum Request<T> {
    RendezVous(RequestMessage<RendezVousRequest, RendezVousResponse>),
    HeartBeat(RequestMessage<HeartBeatRequest, HeartBeatResponse>),
    StartTraining(RequestMessage<StartTrainingRequest, StartTrainingResponse<T>>),
    EndTraining(RequestMessage<EndTrainingRequest<T>, EndTrainingResponse>),
}
