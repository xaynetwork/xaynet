//! This module provides the `PhaseStates` of the [`StateMachine`].

mod error;
mod idle;
mod shutdown;
mod sum;
mod sum2;
mod unmask;
mod update;

pub use self::{
    error::StateError,
    idle::Idle,
    shutdown::Shutdown,
    sum::Sum,
    sum2::Sum2,
    unmask::Unmask,
    update::Update,
};

use crate::{
    state_machine::{
        coordinator::CoordinatorState,
        requests::{Request, RequestReceiver},
        StateMachine,
    },
    utils::trace::{Traceable, Traced},
    PetError,
};

use futures::StreamExt;
use tokio::sync::oneshot;

/// A trait that must be implemented by a state in order to move to a next state.
#[async_trait]
pub trait Phase<R> {
    /// Moves from this state to the next state.
    async fn next(mut self) -> Option<StateMachine<R>>;
}

/// A trait that must be implemented by a state to handle a request.
pub trait Handler<R> {
    /// Handles a request.
    fn handle_request(&mut self, req: R);
}

impl<R, S> Handler<Traced<Request>> for PhaseState<R, S>
where
    Self: Handler<Request>,
{
    /// Handles a [`Request`].
    fn handle_request(&mut self, req: Traced<Request>) {
        let span = req.span().clone();
        let _enter = span.enter();
        <Self as Handler<Request>>::handle_request(self, req.into_inner())
    }
}

/// The state corresponding to a phase of the PET protocol.
///
/// This contains the state-dependent `inner` state and the state-independent `coordinator_state`
/// which is shared across state transitions.
pub struct PhaseState<R, S> {
    /// The inner state.
    pub(in crate::state_machine) inner: S,
    /// The Coordinator state.
    pub(in crate::state_machine) coordinator_state: CoordinatorState,
    /// The request receiver half.
    pub(in crate::state_machine) request_rx: RequestReceiver<R>,
}

impl<R, S> PhaseState<R, S>
where
    Self: Handler<R>,
{
    /// Processes requests for as long as the given duration.
    async fn process_during(&mut self, dur: tokio::time::Duration) -> Result<(), StateError> {
        tokio::select! {
            err = self.process() => {
                error!("processing loop terminated before duration elapsed");
                err
            }
            _ = tokio::time::delay_for(dur) => {
                debug!("duration elapsed");
                Ok(())
            }
        }
    }

    /// Processes requests indefinitely.
    async fn process(&mut self) -> Result<(), StateError> {
        loop {
            let req = self.next_request().await?;
            self.handle_request(req);
        }
    }
}

// Functions that are available to all states
impl<R, S> PhaseState<R, S> {
    /// Receives the next [`Request`].
    ///
    /// # Errors
    /// Returns [`StateError::ChannelError`] when all sender halves have been dropped.
    async fn next_request(&mut self) -> Result<R, StateError> {
        debug!("waiting for the next incoming request");
        self.request_rx.next().await.ok_or_else(|| {
            error!("request receiver broken: senders have been dropped");
            StateError::ChannelError("all message senders have been dropped!")
        })
    }

    /// Handles an invalid request by sending [`PetError::InvalidMessage`] to the request sender.
    fn handle_invalid_message(response_tx: oneshot::Sender<Result<(), PetError>>) {
        debug!("invalid message");
        // `send` returns an error if the receiver half has already been dropped. This means that
        // the receiver is not interested in the response of the request. Therefore the error is
        // ignored.
        let _ = response_tx.send(Err(PetError::InvalidMessage));
    }
}
