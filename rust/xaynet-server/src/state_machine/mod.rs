//! The state machine that controls the execution of the PET protocol.
//!
//! # Overview
//!
//! ![](https://mermaid.ink/svg/eyJjb2RlIjoic3RhdGVEaWFncmFtXG5cdFsqXSAtLT4gSWRsZVxuXG4gIElkbGUgLS0-IFN1bVxuICBTdW0gLS0-IFVwZGF0ZVxuICBVcGRhdGUgLS0-IFN1bTJcbiAgU3VtMiAtLT4gVW5tYXNrXG4gIFVubWFzayAtLT4gSWRsZVxuXG4gIFN1bSAtLT4gRXJyb3JcbiAgVXBkYXRlIC0tPiBFcnJvclxuICBTdW0yIC0tPiBFcnJvclxuICBVbm1hc2sgLS0-IEVycm9yXG4gIEVycm9yIC0tPiBJZGxlXG4gIEVycm9yIC0tPiBTaHV0ZG93blxuXG4gIFNodXRkb3duIC0tPiBbKl1cblxuXG5cblxuXG5cblxuICAiLCJtZXJtYWlkIjp7InRoZW1lIjoibmV1dHJhbCJ9fQ)
//!
//! The [`StateMachine`] is responsible for executing the individual tasks of the PET protocol.
//! The main tasks include: building the sum and seed dictionaries, aggregating the masked
//! models, determining the applicable mask and unmasking the global masked model.
//!
//! Furthermore, the [`StateMachine`] publishes protocol events and handles protocol errors.
//!
//! The [`StateMachine`] as well as the PET settings can be configured in the config file.
//! See [here][settings] for more details.
//!
//! # Phase states
//!
//! **Idle**
//!
//! Publishes [`PhaseName::Idle`], increments the `round id` by `1`, invalidates the
//! [`SumDict`], [`SeedDict`], `scalar` and `mask length`, updates the [`EncryptKeyPair`],
//! `thresholds` as well as the `seed` and publishes the [`EncryptKeyPair`] and the
//! [`RoundParameters`].
//!
//! **Sum**
//!
//! Publishes [`PhaseName::Sum`], builds and publishes the [`SumDict`], ensures that enough sum
//! messages have been submitted and initializes the [`SeedDict`].
//!
//! **Update**
//!
//! Publishes [`PhaseName::Update`], publishes the `scalar`, builds and publishes the
//! [`SeedDict`], ensures that enough update messages have been submitted and aggregates the
//! masked model.
//!
//! **Sum2**
//!
//! Publishes [`PhaseName::Sum2`], builds the [`MaskDict`], ensures that enough sum2
//! messages have been submitted and determines the applicable mask for unmasking the global
//! masked model.
//!
//! **Unmask**
//!
//! Publishes [`PhaseName::Unmask`], unmasks the global masked model and publishes the global
//! model.
//!
//! **Error**
//!
//! Publishes [`PhaseName::Error`] and handles [`StateError`]s that can occur during the
//! execution of the [`StateMachine`]. In most cases, the error is handled by restarting the round.
//! However, if a [`StateError::ChannelError`] occurs, the [`StateMachine`] will shut down.
//!
//! **Shutdown**
//!
//! Publishes [`PhaseName::Shutdown`] and shuts down the [`StateMachine`]. During the shutdown,
//! the [`StateMachine`] performs a clean shutdown of the [Request][requests_idx] channel by
//! closing it and consuming all remaining messages.
//!
//! # Requests
//!
//! By initiating a new [`StateMachine`] via [`StateMachine::new()`], a new
//! [StateMachineRequest][requests_idx] channel is created, the function of which is to send
//! [`StateMachineRequest`]s to the [`StateMachine`]. The sender half of that channel
//! ([`RequestSender`]) is returned back to the caller of [`StateMachine::new()`], whereas the
//! receiver half ([`RequestReceiver`]) is used by the [`StateMachine`].
//!
//! See [here][requests] for more details.
//!
//! # Events
//!
//! During the execution of the PET protocol, the [`StateMachine`] will publish various events
//! (see Phase states). Everyone who is interested in the events can subscribe to the respective
//! events via the [`EventSubscriber`]. An [`EventSubscriber`] is automatically created when a new
//! [`StateMachine`] is created through [`StateMachine::new()`].
//!
//! See [here][events] for more details.
//!
//! [settings]: ../settings/index.html
//! [`PhaseName::Idle`]: crate::state_machine::phases::PhaseName::Idle
//! [`PhaseName::Sum`]: crate::state_machine::phases::PhaseName::Sum
//! [`PhaseName::Update`]: crate::state_machine::phases::PhaseName::Update
//! [`PhaseName::Sum2`]: crate::state_machine::phases::PhaseName::Sum2
//! [`PhaseName::Unmask`]: crate::state_machine::phases::PhaseName::Unmask
//! [`PhaseName::Error`]: crate::state_machine::phases::PhaseName::Error
//! [`PhaseName::Shutdown`]: crate::state_machine::phases::PhaseName::Shutdown
//! [`SumDict`]: xaynet_core::SumDict
//! [`SeedDict`]: xaynet_core::SeedDict
//! [`EncryptKeyPair`]: xaynet_core::crypto::EncryptKeyPair
//! [`RoundParameters`]: xaynet_core::common::RoundParameters
//! [`MaskDict`]: crate::state_machine::coordinator::MaskDict
//! [`StateMachineRequest`]: crate::state_machine::requests::StateMachineRequest
//! [requests_idx]: ./requests/index.html
//! [events]: ./events/index.html

