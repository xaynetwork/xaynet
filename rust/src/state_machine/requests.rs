//! This module provides the the [`StateMachine`]'s `Request`, `RequestSender` and `RequestReceiver`
//! types.
//!
//! [`StateMachine`]: crate::state_machine::StateMachine
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use derive_more::From;
use futures::Stream;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

/// Error that occurs when a [`RequestSender`] tries to send a request on a closed `Request` channel.
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

/// A sum request.
pub struct SumRequest {
    /// The public key of the participant.
    pub participant_pk: SumParticipantPublicKey,
    /// The ephemeral public key of the participant.
    pub ephm_pk: SumParticipantEphemeralPublicKey,
}

/// An update request.
pub struct UpdateRequest {
    /// The public key of the participant.
    pub participant_pk: UpdateParticipantPublicKey,
    /// The local seed dict that contains the seed used to mask `masked_model`.
    pub local_seed_dict: LocalSeedDict,
    /// The masked model trained by the participant.
    pub masked_model: MaskObject,
}

/// A sum2 request.
pub struct Sum2Request {
    /// The public key of the participant.
    pub participant_pk: ParticipantPublicKey,
    /// The mask computed by the participant.
    pub mask: MaskObject,
}

/// A sum response.
pub type SumResponse = Result<(), Error>;
/// An update response.
pub type UpdateResponse = Result<(), Error>;
/// A sum2 response.
pub type Sum2Response = Result<(), Error>;

/// A [`StateMachine`] request.
///
/// [`StateMachine`]: crate::state_machine
pub enum Request {
    Sum((SumRequest, oneshot::Sender<SumResponse>)),
    Update((UpdateRequest, oneshot::Sender<UpdateResponse>)),
    Sum2((Sum2Request, oneshot::Sender<Sum2Response>)),
}

/// A handle to send requests to the [`StateMachine`].
///
/// [`StateMachine`]: crate::state_machine
#[derive(From)]
pub struct RequestSender<R>(mpsc::UnboundedSender<R>);

impl<R> Clone for RequestSender<R> {
    // Clones the sender half of the `Request` channel.
    fn clone(&self) -> Self {
        RequestSender(self.0.clone())
    }
}

impl<R> RequestSender<R> {
    /// Sends a request to the [`StateMachine`].
    ///
    /// # Errors
    /// Fails if the [`StateMachine`] has already shut down and the `Request` channel has been
    /// closed as a result.
    ///
    /// [`StateMachine`]: crate::state_machine
    pub fn send(&self, req: R) -> Result<(), StateMachineShutdown> {
        self.0.send(req).map_err(|_| StateMachineShutdown)
    }
}

/// The receiver half of the `Request` channel that is used by the [`StateMachine`] to receive
/// requests.
///
/// [`StateMachine`]: crate::state_machine
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
    /// Creates a new `Request` channel and returns the [`RequestReceiver`] as well as the
    /// [`RequestSender`] half.
    pub fn new() -> (Self, RequestSender<R>) {
        let (tx, rx) = mpsc::unbounded_channel::<R>();
        let receiver = RequestReceiver::from(rx);
        let handle = RequestSender::from(tx);
        (receiver, handle)
    }

    /// Closes the `Request` channel.
    /// See [the `tokio` documentation][close] for more information.
    ///
    /// [close]: https://docs.rs/tokio/0.2.21/tokio/sync/mpsc/struct.UnboundedReceiver.html#method.close
    pub fn close(&mut self) {
        self.0.close()
    }

    /// Receives the next request.
    /// See [the `tokio` documentation][receive] for more information.
    ///
    /// [receive]: https://docs.rs/tokio/0.2.21/tokio/sync/mpsc/struct.UnboundedReceiver.html#method.recv
    pub async fn recv(&mut self) -> Option<R> {
        self.0.recv().await
    }

    /// Try to retrieve the next request without blocked
    /// See [the `tokio` documentation][try_receive] for more information.
    ///
    /// [try_receive]: https://docs.rs/tokio/0.2.21/tokio/sync/mpsc/struct.UnboundedReceiver.html#method.try_recv
    pub fn try_recv(&mut self) -> Result<R, tokio::sync::mpsc::error::TryRecvError> {
        self.0.try_recv()
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
