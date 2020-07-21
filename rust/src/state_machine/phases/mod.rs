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

/// Name of the current phase
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PhaseName {
    Idle,
    Sum,
    Update,
    Sum2,
    Unmask,
    Error,
    Shutdown,
}

/// A trait that must be implemented by a state in order to move to a next state.
#[async_trait]
pub trait Phase<R> {
    /// Name of the current phase
    const NAME: PhaseName;

    /// Run this phase to completion
    async fn run(&mut self) -> Result<(), StateError>;

    /// Moves from this state to the next state.
    fn next(self) -> Option<StateMachine<R>>;
}

/// A trait that must be implemented by a state to handle a request.
pub trait Handler<R> {
    /// Handles a request.
    fn handle_request(&mut self, req: R);
}

/// When the state machine transitions to a new phase, all the pending
/// requests are considered outdated, and purged. The [`Purge`] trait
/// implements this behavior.
pub trait Purge<R> {
    /// Process an outdated request.
    fn handle_outdated_request(&mut self, req: R);
}

impl<R, S> Purge<Request> for PhaseState<R, S> {
    fn handle_outdated_request(&mut self, req: Request) {
        reject_request(req)
    }
}

impl<R, S> Purge<Traced<Request>> for PhaseState<R, S>
where
    Self: Purge<Request>,
{
    fn handle_outdated_request(&mut self, req: Traced<Request>) {
        let span = req.span().clone();
        let _enter = span.enter();
        <Self as Purge<Request>>::handle_outdated_request(self, req.into_inner())
    }
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
    Self: Handler<R> + Phase<R> + Purge<R>,
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
            self.fetch_exec().await?;
        }
    }

    /// Processes the next available request.
    async fn fetch_exec(&mut self) -> Result<(), StateError> {
        let req = self.next_request().await?;
        self.handle_request(req);
        Ok(())
    }
}

impl<R, S> PhaseState<R, S>
where
    Self: Phase<R> + Purge<R>,
{
    /// Run the current phase to completion, then transition to the
    /// next phase and return it.
    pub async fn run_phase(mut self) -> Option<StateMachine<R>> {
        if let Err(err) = self.run().await {
            return Some(self.into_error_state(err));
        }

        if let Err(err) = self.purge_outdated_requests() {
            // If we're already in the error state or shutdown state,
            // ignore this error
            match <Self as Phase<R>>::NAME {
                PhaseName::Error | PhaseName::Shutdown => {
                    debug!("already in error/shutdown state: ignoring error while purging outdated requests");
                }
                _ => return Some(self.into_error_state(err)),
            }
        }

        self.next()
    }

    /// Process all the pending requests that are now considered
    /// outdated. This happens at the end of each phase, before
    /// transitioning to the next phase.
    fn purge_outdated_requests(&mut self) -> Result<(), StateError> {
        loop {
            match self.try_next_request()? {
                Some(req) => self.handle_outdated_request(req),
                None => return Ok(()),
            }
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

    fn try_next_request(&mut self) -> Result<Option<R>, StateError> {
        match self.request_rx.try_recv() {
            Ok(req) => Ok(Some(req)),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                debug!("no pending request");
                Ok(None)
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Closed) => {
                warn!("failed to get next pending request: channel shut down");
                Err(StateError::ChannelError(
                    "all message senders have been dropped!",
                ))
            }
        }
    }

    fn into_error_state(self, err: StateError) -> StateMachine<R> {
        PhaseState::<R, StateError>::new(self.coordinator_state, self.request_rx, err).into()
    }
}

/// Respond to the given request with a rejection error.
pub fn reject_request(req: Request) {
    match req {
        Request::Sum((_, response_tx)) => send_rejection(response_tx),
        Request::Update((_, response_tx)) => send_rejection(response_tx),
        Request::Sum2((_, response_tx)) => send_rejection(response_tx),
    }
}

/// Send a rejection through the given channel
fn send_rejection(response_tx: oneshot::Sender<Result<(), PetError>>) {
    debug!("invalid message");
    // `send` returns an error if the receiver half has already
    // been dropped. This means that the receiver is not
    // interested in the response of the request. Therefore the
    // error is ignored.
    let _ = response_tx.send(Err(PetError::InvalidMessage));
}
