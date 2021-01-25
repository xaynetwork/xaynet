//! This module provides the `PhaseStates` of the [`StateMachine`].

mod error;
mod idle;
mod init;
mod pause;
mod shutdown;
mod sum;
mod sum2;
mod unmask;
mod update;

use std::fmt;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, error_span, info, warn, Span};
use tracing_futures::Instrument;

pub use self::{
    error::PhaseStateError,
    idle::{Idle, IdleStateError},
    init::Init,
    shutdown::Shutdown,
    sum::{Sum, SumStateError},
    sum2::Sum2,
    unmask::{Unmask, UnmaskStateError},
    update::{Update, UpdateStateError},
};
use super::{
    coordinator::{CountParameters, PhaseParameters},
    requests::UserRequest,
};
use crate::{
    metric,
    metrics::Measurement,
    state_machine::{
        coordinator::CoordinatorState,
        events::EventPublisher,
        requests::{RequestReceiver, ResponseSender, StateMachineRequest, UserRequestReceiver},
        RequestError, StateMachine,
    },
    storage::Storage,
};
/// The name of the current phase.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PhaseName {
    Init,
    Idle,
    Sum,
    Update,
    Sum2,
    Unmask,
    Error,
    Shutdown,
    Pause,
    Purge,
}

/// A trait that must be implemented by a state in order to move to a next state.
#[async_trait]
pub trait Phase<S>
where
    S: Storage,
{
    /// The name of the current phase.
    const NAME: PhaseName;

    /// Runs this phase to completion.
    ///
    /// See the [module level documentation] for more details.
    ///
    /// [module level documentation]: crate::state_machine
    async fn run(&mut self) -> Result<(), PhaseStateError>;

    ///
    async fn publish(&mut self) -> Result<(), PhaseStateError> {
        Ok(())
    }

    /// Moves from this state to the next state.
    ///
    /// See the [module level documentation] for more details.
    ///
    /// [module level documentation]: crate::state_machine
    fn next(self) -> Option<StateMachine<S>>;
}

/// A trait that must be implemented by a state to handle a request.
#[async_trait]
pub trait Handler {
    /// Handles a request.
    ///
    /// # Errors
    /// Fails on PET and storage errors.
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError>;
}

/// A struct that contains the coordinator state and the I/O interfaces that are shared and
/// accessible by all `PhaseState`s.
pub struct Shared<S>
where
    S: Storage,
{
    /// The coordinator state.
    pub(in crate::state_machine) state: CoordinatorState,
    /// The request receiver half.
    pub(in crate::state_machine) request_rx: RequestReceiver,

    // pub(in crate::state_machine) user_request_rx: UserRequestReceiver,
    /// The event publisher.
    pub(in crate::state_machine) events: EventPublisher,
    /// The store for storing coordinator and model data.
    pub(in crate::state_machine) store: S,
}

impl<S> fmt::Debug for Shared<S>
where
    S: Storage,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shared")
            .field("state", &self.state)
            .field("events", &self.events)
            .field("request_rx", &self.request_rx)
            // .field("user_request_rx", &self.user_request_rx)
            .finish()
    }
}

impl<S> Shared<S>
where
    S: Storage,
{
    /// Creates a new shared state.
    pub fn new(
        coordinator_state: CoordinatorState,
        publisher: EventPublisher,
        request_rx: RequestReceiver,
        // user_request_rx: UserRequestReceiver,
        store: S,
    ) -> Self {
        Self {
            state: coordinator_state,
            events: publisher,
            request_rx,
            // user_request_rx,
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
pub struct PhaseState<S, T>
where
    T: Storage,
{
    /// The private state.
    pub(in crate::state_machine) private: S,
    /// The shared coordinator state and I/O interfaces.
    pub(in crate::state_machine) shared: Shared<T>,
}

impl<S, T> PhaseState<S, T>
where
    Self: Handler + Phase<T>,
    T: Storage,
{
    /// Processes requests for as long as the given duration.
    async fn process_during(
        &mut self,
        dur: tokio::time::Duration,
        metrics: &mut MessageMetrics,
    ) -> Result<(), PhaseStateError> {
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
                    self.process_single(req, span, resp_tx, metrics).await;
                }
            }
        }
    }

    /// Processes requests until there are enough.
    async fn process_until_enough(
        &mut self,
        metrics: &mut MessageMetrics,
    ) -> Result<(), PhaseStateError> {
        while !metrics.has_enough_messages() {
            let (req, span, resp_tx) = self.next_request().await?;
            self.process_single(req, span, resp_tx, metrics).await;
        }
        Ok(())
    }

    /// Processes a single request.
    async fn process_single(
        &mut self,
        req: StateMachineRequest,
        span: Span,
        resp_tx: ResponseSender,
        metrics: &mut MessageMetrics,
    ) {
        let _span_guard = span.enter();

        let res = if metrics.has_overmuch_messages() {
            // discard if the maximum message count is reached
            metrics.increment_discarded();
            Err(RequestError::MessageDiscarded)
        } else {
            let handle_res = self.handle_request(req).await;
            if handle_res.is_ok() {
                // accept if processed successfully
                metrics.increment_accepted();
            } else {
                // otherwise reject
                metrics.increment_rejected();
            }

            handle_res
        };

        // This may error out if the receiver has already been dropped but it doesn't matter for us.
        let _ = resp_tx.send(res);
    }

    async fn handle_requests(
        &mut self,
        PhaseParameters { time, count }: PhaseParameters,
    ) -> Result<(), PhaseStateError> {
        let mut metrics =
            MessageMetrics::new(count.clone(), Self::NAME, self.shared.state.round_id);

        debug!("in phase for min {} and max {} seconds", time.min, time.max,);
        self.process_during(Duration::from_secs(time.min), &mut metrics)
            .await?;

        let time_left = time.max - time.min;
        timeout(
            Duration::from_secs(time_left),
            self.process_until_enough(&mut metrics),
        )
        .await??;

        info!(
            "in total {} messages accepted (min {} and max {} required)",
            metrics.accepted, count.min, count.max,
        );
        info!("in total {} messages rejected", metrics.rejected);
        info!("in total {} messages discarded", metrics.discarded);

        Ok(())
    }
}

