use std::fmt;

use async_trait::async_trait;
use derive_more::Display;
use futures::StreamExt;
use tracing::{debug, error, error_span, info, warn, Span};
use tracing_futures::Instrument;

use crate::{
    discarded,
    metric,
    metrics::Measurement,
    state_machine::{
        coordinator::CoordinatorState,
        events::EventPublisher,
        phases::{Failure, PhaseError},
        requests::{RequestError, RequestReceiver, ResponseSender, StateMachineRequest},
        StateMachine,
    },
    storage::Storage,
};

/// The name of the current phase.
#[derive(Clone, Copy, Debug, Display, Eq, PartialEq)]
pub enum PhaseName {
    #[display(fmt = "Idle")]
    Idle,
    #[display(fmt = "Sum")]
    Sum,
    #[display(fmt = "Update")]
    Update,
    #[display(fmt = "Sum2")]
    Sum2,
    #[display(fmt = "Unmask")]
    Unmask,
    #[display(fmt = "Error")]
    Error,
    #[display(fmt = "Shutdown")]
    Shutdown,
}

/// A trait that must be implemented by a state in order to move to a next state.
///
/// See the [module level documentation] for more details.
///
/// [module level documentation]: crate::state_machine
#[async_trait]
pub trait Phase<T>
where
    T: Storage,
{
    /// The name of the current phase.
    const NAME: PhaseName;

    /// Performs the tasks of this phase.
    async fn process(&mut self) -> Result<(), PhaseError>;
    // TODO: add a filter service in PetMessageHandler that only passes through messages if
    // the state machine is in one of the Sum, Update or Sum2 phases. then we can add a Purge
    // phase here which gets broadcasted when the purge starts to prevent further incomming
    // messages, which means we can split `purge()` from `process()` and use a no-op default impl
    // for all phases except Sum, Update and Sum. until then we have to have a purge impl in every
    // phase, which also means that the metrics can be a bit off.

    /// Broadcasts data of this phase (nothing by default).
    fn broadcast(&mut self) {}

    /// Moves from this phase to the next phase.
    async fn next(self) -> Option<StateMachine<T>>;
}

/// A struct that contains the coordinator state and the I/O interfaces that are shared and
/// accessible by all `PhaseState`s.
pub struct Shared<T> {
    /// The coordinator state.
    pub(in crate::state_machine) state: CoordinatorState,
    /// The request receiver half.
    pub(in crate::state_machine) request_rx: RequestReceiver,
    /// The event publisher.
    pub(in crate::state_machine) events: EventPublisher,
    /// The store for storing coordinator and model data.
    pub(in crate::state_machine) store: T,
}

impl<T> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shared")
            .field("state", &self.state)
            .field("request_rx", &self.request_rx)
            .field("events", &self.events)
            .finish()
    }
}

impl<T> Shared<T> {
    /// Creates a new shared state.
    pub fn new(
        coordinator_state: CoordinatorState,
        publisher: EventPublisher,
        request_rx: RequestReceiver,
        store: T,
    ) -> Self {
        Self {
            state: coordinator_state,
            request_rx,
            events: publisher,
            store,
        }
    }

    /// Sets the round ID to the given value.
    pub fn set_round_id(&mut self, id: u64) {
        self.state.round_id = id;
        self.events.set_round_id(id);
    }

    /// Returns the current round ID.
    pub fn round_id(&self) -> u64 {
        self.state.round_id
    }
}

/// The state corresponding to a phase of the PET protocol.
///
/// This contains the state-dependent `private` state and the state-independent `shared` state
/// which is shared across state transitions.
pub struct PhaseState<S, T> {
    /// The private state.
    pub(in crate::state_machine) private: S,
    /// The shared coordinator state and I/O interfaces.
    pub(in crate::state_machine) shared: Shared<T>,
}

impl<S, T> PhaseState<S, T>
where
    S: Send,
    T: Storage,
    Self: Phase<T>,
{
    /// Runs the current phase to completion.
    ///
    /// 1. Performs the phase tasks.
    /// 2. Purges outdated phase messages.
    /// 3. Broadcasts the phase data.
    /// 4. Transitions to the next phase.
    pub async fn run_phase(mut self) -> Option<StateMachine<T>> {
        let phase = Self::NAME;
        let span = error_span!("run_phase", phase = %phase);

        async move {
            info!("starting phase");
            self.shared.events.broadcast_phase(phase);
            metric!(Measurement::Phase, phase as u8);

            if let Err(err) = self.process().await {
                warn!("failed to perform the phase tasks");
                return Some(self.into_failure_state(err));
            }
            info!("phase ran successfully");

            if let Err(err) = self.purge_outdated_requests() {
                warn!("failed to purge outdated requests");
                match phase {
                    PhaseName::Error | PhaseName::Shutdown => {
                        debug!(
                            "already in {} phase: ignoring error while purging outdated requests",
                            phase,
                        );
                    }
                    _ => return Some(self.into_failure_state(err)),
                }
            }

            self.broadcast();

            info!("transitioning to the next phase");
            self.next().await
        }
        .instrument(span)
        .await
    }

    /// Purges all pending requests that are considered outdated at the end of a successful phase.
    fn purge_outdated_requests(&mut self) -> Result<(), PhaseError> {
        info!("discarding outdated requests");
        while let Some((_, span, resp_tx)) = self.try_next_request()? {
            debug!("discarding outdated request");
            let _span_guard = span.enter();
            discarded!(self.shared.state.round_id, Self::NAME);
            let _ = resp_tx.send(Err(RequestError::MessageDiscarded));
        }
        Ok(())
    }
}

impl<S, T> PhaseState<S, T> {
    /// Receives the next [`StateMachineRequest`].
    ///
    /// # Errors
    /// Returns [`PhaseError::RequestChannel`] when all sender halves have been dropped.
    pub async fn next_request(
        &mut self,
    ) -> Result<(StateMachineRequest, Span, ResponseSender), PhaseError> {
        debug!("waiting for the next incoming request");
        self.shared.request_rx.next().await.ok_or_else(|| {
            error!("request receiver broken: senders have been dropped");
            PhaseError::RequestChannel("all message senders have been dropped!")
        })
    }

    pub fn try_next_request(
        &mut self,
    ) -> Result<Option<(StateMachineRequest, Span, ResponseSender)>, PhaseError> {
        match self.shared.request_rx.try_recv() {
            Some(Some(item)) => Ok(Some(item)),
            None => {
                debug!("no pending request");
                Ok(None)
            }
            Some(None) => {
                warn!("failed to get next pending request: channel shut down");
                Err(PhaseError::RequestChannel(
                    "all message senders have been dropped!",
                ))
            }
        }
    }

    fn into_failure_state(self, err: PhaseError) -> StateMachine<T> {
        PhaseState::<Failure, _>::new(self.shared, err).into()
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;
    use crate::{state_machine::tests::utils, storage::tests::init_store};

    #[tokio::test]
    #[serial]
    async fn integration_update_round_id() {
        let store = init_store().await;
        let coordinator_state = utils::coordinator_state();
        let (mut shared, _, event_subscriber) = utils::init_shared(coordinator_state, store);

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
        shared.events.broadcast_phase(PhaseName::Sum);
        let id = phases.get_latest().round_id;
        assert_eq!(id, 1);
    }
}