pub mod coordinator;
pub mod events;
pub mod phases;
pub mod requests;

use self::{
    coordinator::CoordinatorState,
    events::{EventPublisher, EventSubscriber},
    phases::{
        Idle,
        Phase,
        PhaseName,
        PhaseState,
        PhaseStateError,
        Shared,
        Shutdown,
        Sum,
        Sum2,
        Unmask,
        Update,
    },
    requests::{RequestReceiver, RequestSender},
};
use crate::{
    settings::{MaskSettings, ModelSettings, PetSettings},
    storage::{redis, MaskDictIncrError, RedisError, SeedDictUpdateError, SumDictAddError},
};
use derive_more::From;
use thiserror::Error;
use xaynet_core::{mask::UnmaskingError, InitError};

#[cfg(feature = "metrics")]
use crate::metrics::MetricsSender;

/// Error returned when the state machine fails to handle a request
#[derive(Debug, Error)]
pub enum RequestError {
    #[error("the message was rejected")]
    MessageRejected,

    #[error("invalid update: the model or scalar sent by the participant could not be aggregated")]
    AggregationFailed,

    #[error("the request could not be processed due to an internal error")]
    InternalError,

    #[error("redis failed: {0}")]
    Redis(#[from] RedisError),

    #[error(transparent)]
    SeedDictUpdate(#[from] SeedDictUpdateError),

    #[error(transparent)]
    SumDictAdd(#[from] SumDictAddError),

    #[error(transparent)]
    MaskDictIncr(#[from] MaskDictIncrError),
}

pub type StateMachineResult = Result<(), RequestError>;

/// Error that occurs when unmasking of the global model fails.
#[derive(Error, Debug, Eq, PartialEq)]
pub enum UnmaskGlobalModelError {
    #[error("ambiguous masks were computed by the sum participants")]
    AmbiguousMasks,
    #[error("no mask found")]
    NoMask,
    #[error("unmasking error: {0}")]
    Unmasking(#[from] UnmaskingError),
}

/// The state machine with all its states.
#[derive(From)]
pub enum StateMachine {
    Idle(PhaseState<Idle>),
    Sum(PhaseState<Sum>),
    Update(PhaseState<Update>),
    Sum2(PhaseState<Sum2>),
    Unmask(PhaseState<Unmask>),
    Error(PhaseState<PhaseStateError>),
    Shutdown(PhaseState<Shutdown>),
}

impl StateMachine
where
    PhaseState<Idle>: Phase,
    PhaseState<Sum>: Phase,
    PhaseState<Update>: Phase,
    PhaseState<Sum2>: Phase,
    PhaseState<Unmask>: Phase,
    PhaseState<PhaseStateError>: Phase,
    PhaseState<Shutdown>: Phase,
{
    /// Creates a new state machine with the initial state [`Idle`].
    ///
    /// # Errors
    ///
    /// Fails if there is insufficient system entropy to generate secrets.
    ///
    /// <div class="information">
    ///     <div class="tooltip ignore" style="">ⓘ<span class="tooltiptext">Note</span></div>
    /// </div>
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    ///     <strong>Note</strong>: If the <code>StateMachine</code> is created via
    ///     <code>PhaseState::<S>::new(...)</code> it must be ensured that the module
    ///     <a href="https://docs.rs/sodiumoxide/0.2.5/sodiumoxide/fn.init.html">
    ///     <code>sodiumoxide::init()</code></a> has been initialized beforehand.
    /// </pre></div>
    ///
    /// For example:
    /// ```compile_fail
    /// sodiumoxide::init().unwrap();
    /// let state_machine =
    ///     StateMachine::from(PhaseState::<Idle>::new(coordinator_state, req_receiver));
    /// ```
    pub fn new(
        pet_settings: PetSettings,
        mask_settings: MaskSettings,
        model_settings: ModelSettings,
        redis: redis::Client,
        #[cfg(feature = "metrics")] metrics_tx: MetricsSender,
    ) -> Result<(Self, RequestSender, EventSubscriber), InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;

        let coordinator_state = CoordinatorState::new(pet_settings, mask_settings, model_settings);
        let (event_publisher, event_subscriber) = EventPublisher::init(
            coordinator_state.round_id,
            coordinator_state.keys.clone(),
            coordinator_state.round_params.clone(),
            PhaseName::Idle,
        );
        let (req_receiver, handle) = RequestReceiver::new();

        let shared = Shared::new(
            coordinator_state,
            event_publisher,
            req_receiver,
            redis,
            #[cfg(feature = "metrics")]
            metrics_tx,
        );

        let state_machine = StateMachine::from(PhaseState::<Idle>::new(shared));
        Ok((state_machine, handle, event_subscriber))
    }

    /// Moves the [`StateMachine`] to the next state and consumes the current one.
    /// Returns the next state or `None` if the [`StateMachine`] reached the state [`Shutdown`].
    pub async fn next(self) -> Option<Self> {
        match self {
            StateMachine::Idle(state) => state.run_phase().await,
            StateMachine::Sum(state) => state.run_phase().await,
            StateMachine::Update(state) => state.run_phase().await,
            StateMachine::Sum2(state) => state.run_phase().await,
            StateMachine::Unmask(state) => state.run_phase().await,
            StateMachine::Error(state) => state.run_phase().await,
            StateMachine::Shutdown(state) => state.run_phase().await,
        }
    }

    /// Runs the state machine until it shuts down.
    /// The [`StateMachine`] shuts down once all [`RequestSender`] have been dropped.
    pub async fn run(mut self) -> Option<()> {
        loop {
            self = self.next().await?;
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
