use std::{
    pin::Pin,
    task::{Context, Poll},
};

use derive_more::From;
use futures::Stream;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Error)]
#[error("the RequestSender cannot be used because the state machine shut down")]
pub struct StateMachineShutdown;

use crate::{
    mask::object::MaskObject,
    LocalSeedDict,
    ParticipantPublicKey,
    PetError as Error,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

pub struct SumRequest {
    pub participant_pk: SumParticipantPublicKey,
    pub ephm_pk: SumParticipantEphemeralPublicKey,
}

pub struct UpdateRequest {
    pub participant_pk: UpdateParticipantPublicKey,
    pub local_seed_dict: LocalSeedDict,
    pub masked_model: MaskObject,
}

pub struct Sum2Request {
    pub participant_pk: ParticipantPublicKey,
    pub mask: MaskObject,
}

pub type SumResponse = Result<(), Error>;
pub type UpdateResponse = Result<(), Error>;
pub type Sum2Response = Result<(), Error>;

pub enum Request {
    Sum((SumRequest, oneshot::Sender<SumResponse>)),
    Update((UpdateRequest, oneshot::Sender<UpdateResponse>)),
    Sum2((Sum2Request, oneshot::Sender<Sum2Response>)),
}

/// A handle to send requests to the state machine
#[derive(From)]
pub struct RequestSender<R>(mpsc::UnboundedSender<R>);

impl<R> Clone for RequestSender<R> {
    fn clone(&self) -> Self {
        RequestSender(self.0.clone())
    }
}

impl<R> RequestSender<R> {
    pub fn send(&self, req: R) -> Result<(), StateMachineShutdown> {
        self.0.send(req).map_err(|_| StateMachineShutdown)
    }
}

#[derive(From)]
pub struct RequestReceiver<R>(mpsc::UnboundedReceiver<R>);

impl<R> Stream for RequestReceiver<R> {
    type Item = R;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("RequestReceiver: polling");
        Pin::new(&mut self.get_mut().0).poll_next(cx)
    }
}

impl<R> RequestReceiver<R> {
    pub fn new() -> (Self, RequestSender<R>) {
        let (tx, rx) = mpsc::unbounded_channel::<R>();
        let receiver = RequestReceiver::from(rx);
        let handle = RequestSender::from(tx);
        (receiver, handle)
    }

    pub fn close(&mut self) {
        self.0.close()
    }

    pub async fn recv(&mut self) -> Option<R> {
        self.0.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drop<T>(_t: T) {}

    #[tokio::test]
    async fn test_channel() {
        let (mut recv, snd) = RequestReceiver::<()>::new();
        snd.send(()).unwrap();
        recv.recv().await.unwrap();
        drop(snd);
        assert!(recv.recv().await.is_none());
    }
}
