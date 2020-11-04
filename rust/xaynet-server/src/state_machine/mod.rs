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
//! Publishes [`PhaseName::Sum2`], builds the mask dictionary, ensures that enough sum2
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
        Idle, Phase, PhaseName, PhaseState, PhaseStateError, Shared, Shutdown, Sum, Sum2, Unmask,
        Update,
    },
    requests::{RequestReceiver, RequestSender},
};
use crate::{
    settings::{MaskSettings, ModelSettings, PetSettings},
    storage::{api::Store, MaskDictIncrError, RedisError, SeedDictUpdateError, SumDictAddError},
};
use derive_more::From;
use thiserror::Error;
use xaynet_core::mask::UnmaskingError;

#[cfg(feature = "metrics")]
use crate::metrics::MetricsSender;

#[cfg(feature = "model-persistence")]
use xaynet_core::mask::Model;

#[cfg(feature = "model-persistence")]
use crate::settings::RestoreSettings;

/// Error returned when the state machine fails to handle a request
#[derive(Debug, Error)]
pub enum RequestError {
    #[error("the message was rejected")]
    MessageRejected,

    #[error("invalid update: the model or scalar sent by the participant could not be aggregated")]
    AggregationFailed,

    #[error("the request could not be processed due to an internal error: {0}")]
    InternalError(&'static str),

    #[error(transparent)]
    SeedDictUpdate(#[from] SeedDictUpdateError),

    #[error(transparent)]
    SumDictAdd(#[from] SumDictAddError),

    #[error(transparent)]
    MaskDictIncr(#[from] MaskDictIncrError),

    #[error("failed to handle message: {0}")]
    MessageHandle(#[from] crate::storage::api::StorageError),
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
pub enum StateMachine<St>
where
    St: Store,
{
    Idle(PhaseState<Idle, St>),
    Sum(PhaseState<Sum, St>),
    Update(PhaseState<Update, St>),
    Sum2(PhaseState<Sum2, St>),
    Unmask(PhaseState<Unmask, St>),
    Error(PhaseState<PhaseStateError, St>),
    Shutdown(PhaseState<Shutdown, St>),
}

impl<St> StateMachine<St>
where
    PhaseState<Idle, St>: Phase<St>,
    PhaseState<Sum, St>: Phase<St>,
    PhaseState<Update, St>: Phase<St>,
    PhaseState<Sum2, St>: Phase<St>,
    PhaseState<Unmask, St>: Phase<St>,
    PhaseState<PhaseStateError, St>: Phase<St>,
    PhaseState<Shutdown, St>: Phase<St>,
    St: Store,
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
    #[error("failed to fetch global model")]
    GlobalModelUnavailable,
    #[error("{0}")]
    GlobalModelInvalid(String),
    #[error("{0}")]
    Storage(#[from] crate::storage::api::StorageError),
}

/// The state machine initializer that initializes a new state machine.
pub struct StateMachineInitializer<St>
where
    St: Store,
{
    pet_settings: PetSettings,
    mask_settings: MaskSettings,
    model_settings: ModelSettings,
    store: St,

    #[cfg(feature = "model-persistence")]
    restore_settings: RestoreSettings,
    #[cfg(feature = "metrics")]
    metrics_handle: MetricsSender,
}

impl<St> StateMachineInitializer<St>
where
    St: Store,
{
    /// Creates a new [`StateMachineInitializer`].
    pub fn new(
        pet_settings: PetSettings,
        mask_settings: MaskSettings,
        model_settings: ModelSettings,
        store: St,
        #[cfg(feature = "model-persistence")] restore_settings: RestoreSettings,
        #[cfg(feature = "metrics")] metrics_handle: MetricsSender,
    ) -> Self {
        Self {
            pet_settings,
            mask_settings,
            model_settings,
            store,
            #[cfg(feature = "model-persistence")]
            restore_settings,
            #[cfg(feature = "metrics")]
            metrics_handle,
        }
    }

    #[cfg(not(feature = "model-persistence"))]
    /// Initializes a new [`StateMachine`] with the given settings.
    pub async fn init(
        self,
    ) -> StateMachineInitializationResult<(StateMachine<S>, RequestSender, EventSubscriber)> {
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
        self.store.delete_coordinator_data().await?;
        Ok((
            CoordinatorState::new(
                self.pet_settings,
                self.mask_settings,
                self.model_settings.clone(),
            ),
            ModelUpdate::Invalidate,
        ))
    }

    // Initializes a new [`StateMachine`] with its components.
    fn init_state_machine(
        self,
        coordinator_state: CoordinatorState,
        global_model: ModelUpdate,
    ) -> (StateMachine<St>, RequestSender, EventSubscriber) {
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
            self.store,
            #[cfg(feature = "metrics")]
            self.metrics_handle,
        );

        let state_machine = StateMachine::from(PhaseState::<Idle, _>::new(shared));
        (state_machine, request_tx, event_subscriber)
    }
}

#[cfg(feature = "model-persistence")]
impl<St> StateMachineInitializer<St>
where
    St: Store,
{
    /// Initializes a new [`StateMachine`] by trying to restore the previous coordinator state
    /// along with the latest global model. After a successful initialization, the state machine
    /// always starts from a new round. This means that the round id is increased by one.
    /// If the state machine is reset during the initialization, the state machine starts
    /// with the round id `1`.
    ///
    /// # Behavior
    /// ![](https://mermaid.ink/svg/eyJjb2RlIjoic2VxdWVuY2VEaWFncmFtXG4gICAgYWx0IHJlc3RvcmUuZW5hYmxlID0gZmFsc2VcbiAgICAgICAgQ29vcmRpbmF0b3ItPj4rUmVkaXM6IGZsdXNoIGRiXG4gICAgICAgIE5vdGUgb3ZlciBDb29yZGluYXRvcixSZWRpczogc3RhcnQgZnJvbSBzZXR0aW5nc1xuICAgIGVsc2VcbiAgICAgICAgQ29vcmRpbmF0b3ItPj4rUmVkaXM6IGdldCBzdGF0ZVxuICAgICAgICBSZWRpcy0tPj4tQ29vcmRpbmF0b3I6IHN0YXRlXG4gICAgICAgIGFsdCBzdGF0ZSBub24tZXhpc3RlbnRcbiAgICAgICAgICAgIENvb3JkaW5hdG9yLT4-K1JlZGlzOiBmbHVzaCBkYlxuICAgICAgICAgICAgTm90ZSBvdmVyIENvb3JkaW5hdG9yLFJlZGlzOiBzdGFydCBmcm9tIHNldHRpbmdzXG4gICAgICAgIGVsc2Ugc3RhdGUgZXhpc3RcbiAgICAgICAgICAgIENvb3JkaW5hdG9yLT4-K1JlZGlzOiBnZXQgbGF0ZXN0IGdsb2JhbCBtb2RlbCBpZFxuICAgICAgICAgICAgUmVkaXMtLT4-LUNvb3JkaW5hdG9yOiBnbG9iYWwgbW9kZWwgaWRcbiAgICAgICAgICAgIGFsdCBnbG9iYWwgbW9kZWwgaWQgbm9uLWV4aXN0ZW50XG4gICAgICAgICAgICAgICAgTm90ZSBvdmVyIENvb3JkaW5hdG9yLFMzOiByZXN0b3JlIGNvb3JkaW5hdG9yIHdpdGggbGF0ZXN0IHN0YXRlIGJ1dCB3aXRob3V0IGEgZ2xvYmFsIG1vZGVsXG4gICAgICAgICAgICBlbHNlIGdsb2JhbCBtb2RlbCBpZCBleGlzdFxuICAgICAgICAgICAgICBDb29yZGluYXRvci0-PitTMzogZ2V0IGdsb2JhbCBtb2RlbFxuICAgICAgICAgICAgICBTMy0tPj4tQ29vcmRpbmF0b3I6IGdsb2JhbCBtb2RlbFxuICAgICAgICAgICAgICBhbHQgZ2xvYmFsIG1vZGVsIG5vbi1leGlzdGVudFxuICAgICAgICAgICAgICAgIE5vdGUgb3ZlciBDb29yZGluYXRvcixTMzogZXhpdCB3aXRoIGVycm9yXG4gICAgICAgICAgICAgIGVsc2UgZ2xvYmFsIG1vZGVsIGV4aXN0XG4gICAgICAgICAgICAgICAgTm90ZSBvdmVyIENvb3JkaW5hdG9yLFMzOiByZXN0b3JlIGNvb3JkaW5hdG9yIHdpdGggbGF0ZXN0IHN0YXRlIGFuZCBsYXRlc3QgZ2xvYmFsIG1vZGVsXG4gICAgICAgICAgICAgIGVuZFxuICAgICAgICAgICAgZW5kXG4gICAgICAgICAgZW5kXG4gICAgICAgIGVuZCIsIm1lcm1haWQiOnsidGhlbWUiOiJkZWZhdWx0IiwidGhlbWVWYXJpYWJsZXMiOnsiYmFja2dyb3VuZCI6IndoaXRlIiwicHJpbWFyeUNvbG9yIjoiI0VDRUNGRiIsInNlY29uZGFyeUNvbG9yIjoiI2ZmZmZkZSIsInRlcnRpYXJ5Q29sb3IiOiJoc2woODAsIDEwMCUsIDk2LjI3NDUwOTgwMzklKSIsInByaW1hcnlCb3JkZXJDb2xvciI6ImhzbCgyNDAsIDYwJSwgODYuMjc0NTA5ODAzOSUpIiwic2Vjb25kYXJ5Qm9yZGVyQ29sb3IiOiJoc2woNjAsIDYwJSwgODMuNTI5NDExNzY0NyUpIiwidGVydGlhcnlCb3JkZXJDb2xvciI6ImhzbCg4MCwgNjAlLCA4Ni4yNzQ1MDk4MDM5JSkiLCJwcmltYXJ5VGV4dENvbG9yIjoiIzEzMTMwMCIsInNlY29uZGFyeVRleHRDb2xvciI6IiMwMDAwMjEiLCJ0ZXJ0aWFyeVRleHRDb2xvciI6InJnYig5LjUwMDAwMDAwMDEsIDkuNTAwMDAwMDAwMSwgOS41MDAwMDAwMDAxKSIsImxpbmVDb2xvciI6IiMzMzMzMzMiLCJ0ZXh0Q29sb3IiOiIjMzMzIiwibWFpbkJrZyI6IiNFQ0VDRkYiLCJzZWNvbmRCa2ciOiIjZmZmZmRlIiwiYm9yZGVyMSI6IiM5MzcwREIiLCJib3JkZXIyIjoiI2FhYWEzMyIsImFycm93aGVhZENvbG9yIjoiIzMzMzMzMyIsImZvbnRGYW1pbHkiOiJcInRyZWJ1Y2hldCBtc1wiLCB2ZXJkYW5hLCBhcmlhbCIsImZvbnRTaXplIjoiMTZweCIsImxhYmVsQmFja2dyb3VuZCI6IiNlOGU4ZTgiLCJub2RlQmtnIjoiI0VDRUNGRiIsIm5vZGVCb3JkZXIiOiIjOTM3MERCIiwiY2x1c3RlckJrZyI6IiNmZmZmZGUiLCJjbHVzdGVyQm9yZGVyIjoiI2FhYWEzMyIsImRlZmF1bHRMaW5rQ29sb3IiOiIjMzMzMzMzIiwidGl0bGVDb2xvciI6IiMzMzMiLCJlZGdlTGFiZWxCYWNrZ3JvdW5kIjoiI2U4ZThlOCIsImFjdG9yQm9yZGVyIjoiaHNsKDI1OS42MjYxNjgyMjQzLCA1OS43NzY1MzYzMTI4JSwgODcuOTAxOTYwNzg0MyUpIiwiYWN0b3JCa2ciOiIjRUNFQ0ZGIiwiYWN0b3JUZXh0Q29sb3IiOiJibGFjayIsImFjdG9yTGluZUNvbG9yIjoiZ3JleSIsInNpZ25hbENvbG9yIjoiIzMzMyIsInNpZ25hbFRleHRDb2xvciI6IiMzMzMiLCJsYWJlbEJveEJrZ0NvbG9yIjoiI0VDRUNGRiIsImxhYmVsQm94Qm9yZGVyQ29sb3IiOiJoc2woMjU5LjYyNjE2ODIyNDMsIDU5Ljc3NjUzNjMxMjglLCA4Ny45MDE5NjA3ODQzJSkiLCJsYWJlbFRleHRDb2xvciI6ImJsYWNrIiwibG9vcFRleHRDb2xvciI6ImJsYWNrIiwibm90ZUJvcmRlckNvbG9yIjoiI2FhYWEzMyIsIm5vdGVCa2dDb2xvciI6IiNmZmY1YWQiLCJub3RlVGV4dENvbG9yIjoiYmxhY2siLCJhY3RpdmF0aW9uQm9yZGVyQ29sb3IiOiIjNjY2IiwiYWN0aXZhdGlvbkJrZ0NvbG9yIjoiI2Y0ZjRmNCIsInNlcXVlbmNlTnVtYmVyQ29sb3IiOiJ3aGl0ZSIsInNlY3Rpb25Ca2dDb2xvciI6InJnYmEoMTAyLCAxMDIsIDI1NSwgMC40OSkiLCJhbHRTZWN0aW9uQmtnQ29sb3IiOiJ3aGl0ZSIsInNlY3Rpb25Ca2dDb2xvcjIiOiIjZmZmNDAwIiwidGFza0JvcmRlckNvbG9yIjoiIzUzNGZiYyIsInRhc2tCa2dDb2xvciI6IiM4YTkwZGQiLCJ0YXNrVGV4dExpZ2h0Q29sb3IiOiJ3aGl0ZSIsInRhc2tUZXh0Q29sb3IiOiJ3aGl0ZSIsInRhc2tUZXh0RGFya0NvbG9yIjoiYmxhY2siLCJ0YXNrVGV4dE91dHNpZGVDb2xvciI6ImJsYWNrIiwidGFza1RleHRDbGlja2FibGVDb2xvciI6IiMwMDMxNjMiLCJhY3RpdmVUYXNrQm9yZGVyQ29sb3IiOiIjNTM0ZmJjIiwiYWN0aXZlVGFza0JrZ0NvbG9yIjoiI2JmYzdmZiIsImdyaWRDb2xvciI6ImxpZ2h0Z3JleSIsImRvbmVUYXNrQmtnQ29sb3IiOiJsaWdodGdyZXkiLCJkb25lVGFza0JvcmRlckNvbG9yIjoiZ3JleSIsImNyaXRCb3JkZXJDb2xvciI6IiNmZjg4ODgiLCJjcml0QmtnQ29sb3IiOiJyZWQiLCJ0b2RheUxpbmVDb2xvciI6InJlZCIsImxhYmVsQ29sb3IiOiJibGFjayIsImVycm9yQmtnQ29sb3IiOiIjNTUyMjIyIiwiZXJyb3JUZXh0Q29sb3IiOiIjNTUyMjIyIiwiY2xhc3NUZXh0IjoiIzEzMTMwMCIsImZpbGxUeXBlMCI6IiNFQ0VDRkYiLCJmaWxsVHlwZTEiOiIjZmZmZmRlIiwiZmlsbFR5cGUyIjoiaHNsKDMwNCwgMTAwJSwgOTYuMjc0NTA5ODAzOSUpIiwiZmlsbFR5cGUzIjoiaHNsKDEyNCwgMTAwJSwgOTMuNTI5NDExNzY0NyUpIiwiZmlsbFR5cGU0IjoiaHNsKDE3NiwgMTAwJSwgOTYuMjc0NTA5ODAzOSUpIiwiZmlsbFR5cGU1IjoiaHNsKC00LCAxMDAlLCA5My41Mjk0MTE3NjQ3JSkiLCJmaWxsVHlwZTYiOiJoc2woOCwgMTAwJSwgOTYuMjc0NTA5ODAzOSUpIiwiZmlsbFR5cGU3IjoiaHNsKDE4OCwgMTAwJSwgOTMuNTI5NDExNzY0NyUpIn19LCJ1cGRhdGVFZGl0b3IiOmZhbHNlfQ)
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
    ) -> StateMachineInitializationResult<(StateMachine<St>, RequestSender, EventSubscriber)> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(StateMachineInitializationError::CryptoInit))?;

        let (coordinator_state, global_model) = if self.restore_settings.enable {
            self.from_previous_state().await?
        } else {
            info!("restoring coordinator state is disabled");
            info!("initialize state machine from settings");
            self.from_settings().await?
        };

        Ok(self.init_state_machine(coordinator_state, global_model))
    }

    // see [`StateMachineInitializer::init`]
    async fn from_previous_state(
        &self,
    ) -> StateMachineInitializationResult<(CoordinatorState, ModelUpdate)> {
        let (coordinator_state, global_model) =
            if let Some(coordinator_state) = self.store.get_coordinator_state().await? {
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
        let latest_global_model_id = self.store.get_latest_global_model_id().await?;

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
        match self.store.get_global_model(&global_model_id).await? {
            Some(global_model) => {
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
            None => {
                warn!("cannot find global model {}", &global_model_id);
                // the model id exists but we cannot find it in S3 / Minio
                // here we better fail because if we restart a coordinator with an empty model
                // the clients will throw away their current global model and start from scratch
                Err(StateMachineInitializationError::GlobalModelUnavailable)
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
