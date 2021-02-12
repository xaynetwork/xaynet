//! This module provides the the `StateMachine`, `Request`, `RequestSender` and `RequestReceiver`
//! types.

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use derive_more::From;
use displaydoc::Display;
use futures::{future::FutureExt, Stream};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tracing::{trace, Span};

use crate::storage::{LocalSeedDictAddError, MaskScoreIncrError, StorageError, SumPartAddError};
use xaynet_core::{
    mask::MaskObject,
    message::{Message, Payload, Update},
    LocalSeedDict,
    ParticipantPublicKey,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

/// Errors which can occur while the state machine handles a request.
#[derive(Debug, Display, Error)]
pub enum RequestError {
    /// The message was rejected.
    MessageRejected,
    /// The message was discarded.
    MessageDiscarded,
    /// Invalid update: the model or scalar sent by the participant could not be aggregated.
    AggregationFailed,
    /// The request could not be processed due to an internal error: {0}.
    InternalError(&'static str),
    /// Storage request failed: {0}.
    CoordinatorStorage(#[from] StorageError),
    /// Adding a local seed dict to the seed dictionary failed: {0}.
    LocalSeedDictAdd(#[from] LocalSeedDictAddError),
    /// Adding a sum participant to the sum dictionary failed: {0}.
    SumPartAdd(#[from] SumPartAddError),
    /// Incrementing a mask score failed: {0}.
    MaskScoreIncr(#[from] MaskScoreIncrError),
}

/// A sum request.
#[derive(Debug)]
pub struct SumRequest {
    /// The public key of the participant.
    pub participant_pk: SumParticipantPublicKey,
    /// The ephemeral public key of the participant.
    pub ephm_pk: SumParticipantEphemeralPublicKey,
}

/// An update request.
#[derive(Debug)]
pub struct UpdateRequest {
    /// The public key of the participant.
    pub participant_pk: UpdateParticipantPublicKey,
    /// The local seed dict that contains the seed used to mask `masked_model`.
    pub local_seed_dict: LocalSeedDict,
    /// The masked model trained by the participant.
    pub masked_model: MaskObject,
}

/// A sum2 request.
#[derive(Debug)]
pub struct Sum2Request {
    /// The public key of the participant.
    pub participant_pk: ParticipantPublicKey,
    /// The model mask computed by the participant.
    pub model_mask: MaskObject,
}

/// A [`StateMachine`] request.
///
/// [`StateMachine`]: crate::state_machine
#[derive(Debug, From)]
pub enum StateMachineRequest {
    Sum(SumRequest),
    Update(UpdateRequest),
    Sum2(Sum2Request),
}

impl From<Message> for StateMachineRequest {
    fn from(message: Message) -> Self {
        let participant_pk = message.participant_pk;
        match message.payload {
            Payload::Sum(sum) => StateMachineRequest::Sum(SumRequest {
                participant_pk,
                ephm_pk: sum.ephm_pk,
            }),
            Payload::Update(update) => {
                let Update {
                    local_seed_dict,
                    masked_model,
                    ..
                } = update;
                StateMachineRequest::Update(UpdateRequest {
                    participant_pk,
                    local_seed_dict,
                    masked_model,
                })
            }
            Payload::Sum2(sum2) => StateMachineRequest::Sum2(Sum2Request {
                participant_pk,
                model_mask: sum2.model_mask,
            }),
            Payload::Chunk(_) => unimplemented!(),
        }
    }
}

/// A handle to send requests to the [`StateMachine`].
///
/// [`StateMachine`]: crate::state_machine
#[derive(Clone, From, Debug)]
pub struct RequestSender(mpsc::UnboundedSender<(StateMachineRequest, Span, ResponseSender)>);

impl RequestSender {
    /// Sends a request to the [`StateMachine`].
    ///
    /// # Errors
    /// Fails if the [`StateMachine`] has already shut down and the `Request` channel has been
    /// closed as a result.
    ///
    /// [`StateMachine`]: crate::state_machine
    pub async fn request(&self, req: StateMachineRequest, span: Span) -> Result<(), RequestError> {
        let (resp_tx, resp_rx) = oneshot::channel::<Result<(), RequestError>>();
        self.0.send((req, span, resp_tx)).map_err(|_| {
            RequestError::InternalError(
                "failed to send request to the state machine: state machine is shutting down",
            )
        })?;
        resp_rx.await.map_err(|_| {
            RequestError::InternalError("failed to receive response from the state machine")
        })?
    }

    #[cfg(test)]
    pub fn is_closed(&self) -> bool {
        self.0.is_closed()
    }
}

/// A channel for sending the state machine to send the response to a
/// [`StateMachineRequest`].
pub(in crate::state_machine) type ResponseSender = oneshot::Sender<Result<(), RequestError>>;

/// The receiver half of the `Request` channel that is used by the [`StateMachine`] to receive
/// requests.
///
/// [`StateMachine`]: crate::state_machine
#[derive(From, Debug)]
pub struct RequestReceiver(mpsc::UnboundedReceiver<(StateMachineRequest, Span, ResponseSender)>);

impl Stream for RequestReceiver {
    type Item = (StateMachineRequest, Span, ResponseSender);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("RequestReceiver: polling");
        Pin::new(&mut self.get_mut().0).poll_recv(cx)
    }
}

impl RequestReceiver {
    /// Creates a new `Request` channel and returns the [`RequestReceiver`] as well as the
    /// [`RequestSender`] half.
    pub fn new() -> (Self, RequestSender) {
        let (tx, rx) = mpsc::unbounded_channel::<(StateMachineRequest, Span, ResponseSender)>();
        let receiver = RequestReceiver::from(rx);
        let handle = RequestSender::from(tx);
        (receiver, handle)
    }

    /// Closes the `Request` channel.
    /// See [the `tokio` documentation][close] for more information.
    ///
    /// [close]: https://docs.rs/tokio/1.1.0/tokio/sync/mpsc/struct.UnboundedReceiver.html#method.close
    pub fn close(&mut self) {
        self.0.close()
    }

    /// Receives the next request.
    /// See [the `tokio` documentation][receive] for more information.
    ///
    /// [receive]: https://docs.rs/tokio/1.1.0/tokio/sync/mpsc/struct.UnboundedReceiver.html#method.recv
    pub async fn recv(&mut self) -> Option<(StateMachineRequest, Span, ResponseSender)> {
        self.0.recv().await
    }

    /// Try to retrieve the next request without blocking
    pub fn try_recv(&mut self) -> Option<Option<(StateMachineRequest, Span, ResponseSender)>> {
        // Note `try_recv` (tokio 0.2.x) or `recv().now_or_never()` (tokio 1.x)
        // has an implementation bug where previously sent messages may not be
        // available immediately.
        // Related issue: https://github.com/tokio-rs/tokio/issues/3350
        // At the moment it behaves like `try_recv`, but we should check if this
        // bug is a problem for us. But first we should replace the unbounded channel canal with
        // a bounded channel (XN-1162)
        self.0.recv().now_or_never()
    }
}
