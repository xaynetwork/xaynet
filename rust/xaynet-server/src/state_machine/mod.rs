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
//! Publishes [`PhaseName::Error`] and handles [`PhaseStateError`]s that can occur during the
//! execution of the [`StateMachine`]. In most cases, the error is handled by restarting the round.
//! However, if a [`PhaseStateError::Channel`] occurs, the [`StateMachine`] will shut down.
//!
//! **Shutdown**
//!
//! Publishes [`PhaseName::Shutdown`] and shuts down the [`StateMachine`]. During the shutdown,
//! the [`StateMachine`] performs a clean shutdown of the [Request][requests_idx] channel by
//! closing it and consuming all remaining messages.
//!
//! # Requests
//!
//! By initiating a new [`StateMachine`] via [`StateMachineInitializer::init()`], a new
//! [StateMachineRequest][requests_idx] channel is created, the function of which is to send
//! [`StateMachineRequest`]s to the [`StateMachine`]. The sender half of that channel
//! ([`RequestSender`]) is returned back to the caller of
//! [`StateMachineInitializer::init()`], whereas the receiver half ([`RequestReceiver`])
//! is used by the [`StateMachine`].
//!
//! See [here][requests] for more details.
//!
//! # Events
//!
//! During the execution of the PET protocol, the [`StateMachine`] will publish various events
//! (see Phase states). Everyone who is interested in the events can subscribe to the respective
//! events via the [`EventSubscriber`]. An [`EventSubscriber`] is automatically created when a new
//! [`StateMachine`] is created through [`StateMachineInitializer::init()`].
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
    events::{EventPublisher, EventSubscriber, ModelUpdate},
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
use xaynet_core::mask::UnmaskingError;

#[cfg(feature = "metrics")]
use crate::metrics::MetricsSender;

#[cfg(feature = "model-persistence")]
use xaynet_core::mask::Model;

#[cfg(feature = "model-persistence")]
use crate::{settings::RestoreSettings, storage::s3};

/// Error returned when the state machine fails to handle a request
#[derive(Debug, Error)]
pub enum RequestError {
    #[error("the message was rejected")]
    MessageRejected,

    #[error("invalid update: the model or scalar sent by the participant could not be aggregated")]
    AggregationFailed,

