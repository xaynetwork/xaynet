//! This module provides the `PhaseStates` of the [`StateMachine`].

mod error;
mod idle;
mod shutdown;
mod sum;
mod sum2;
mod unmask;
mod update;

use std::fmt;

use async_trait::async_trait;
use futures::StreamExt;
use tracing::{debug, error, error_span, info, warn, Span};
use tracing_futures::Instrument;

pub use self::{
    error::PhaseStateError,
    idle::{Idle, IdleStateError},
    shutdown::Shutdown,
    sum::{Sum, SumStateError},
    sum2::Sum2,
    unmask::{Unmask, UnmaskStateError},
    update::{Update, UpdateStateError},
};
use crate::{
    metric,
    metrics::Measurement,
    state_machine::{
        coordinator::CoordinatorState,
        events::EventPublisher,
        requests::{RequestReceiver, ResponseSender, StateMachineRequest},
        RequestError,
        StateMachine,
    },
    storage::{CoordinatorStorage, ModelStorage, Store},
};

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
pub trait Phase<C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Name of the current phase
    const NAME: PhaseName;

    /// Run this phase to completion
    async fn run(&mut self) -> Result<(), PhaseStateError>;

    /// Moves from this state to the next state.
    fn next(self) -> Option<StateMachine<C, M>>;
}

/// A trait that must be implemented by a state to handle a request.
#[async_trait]
pub trait Handler {
    /// Handles a request.
    ///
    /// # Errors
    /// Fails on PET and storage errors.
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError>;

    /// Checks whether enough requests have been processed successfully wrt the PET settings.
    fn has_enough_messages(&self) -> bool;

    /// Checks whether too many requests are processed wrt the PET settings.
    fn has_overmuch_messages(&self) -> bool;

    /// Increments the counter for accepted requests.
    fn increment_accepted(&mut self);

    /// Increments the counter for rejected requests.
    fn increment_rejected(&mut self);

    /// Increments the counter for discarded requests.
    fn increment_discarded(&mut self);
}

/// A struct that contains the coordinator state and the I/O interfaces that are shared and
/// accessible by all `PhaseState`s.
pub struct Shared<C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// The coordinator state.
    pub(in crate::state_machine) state: CoordinatorState,
    /// The request receiver half.
    pub(in crate::state_machine) request_rx: RequestReceiver,
    /// The event publisher.
    pub(in crate::state_machine) events: EventPublisher,
    /// The store for storing coordinator and model data.
    pub(in crate::state_machine) store: Store<C, M>,
}

impl<C, M> fmt::Debug for Shared<C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shared")
            .field("state", &self.state)
            .field("request_rx", &self.request_rx)
            .field("events", &self.events)
            .finish()
    }
}

impl<C, M> Shared<C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    pub fn new(
        coordinator_state: CoordinatorState,
        publisher: EventPublisher,
        request_rx: RequestReceiver,
        store: Store<C, M>,
    ) -> Self {
        Self {
            state: coordinator_state,
            request_rx,
            events: publisher,
            store,
        }
    }

    /// Set the round ID to the given value
    pub fn set_round_id(&mut self, id: u64) {
        self.state.round_id = id;
        self.events.set_round_id(id);
    }

    /// Return the current round ID
    pub fn round_id(&self) -> u64 {
        self.state.round_id
    }
}

/// The state corresponding to a phase of the PET protocol.
///
/// This contains the state-dependent `private` state and the state-independent `shared` state
/// which is shared across state transitions.
pub struct PhaseState<S, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// The private state.
    pub(in crate::state_machine) private: S,
    /// The shared coordinator state and I/O interfaces.
    pub(in crate::state_machine) shared: Shared<C, M>,
}

