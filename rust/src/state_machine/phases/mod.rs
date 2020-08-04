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
        requests::{RequestReceiver, ResponseSender, StateMachineRequest},
        StateMachine,
    },
    utils::Request,
    PetError,
};

use futures::StreamExt;
use tracing_futures::Instrument;

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
pub trait Phase {
    /// Name of the current phase
    const NAME: PhaseName;

    /// Run this phase to completion
    async fn run(&mut self) -> Result<(), StateError>;

    /// Moves from this state to the next state.
    fn next(self) -> Option<StateMachine>;
}

/// A trait that must be implemented by a state to handle a request.
pub trait Handler {
    /// Handles a request.
    fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), PetError>;
}

/// The state corresponding to a phase of the PET protocol.
///
/// This contains the state-dependent `inner` state and the state-independent `coordinator_state`
/// which is shared across state transitions.
pub struct PhaseState<S> {
    /// The inner state.
    pub(in crate::state_machine) inner: S,
    /// The Coordinator state.
    pub(in crate::state_machine) coordinator_state: CoordinatorState,
    /// The request receiver half.
    pub(in crate::state_machine) request_rx: RequestReceiver,
}

impl<S> PhaseState<S>
where
    Self: Handler + Phase,
{
    /// Processes requests for as long as the given duration.
    async fn process_during(&mut self, dur: tokio::time::Duration) -> Result<(), StateError> {
        tokio::select! {
            err = self.process_loop() => {
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
    async fn process_loop(&mut self) -> Result<(), StateError> {
        loop {
            self.process_single().await?;
        }
    }

    /// Processes the next available request.
    async fn process_single(&mut self) -> Result<(), StateError> {
        let (req, resp_tx) = self.next_request().await?;
        let span = req.span();
        let _span_guard = span.enter();
        let res = self.handle_request(req.into_inner());
        // This may error out if the receiver has already be dropped but
        // it doesn't matter for us.
        let _ = resp_tx.send(res.map_err(Into::into));
        Ok(())
    }
}

impl<S> PhaseState<S>
where
    Self: Phase,
{
    /// Run the current phase to completion, then transition to the
    /// next phase and return it.
    pub async fn run_phase(mut self) -> Option<StateMachine> {
        let phase = <Self as Phase>::NAME;
        let span = error_span!("run_phase", phase = ?phase);

        async move {
            info!("starting phase");
            info!("broadcasting phase event");
            self.coordinator_state.events.broadcast_phase(
                phase,
            );

            if let Err(err) = self.run().await {
                warn!("phase failed: {:?}", err);
                return Some(self.into_error_state(err));
            }

            info!("phase ran succesfully");

            debug!("purging outdated requests before transitioning");
            if let Err(err) = self.purge_outdated_requests() {
                warn!("failed to purge outdated requests");
                // If we're already in the error state or shutdown state,
                // ignore this error
                match phase {
                    PhaseName::Error | PhaseName::Shutdown => {
                        debug!("already in error/shutdown state: ignoring error while purging outdated requests");
                    }
                    _ => return Some(self.into_error_state(err)),
                }
            }

            info!("transitioning to the next phase");
            self.next()
        }.instrument(span).await
    }

    /// Process all the pending requests that are now considered
    /// outdated. This happens at the end of each phase, before
    /// transitioning to the next phase.
    fn purge_outdated_requests(&mut self) -> Result<(), StateError> {
        loop {
            match self.try_next_request()? {
                Some((req, resp_tx)) => {
                    let span = req.span();
                    let _span_guard = span.enter();
                    info!("rejecting request");
                    let _ = resp_tx.send(Err(PetError::InvalidMessage.into()));
                }
                None => return Ok(()),
            }
        }
    }
}

// Functions that are available to all states
impl<S> PhaseState<S> {
    /// Receives the next [`Request`].
    ///
    /// # Errors
    /// Returns [`StateError::ChannelError`] when all sender halves have been dropped.
    async fn next_request(
        &mut self,
    ) -> Result<(Request<StateMachineRequest>, ResponseSender), StateError> {
        debug!("waiting for the next incoming request");
        self.request_rx.next().await.ok_or_else(|| {
            error!("request receiver broken: senders have been dropped");
            StateError::ChannelError("all message senders have been dropped!")
        })
    }

    fn try_next_request(
        &mut self,
    ) -> Result<Option<(Request<StateMachineRequest>, ResponseSender)>, StateError> {
        match self.request_rx.try_recv() {
            Ok(item) => Ok(Some(item)),
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

    fn into_error_state(self, err: StateError) -> StateMachine {
        PhaseState::<StateError>::new(self.coordinator_state, self.request_rx, err).into()
    }
}