pub struct MessageMetrics {
    /// The number of sum messages successfully processed.
    pub accepted: u64,
    /// The number of sum messages failed to processed.
    pub rejected: u64,
    /// The number of sum messages discarded without being processed.
    pub discarded: u64,
    //phase parameters
    limits: CountParameters,

    phase: PhaseName,
    round_id: u64,
}

impl MessageMetrics {
    fn new(limits: CountParameters, phase: PhaseName, round_id: u64) -> Self {
        Self {
            accepted: 0,
            rejected: 0,
            discarded: 0,
            limits,
            phase,
            round_id,
        }
    }

    fn has_enough_messages(&self) -> bool {
        self.accepted >= self.limits.min
    }

    fn has_overmuch_messages(&self) -> bool {
        self.accepted >= self.limits.max
    }

    fn increment_accepted(&mut self) {
        self.accepted += 1;
        debug!(
            "{} messages accepted (min {} and max {} required)",
            self.accepted, self.limits.min, self.limits.max,
        );
        // TODO: currently the metric! macro contains redundant information in case of
        // accepted messages: the `Measurement::MessageSum/Update/Sum2` as well as the
        // ("phase", name_u8). once we change those three enum variants to just one
        // `Measurement::MessageAccepted` we don't need this match workaround and can
        // call metric! directly.
        metric!(
            match self.phase {
                PhaseName::Sum => Measurement::MessageSum,
                PhaseName::Update => Measurement::MessageUpdate,
                PhaseName::Sum2 => Measurement::MessageSum2,
                _ => unreachable!(),
            },
            1,
            ("round_id", self.round_id),
            ("phase", self.phase as u8)
        );
    }

    fn increment_rejected(&mut self) {
        self.rejected += 1;
        debug!("{} messages rejected", self.rejected);
        metric!(
            Measurement::MessageRejected,
            1,
            ("round_id", self.round_id),
            ("phase", self.phase as u8)
        );
    }

    fn increment_discarded(&mut self) {
        self.discarded += 1;
        debug!("{} messages discarded", self.discarded);
        metric!(
            Measurement::MessageDiscarded,
            1,
            ("round_id", self.round_id),
            ("phase", self.phase as u8)
        );
    }
}

impl<S, T> PhaseState<S, T>
where
    Self: Phase<T>,
    T: Storage,
    S: Send,
{
    /// Run the current phase to completion, then transition to the
    /// next phase and return it.
    pub async fn run_phase(mut self, user_tx: &mut UserRequestReceiver) -> Option<StateMachine<T>> {
        let phase = <Self as Phase<_>>::NAME;
        let span = error_span!("run_phase", phase = ?phase);

        async move {
            info!("starting phase");
            info!("broadcasting phase event");
            self.shared.events.broadcast_phase(phase);
            metric!(Measurement::Phase, phase as u8);

            // tokio::select! {
            //     _ =  user_tx.next() => {
            //         warn!("");
            //     }
            //     res = self.run() => {

            //     }
            // }

            let run_res = self.run().await;
            let purge_res = self.maybe_purge_outdated_requests();
            if let Err(err) = run_res.and(purge_res) {
                return Some(self.into_error_state(err));
            };

            if let Err(err) = self.publish().await {
                return Some(self.into_error_state(err));
            };

            info!("phase ran successfully");
            info!("transitioning to the next phase");
            self.next()
        }
        .instrument(span)
        .await
    }

    fn maybe_purge_outdated_requests(&mut self) -> Result<(), PhaseStateError> {
        match <Self as Phase<_>>::NAME {
            PhaseName::Sum | PhaseName::Update | PhaseName::Sum2 => {
                debug!("purging outdated requests before transitioning");
                self.purge_outdated_requests()
            }
            _ => Ok(()),
        }
    }

    /// Process all the pending requests that are now considered
    /// outdated. This happens at the end of each phase, before
    /// transitioning to the next phase.
    fn purge_outdated_requests(&mut self) -> Result<(), PhaseStateError> {
        self.shared.events.broadcast_phase(PhaseName::Purge);
        metric!(Measurement::Phase, PhaseName::Purge as u8);

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
impl<S, T> PhaseState<S, T>
where
    T: Storage,
{
    /// Receives the next [`StateMachineRequest`].
    ///
    /// # Errors
    /// Returns [`PhaseStateError::RequestChannel`] when all sender halves have been dropped.
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

    fn into_error_state(self, err: PhaseStateError) -> StateMachine<T> {
        PhaseState::<PhaseStateError, _>::new(self.shared, err).into()
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