impl<S, C, M> PhaseState<S, C, M>
where
    Self: Handler + Phase<C, M>,
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Processes requests for as long as the given duration.
    async fn process_during(&mut self, dur: tokio::time::Duration) -> Result<(), PhaseStateError> {
        // even though this is called a `Delay` it is actually a fixed deadline, hence the loop
        // below doesn't start the delay again at each iteration and just checks for the deadline
        let mut delay = tokio::time::delay_for(dur);

        loop {
            tokio::select! {
                _ = &mut delay => {
                    debug!("duration elapsed");
                    break Ok(());
                }
                next = self.next_request() => {
                    let (req, span, resp_tx) = next?;
                    self.process_single(req, span, resp_tx).await;
                }
            }
        }
    }

    /// Processes the next available request.
    async fn process_next(&mut self) -> Result<(), PhaseStateError> {
        let (req, span, resp_tx) = self.next_request().await?;
        self.process_single(req, span, resp_tx).await;
        Ok(())
    }

    /// Processes a single request.
    async fn process_single(
        &mut self,
        req: StateMachineRequest,
        span: Span,
        resp_tx: ResponseSender,
    ) {
        let _span_guard = span.enter();

        let res = if self.has_overmuch_messages() {
            // discard if the maximum message count is reached
            self.increment_discarded();
            metric!(
                Measurement::MessageDiscarded,
                1,
                ("round_id", self.shared.state.round_id),
                ("phase", Self::NAME as u8)
            );
            Err(RequestError::MessageDiscarded)
        } else {
            match self.handle_request(req).await {
                // accept if processed successfully
                ok @ Ok(_) => {
                    self.increment_accepted();
                    // TODO: currently the metric! macro contains redundant information in case of
                    // accepted messages: the `Measurement::MessageSum/Update/Sum2` as well as the
                    // ("phase", name_u8). once we changed those three enum variants to just
                    // `Measurement::MessageAccepted` we don't need this match workaround and can
                    // call metric! directly.
                    metric!(
                        match Self::NAME {
                            PhaseName::Sum => Measurement::MessageSum,
                            PhaseName::Update => Measurement::MessageUpdate,
                            PhaseName::Sum2 => Measurement::MessageSum2,
                            _ => unreachable!(),
                        },
                        1,
                        ("round_id", self.shared.state.round_id),
                        ("phase", Self::NAME as u8)
                    );
                    ok
                }
                // otherwise reject
                error @ Err(_) => {
                    self.increment_rejected();
                    metric!(
                        Measurement::MessageRejected,
                        1,
                        ("round_id", self.shared.state.round_id),
                        ("phase", Self::NAME as u8)
                    );
                    error
                }
            }
        };

        // This may error out if the receiver has already been dropped but it doesn't matter for us.
        let _ = resp_tx.send(res);
    }
}

impl<S, C, M> PhaseState<S, C, M>
where
    Self: Phase<C, M>,
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Run the current phase to completion, then transition to the
    /// next phase and return it.
    pub async fn run_phase(mut self) -> Option<StateMachine<C, M>> {
        let phase = <Self as Phase<_, _>>::NAME;
        let span = error_span!("run_phase", phase = ?phase);

        async move {
            info!("starting phase");
            info!("broadcasting phase event");
            self.shared.events.broadcast_phase(phase);

            metric!(Measurement::Phase, phase as u8);

            if let Err(err) = self.run().await {
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
    fn purge_outdated_requests(&mut self) -> Result<(), PhaseStateError> {
        loop {
            match self.try_next_request()? {
                Some((_req, span, resp_tx)) => {
                    let _span_guard = span.enter();
                    info!("discarding outdated request");
                    let _ = resp_tx.send(Err(RequestError::MessageRejected));

                    metric!(
                        Measurement::MessageDiscarded,
                        1,
                        ("round_id", self.shared.state.round_id),
                        ("phase", Self::NAME as u8)
                    );
                }
                None => return Ok(()),
            }
        }
    }
}

// Functions that are available to all states
impl<S, C, M> PhaseState<S, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Receives the next [`Request`].
    ///
    /// # Errors
    /// Returns [`StateError::ChannelError`] when all sender halves have been dropped.
    async fn next_request(
        &mut self,
    ) -> Result<(StateMachineRequest, Span, ResponseSender), PhaseStateError> {
        debug!("waiting for the next incoming request");
        self.shared.request_rx.next().await.ok_or_else(|| {
            error!("request receiver broken: senders have been dropped");
            PhaseStateError::RequestChannel("all message senders have been dropped!")
        })
    }

    fn try_next_request(
        &mut self,
    ) -> Result<Option<(StateMachineRequest, Span, ResponseSender)>, PhaseStateError> {
        match self.shared.request_rx.try_recv() {
            Ok(item) => Ok(Some(item)),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                debug!("no pending request");
                Ok(None)
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Closed) => {
                warn!("failed to get next pending request: channel shut down");
                Err(PhaseStateError::RequestChannel(
                    "all message senders have been dropped!",
                ))
            }
        }
    }

    fn into_error_state(self, err: PhaseStateError) -> StateMachine<C, M> {
        PhaseState::<PhaseStateError, _, _>::new(self.shared, err).into()
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