    #[error("the request could not be processed due to an internal error: {0}")]
    InternalError(&'static str),

    #[error("redis request failed: {0}")]
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

type StateMachineInitializationResult<T> = Result<T, StateMachineInitializationError>;

/// Error that can occur during the initialization of the [`StateMachine`].
#[derive(Debug, Error)]
pub enum StateMachineInitializationError {
    #[error("redis request failed: {0}")]
    Redis(#[from] RedisError),
    #[error("failed to initialize crypto library")]
    CryptoInit,
    #[error("failed to fetch global model: {0}")]
    GlobalModelUnavailable(String),
    #[error("{0}")]
    GlobalModelInvalid(String),
}

/// The state machine initializer that initializes a new state machine.
pub struct StateMachineInitializer {
    pet_settings: PetSettings,
    mask_settings: MaskSettings,
    model_settings: ModelSettings,
    #[cfg(feature = "model-persistence")]
    restore_settings: RestoreSettings,

    redis_handle: redis::Client,
    #[cfg(feature = "model-persistence")]
    s3_handle: s3::Client,
    #[cfg(feature = "metrics")]
    metrics_handle: MetricsSender,
}

impl StateMachineInitializer {
    /// Creates a new [`StateMachineInitializer`].
    pub fn new(
        pet_settings: PetSettings,
        mask_settings: MaskSettings,
        model_settings: ModelSettings,
        #[cfg(feature = "model-persistence")] restore_settings: RestoreSettings,
        redis_handle: redis::Client,
        #[cfg(feature = "model-persistence")] s3_handle: s3::Client,
        #[cfg(feature = "metrics")] metrics_handle: MetricsSender,
    ) -> Self {
        Self {
            pet_settings,
            mask_settings,
            model_settings,
            #[cfg(feature = "model-persistence")]
            restore_settings,
            redis_handle,
            #[cfg(feature = "model-persistence")]
            s3_handle,
            #[cfg(feature = "metrics")]
            metrics_handle,
        }
    }

    #[cfg(not(feature = "model-persistence"))]
    /// Initializes a new [`StateMachine`] with the given settings.
    pub async fn init(
        self,
    ) -> StateMachineInitializationResult<(StateMachine, RequestSender, EventSubscriber)> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(StateMachineInitializationError::CryptoInit))?;

        let (coordinator_state, global_model) = { self.from_settings().await? };
        Ok(self.init_state_machine(coordinator_state, global_model))
    }

    // Creates a new [`CoordinatorState`] from the given settings and flushes
    // all coordinator data in Redis. Should only be called for the first start
    // or if we need to perform reset.
    async fn from_settings(
        &self,
    ) -> StateMachineInitializationResult<(CoordinatorState, ModelUpdate)> {
        self.redis_handle
            .connection()
            .await
            .flush_coordinator_data()
            .await?;
        Ok((
            CoordinatorState::new(self.pet_settings, self.mask_settings, self.model_settings),
            ModelUpdate::Invalidate,
        ))
    }

    // Initializes a new [`StateMachine`] with its components.
    fn init_state_machine(
        self,
        coordinator_state: CoordinatorState,
        global_model: ModelUpdate,
    ) -> (StateMachine, RequestSender, EventSubscriber) {
        let (event_publisher, event_subscriber) = EventPublisher::init(
            coordinator_state.round_id,
            coordinator_state.keys.clone(),
            coordinator_state.round_params.clone(),
            PhaseName::Idle,
            global_model,
        );

        let (request_rx, request_tx) = RequestReceiver::new();

        let shared = Shared::new(
            coordinator_state,
            event_publisher,
            request_rx,
            self.redis_handle,
            #[cfg(feature = "model-persistence")]
            self.s3_handle,
            #[cfg(feature = "metrics")]
            self.metrics_handle,
        );

        let state_machine = StateMachine::from(PhaseState::<Idle>::new(shared));
        (state_machine, request_tx, event_subscriber)
    }
}

#[cfg(feature = "model-persistence")]
impl StateMachineInitializer {
    /// Initializes a new [`StateMachine`] by trying to restore the previous coordinator state
    /// along with the latest global model. After a successful initialization, the state machine
    /// always starts from a new round. This means that the round id is increased by one.
    /// If the state machine is reset during the initialization, the state machine starts
    /// with the round id `1`.
    ///
    /// # Behavior
    /// ![](https://mermaid.ink/svg/eyJjb2RlIjoic2VxdWVuY2VEaWFncmFtXG4gICAgYWx0IG5vIHJlc3RvcmVcbiAgICAgICAgQ29vcmRpbmF0b3ItPj4rUmVkaXM6IGZsdXNoIGRiXG4gICAgICAgIE5vdGUgb3ZlciBDb29yZGluYXRvcixSZWRpczogc3RhcnQgZnJvbSBzZXR0aW5ncyBcbiAgICBlbHNlXG4gICAgICAgIENvb3JkaW5hdG9yLT4-K1JlZGlzOiBnZXQgc3RhdGVcbiAgICAgICAgUmVkaXMtLT4-LUNvb3JkaW5hdG9yOiBzdGF0ZVxuICAgICAgICBhbHQgc3RhdGUgbm9uLWV4aXN0ZW50XG4gICAgICAgICAgQ29vcmRpbmF0b3ItPj4rUmVkaXM6IGZsdXNoIGRiXG4gICAgICAgICAgTm90ZSBvdmVyIENvb3JkaW5hdG9yLFJlZGlzOiBzdGFydCBmcm9tIHNldHRpbmdzIFxuICAgICAgICBlbHNlIHN0YXRlIGV4aXN0XG4gICAgICAgICAgQ29vcmRpbmF0b3ItPj4rUmVkaXM6IGdldCBsYXRlc3QgZ2xvYmFsIG1vZGVsIGlkXG4gICAgICAgICAgUmVkaXMtLT4-LUNvb3JkaW5hdG9yOiBnbG9iYWwgbW9kZWwgaWRcbiAgICAgICAgICBhbHQgZ2xvYmFsIG1vZGVsIGlkIG5vbi1leGlzdGVudFxuICAgICAgICAgICAgTm90ZSBvdmVyIENvb3JkaW5hdG9yLFMzOiByZXN0b3JlIGNvb3JkaW5hdG9yIHdpdGggbGF0ZXN0IHN0YXRlIGJ1dCB3aXRob3V0IGEgZ2xvYmFsIG1vZGVsIFxuICAgICAgICAgIGVsc2UgZ2xvYmFsIG1vZGVsIGlkIGV4aXN0XG4gICAgICAgICAgICBDb29yZGluYXRvci0-PitTMzogZ2V0IGdsb2JhbCBtb2RlbFxuICAgICAgICAgICAgUzMtLT4-LUNvb3JkaW5hdG9yOiBnbG9iYWwgbW9kZWxcbiAgICAgICAgICAgIGFsdCBnbG9iYWwgbW9kZWwgbm9uLWV4aXN0ZW50XG4gICAgICAgICAgICAgIE5vdGUgb3ZlciBDb29yZGluYXRvcixTMzogZXhpdCB3aXRoIGVycm9yXG4gICAgICAgICAgICBlbHNlIGdsb2JhbCBtb2RlbCBleGlzdFxuICAgICAgICAgICAgICBOb3RlIG92ZXIgQ29vcmRpbmF0b3IsUzM6IHJlc3RvcmUgY29vcmRpbmF0b3Igd2l0aCBsYXRlc3Qgc3RhdGUgYW5kIGxhdGVzdCBnbG9iYWwgbW9kZWwgXG4gICAgICAgICAgICBlbmRcbiAgICAgICAgICBlbmRcbiAgICAgICAgZW5kIFxuICAgIGVuZCIsIm1lcm1haWQiOnsidGhlbWUiOiJuZXV0cmFsIiwidGhlbWVWYXJpYWJsZXMiOnsicHJpbWFyeUNvbG9yIjoiI2VlZSIsImNvbnRyYXN0IjoiIzI2YSIsInNlY29uZGFyeUNvbG9yIjoiaHNsKDIxMCwgNjYuNjY2NjY2NjY2NyUsIDk1JSkiLCJiYWNrZ3JvdW5kIjoiI2ZmZmZmZiIsInRlcnRpYXJ5Q29sb3IiOiJoc2woLTE2MCwgMCUsIDkzLjMzMzMzMzMzMzMlKSIsInByaW1hcnlCb3JkZXJDb2xvciI6ImhzbCgwLCAwJSwgODMuMzMzMzMzMzMzMyUpIiwic2Vjb25kYXJ5Qm9yZGVyQ29sb3IiOiJoc2woMjEwLCAyNi42NjY2NjY2NjY3JSwgODUlKSIsInRlcnRpYXJ5Qm9yZGVyQ29sb3IiOiJoc2woLTE2MCwgMCUsIDgzLjMzMzMzMzMzMzMlKSIsInByaW1hcnlUZXh0Q29sb3IiOiIjMTExMTExIiwic2Vjb25kYXJ5VGV4dENvbG9yIjoicmdiKDIxLjI1LCAxMi43NSwgNC4yNSkiLCJ0ZXJ0aWFyeVRleHRDb2xvciI6InJnYigxNy4wMDAwMDAwMDAxLCAxNy4wMDAwMDAwMDAxLCAxNy4wMDAwMDAwMDAxKSIsImxpbmVDb2xvciI6IiM2NjYiLCJ0ZXh0Q29sb3IiOiIjMDAwMDAwIiwiYWx0QmFja2dyb3VuZCI6ImhzbCgyMTAsIDY2LjY2NjY2NjY2NjclLCA5NSUpIiwibWFpbkJrZyI6IiNlZWUiLCJzZWNvbmRCa2ciOiJoc2woMjEwLCA2Ni42NjY2NjY2NjY3JSwgOTUlKSIsImJvcmRlcjEiOiIjOTk5IiwiYm9yZGVyMiI6IiMyNmEiLCJub3RlIjoiI2ZmYSIsInRleHQiOiIjMzMzIiwiY3JpdGljYWwiOiIjZDQyIiwiZG9uZSI6IiNiYmIiLCJhcnJvd2hlYWRDb2xvciI6IiMzMzMzMzMiLCJmb250RmFtaWx5IjoiXCJ0cmVidWNoZXQgbXNcIiwgdmVyZGFuYSwgYXJpYWwiLCJmb250U2l6ZSI6IjE2cHgiLCJub2RlQmtnIjoiI2VlZSIsIm5vZGVCb3JkZXIiOiIjOTk5IiwiY2x1c3RlckJrZyI6ImhzbCgyMTAsIDY2LjY2NjY2NjY2NjclLCA5NSUpIiwiY2x1c3RlckJvcmRlciI6IiMyNmEiLCJkZWZhdWx0TGlua0NvbG9yIjoiIzY2NiIsInRpdGxlQ29sb3IiOiIjMzMzIiwiZWRnZUxhYmVsQmFja2dyb3VuZCI6IndoaXRlIiwiYWN0b3JCb3JkZXIiOiJoc2woMCwgMCUsIDgzJSkiLCJhY3RvckJrZyI6IiNlZWUiLCJhY3RvclRleHRDb2xvciI6IiMzMzMiLCJhY3RvckxpbmVDb2xvciI6IiM2NjYiLCJzaWduYWxDb2xvciI6IiMzMzMiLCJzaWduYWxUZXh0Q29sb3IiOiIjMzMzIiwibGFiZWxCb3hCa2dDb2xvciI6IiNlZWUiLCJsYWJlbEJveEJvcmRlckNvbG9yIjoiaHNsKDAsIDAlLCA4MyUpIiwibGFiZWxUZXh0Q29sb3IiOiIjMzMzIiwibG9vcFRleHRDb2xvciI6IiMzMzMiLCJub3RlQm9yZGVyQ29sb3IiOiJoc2woNjAsIDEwMCUsIDIzLjMzMzMzMzMzMzMlKSIsIm5vdGVCa2dDb2xvciI6IiNmZmEiLCJub3RlVGV4dENvbG9yIjoiIzMzMyIsImFjdGl2YXRpb25Cb3JkZXJDb2xvciI6IiM2NjYiLCJhY3RpdmF0aW9uQmtnQ29sb3IiOiIjZjRmNGY0Iiwic2VxdWVuY2VOdW1iZXJDb2xvciI6IndoaXRlIiwic2VjdGlvbkJrZ0NvbG9yIjoiaHNsKDIxMCwgNjYuNjY2NjY2NjY2NyUsIDcwJSkiLCJhbHRTZWN0aW9uQmtnQ29sb3IiOiJ3aGl0ZSIsInNlY3Rpb25Ca2dDb2xvcjIiOiJoc2woMjEwLCA2Ni42NjY2NjY2NjY3JSwgNzAlKSIsInRhc2tCb3JkZXJDb2xvciI6ImhzbCgyMTAsIDY2LjY2NjY2NjY2NjclLCAzMCUpIiwidGFza0JrZ0NvbG9yIjoiIzI2YSIsInRhc2tUZXh0TGlnaHRDb2xvciI6IndoaXRlIiwidGFza1RleHRDb2xvciI6IndoaXRlIiwidGFza1RleHREYXJrQ29sb3IiOiIjMzMzIiwidGFza1RleHRPdXRzaWRlQ29sb3IiOiIjMzMzIiwidGFza1RleHRDbGlja2FibGVDb2xvciI6IiMwMDMxNjMiLCJhY3RpdmVUYXNrQm9yZGVyQ29sb3IiOiJoc2woMjEwLCA2Ni42NjY2NjY2NjY3JSwgMzAlKSIsImFjdGl2ZVRhc2tCa2dDb2xvciI6IiNlZWUiLCJncmlkQ29sb3IiOiJoc2woMCwgMCUsIDkwJSkiLCJkb25lVGFza0JrZ0NvbG9yIjoiI2JiYiIsImRvbmVUYXNrQm9yZGVyQ29sb3IiOiIjNjY2IiwiY3JpdEJrZ0NvbG9yIjoiI2Q0MiIsImNyaXRCb3JkZXJDb2xvciI6ImhzbCgxMC45MDkwOTA5MDkxLCA3My4zMzMzMzMzMzMzJSwgNDAlKSIsInRvZGF5TGluZUNvbG9yIjoiI2Q0MiIsImxhYmVsQ29sb3IiOiJibGFjayIsImVycm9yQmtnQ29sb3IiOiIjNTUyMjIyIiwiZXJyb3JUZXh0Q29sb3IiOiIjNTUyMjIyIiwiY2xhc3NUZXh0IjoiIzExMTExMSIsImZpbGxUeXBlMCI6IiNlZWUiLCJmaWxsVHlwZTEiOiJoc2woMjEwLCA2Ni42NjY2NjY2NjY3JSwgOTUlKSIsImZpbGxUeXBlMiI6ImhzbCg2NCwgMCUsIDkzLjMzMzMzMzMzMzMlKSIsImZpbGxUeXBlMyI6ImhzbCgyNzQsIDY2LjY2NjY2NjY2NjclLCA5NSUpIiwiZmlsbFR5cGU0IjoiaHNsKC02NCwgMCUsIDkzLjMzMzMzMzMzMzMlKSIsImZpbGxUeXBlNSI6ImhzbCgxNDYsIDY2LjY2NjY2NjY2NjclLCA5NSUpIiwiZmlsbFR5cGU2IjoiaHNsKDEyOCwgMCUsIDkzLjMzMzMzMzMzMzMlKSIsImZpbGxUeXBlNyI6ImhzbCgzMzgsIDY2LjY2NjY2NjY2NjclLCA5NSUpIn19fQ)
    ///
    /// - If the [`RestoreSettings.enable`] flag is set to `false`, the current coordinator
    ///   state will be reset and a new [`StateMachine`] is created with the given settings.
    /// - If no coordinator state exists, the current coordinator state will be reset and a new
    ///   [`StateMachine`] is created with the given settings.
    /// - If a coordinator state exists but no global model has been created so far, the
    ///   [`StateMachine`] will be restored with the coordinator state but without a global model.
    /// - If a coordinator state and a global model exists, the [`StateMachine`] will be restored
    ///   with the coordinator state and the global model.
    /// - If a global model has been created but does not exists, the initialization will fail with
    ///   [`StateMachineInitializationError::GlobalModelUnavailable`].
    /// - If a global model exists but its properties do not match the coordinator model settings,
    ///   the initialization will fail with [`StateMachineInitializationError::GlobalModelInvalid`].
    /// - Any network error will cause the initialization to fail.
    pub async fn init(
        self,
    ) -> StateMachineInitializationResult<(StateMachine, RequestSender, EventSubscriber)> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(StateMachineInitializationError::CryptoInit))?;

