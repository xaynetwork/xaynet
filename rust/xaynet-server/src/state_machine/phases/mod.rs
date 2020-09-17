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
        events::EventPublisher,
        requests::{RequestReceiver, ResponseSender, StateMachineRequest},
        StateMachine,
        StateMachineError,
    },
    storage::redis,
};

#[cfg(feature = "metrics")]
use crate::{metrics, metrics::MetricsSender};

use futures::StreamExt;
use tracing::Span;
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
#[async_trait]
pub trait Handler {
    /// Handles a request.
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), StateMachineError>;
}

/// I/O interfaces.
#[cfg_attr(test, derive(Debug))]
pub struct IO {
    /// The request receiver half.
    pub(in crate::state_machine) request_rx: RequestReceiver,
    /// The event publisher.
    pub(in crate::state_machine) events: EventPublisher,
    /// Redis client.
    pub(in crate::state_machine) redis: redis::Client,
    #[cfg(feature = "metrics")]
    /// The metrics sender half.
    pub(in crate::state_machine) metrics_tx: MetricsSender,
}

/// A struct that contains the coordinator state and the I/O interfaces that is shared and
/// accessible by all `PhaseState`s.
#[cfg_attr(test, derive(Debug))]
pub struct Shared {
    /// The coordinator state.
    pub(in crate::state_machine) state: CoordinatorState,
    /// I/O interfaces.
    pub(in crate::state_machine) io: IO,
}

impl Shared {
    pub fn new(
        coordinator_state: CoordinatorState,
        publisher: EventPublisher,
        request_rx: RequestReceiver,
        redis: redis::Client,
        #[cfg(feature = "metrics")] metrics_tx: MetricsSender,
    ) -> Self {
        Self {
            state: coordinator_state,
            io: IO {
                request_rx,
                events: publisher,
                redis,
                #[cfg(feature = "metrics")]
                metrics_tx,
            },
        }
    }

    /// Set the round ID to the given value
    pub fn set_round_id(&mut self, id: u64) {
        self.state.round_id = id;
        self.io.events.set_round_id(id);
    }

    /// Return the current round ID
    pub fn round_id(&self) -> u64 {
        self.state.round_id
    }
}

/// The state corresponding to a phase of the PET protocol.
///
/// This contains the state-dependent `inner` state and the state-independent `shared.state`
/// which is shared across state transitions. Furthermore, `shared.io` contains the I/O interfaces
/// of the state machine.
pub struct PhaseState<S> {
    /// The inner state.
    pub(in crate::state_machine) inner: S,
    /// The shared coordinator state and I/O interfaces.
    pub(in crate::state_machine) shared: Shared,
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
        let (req, span, resp_tx) = self.next_request().await?;
        let _span_guard = span.enter();
        let res = self.handle_request(req).await;

        if let Err(ref err) = res {
            error!("failed to handle message: {:?}", err);
            metrics!(
                self.shared.io.metrics_tx,
                metrics::message::rejected::increment(self.shared.state.round_id, Self::NAME)
            );
        }

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
            self.shared.io.events.broadcast_phase(
                phase,
            );

            metrics!(self.shared.io.metrics_tx, metrics::phase::update(phase));

            if let Err(err) = self.run().await {
                warn!("phase failed: {:?}", err);
                return Some(self.into_error_state(err));
            }

            info!("phase ran successfully");

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
                Some((_req, span, resp_tx)) => {
                    let _span_guard = span.enter();
                    info!("rejecting request");
                    let _ = resp_tx.send(Err(StateMachineError::MessageRejected));

                    metrics!(
                        self.shared.io.metrics_tx,
                        metrics::message::discarded::increment(
                            self.shared.state.round_id,
                            Self::NAME
                        )
                    );
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
    ) -> Result<(StateMachineRequest, Span, ResponseSender), StateError> {
        debug!("waiting for the next incoming request");
        self.shared.io.request_rx.next().await.ok_or_else(|| {
            error!("request receiver broken: senders have been dropped");
            StateError::ChannelError("all message senders have been dropped!")
        })
    }

    fn try_next_request(
        &mut self,
    ) -> Result<Option<(StateMachineRequest, Span, ResponseSender)>, StateError> {
        match self.shared.io.request_rx.try_recv() {
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
        PhaseState::<StateError>::new(self.shared, err).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_machine::tests::utils;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn integration_update_round_id() {
        let (mut shared, event_subscriber, ..) = utils::init_shared().await;

        let phases = event_subscriber.phase_listener();
        // When starting the round ID should be 0
        let id = phases.get_latest().round_id;
        assert_eq!(id, 0);

        shared.set_round_id(1);
        assert_eq!(shared.state.round_id, 1);

        // Old events should still have the same round ID
        let id = phases.get_latest().round_id;
        assert_eq!(id, 0);

        // But new events should have the new round ID
        shared.io.events.broadcast_phase(PhaseName::Sum);
        let id = phases.get_latest().round_id;
        assert_eq!(id, 1);
    }
}