        let (coordinator_state, global_model) = if self.restore_settings.no_restore {
            info!("requested not to restore the coordinator state");
            info!("initialize state machine from settings");
            self.from_settings().await?
        } else {
            self.from_previous_state().await?
        };

        Ok(self.init_state_machine(coordinator_state, global_model))
    }

    // see [`StateMachineInitializer::init`]
    async fn from_previous_state(
        &self,
    ) -> StateMachineInitializationResult<(CoordinatorState, ModelUpdate)> {
        let (coordinator_state, global_model) = if let Some(coordinator_state) = self
            .redis_handle
            .connection()
            .await
            .get_coordinator_state()
            .await?
        {
            self.try_restore_state(coordinator_state).await?
        } else {
            // no state in redis available seems to be a fresh start
            self.from_settings().await?
        };

        Ok((coordinator_state, global_model))
    }

    // see [`StateMachineInitializer::init`]
    async fn try_restore_state(
        &self,
        coordinator_state: CoordinatorState,
    ) -> StateMachineInitializationResult<(CoordinatorState, ModelUpdate)> {
        let latest_global_model_id = self
            .redis_handle
            .connection()
            .await
            .get_latest_global_model_id()
            .await?;

        let global_model_id = match latest_global_model_id {
            // the state machine was shut down before completing a round
            // we cannot use the round_id here because we increment the round_id after each restart
            // that means even if the round id is larger than one, it doesn't mean that a
            // round has ever been completed
            None => {
                debug!("apparently no round has been completed yet");
                debug!("restore coordinator without a global model");
                return Ok((coordinator_state, ModelUpdate::Invalidate));
            }
            Some(global_model_id) => global_model_id,
        };

        let global_model = self
            .download_global_model(&coordinator_state, &global_model_id)
            .await?;

        debug!(
            "restore coordinator with global model id: {}",
            global_model_id
        );
        Ok((
            coordinator_state,
            ModelUpdate::New(std::sync::Arc::new(global_model)),
        ))
    }

    // Downloads a global model and checks its properties for suitability.
    async fn download_global_model(
        &self,
        coordinator_state: &CoordinatorState,
        global_model_id: &str,
    ) -> StateMachineInitializationResult<Model> {
        match self.s3_handle.download_global_model(&global_model_id).await {
            Ok(global_model) => {
                if Self::model_properties_matches_settings(coordinator_state, &global_model) {
                    Ok(global_model)
                } else {
                    let error_msg = format!(
                        "the size of global model with the id {} does not match with the value of the model size setting {} != {}",
                        &global_model_id,
                        global_model.len(),
                        coordinator_state.model_size);

                    Err(StateMachineInitializationError::GlobalModelInvalid(
                        error_msg,
                    ))
                }
            }
            Err(err) => {
                warn!("cannot find global model {}", &global_model_id);
                // the model id exists but we cannot find it in S3 / Minio
                // here we better fail because if we restart a coordinator with an empty model
                // the clients will throw away their current global model and start from scratch
                Err(StateMachineInitializationError::GlobalModelUnavailable(
                    format!("{}", err),
                ))
            }
        }
    }

    // Checks whether the properties of the downloaded global model match the current
    // model settings of the coordinator.
    fn model_properties_matches_settings(
        coordinator_state: &CoordinatorState,
        global_model: &Model,
    ) -> bool {
        coordinator_state.model_size == global_model.len()
    }
}

#[cfg(test)]
pub(crate) mod tests;
