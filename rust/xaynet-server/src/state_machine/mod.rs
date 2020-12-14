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
//! Publishes [`PhaseName::Update`], builds and publishes the [`SeedDict`], ensures that enough
//! update messages have been submitted and aggregates the masked model.
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
//! However, if a [`PhaseStateError::RequestChannel`] occurs, the [`StateMachine`] will shut down.
//!
//! **Shutdown**
//!
//! Publishes [`PhaseName::Shutdown`] and shuts down the [`StateMachine`]. During the shutdown,
//! the [`StateMachine`] performs a clean shutdown of the [Request][requests] channel by
//! closing it and consuming all remaining messages.
//!
//! # Requests
//!
//! By initiating a new [`StateMachine`] via [`StateMachineInitializer::init()`], a new
//! [StateMachineRequest][requests] channel is created, the function of which is to send
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
//! [settings]: crate::settings
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
//! [requests]: crate::state_machine::requests
//! [`RequestSender`]: crate::state_machine::requests::RequestSender
//! [`RequestReceiver`]: crate::state_machine::requests::RequestReceiver
//! [events]: crate::state_machine::events
//! [`EventSubscriber`]: crate::state_machine::events::EventSubscriber

pub mod coordinator;
pub mod events;
pub mod initializer;
pub mod phases;
pub mod requests;
pub use self::initializer::StateMachineInitializer;

use derive_more::From;
use thiserror::Error;

use self::phases::{Idle, Phase, PhaseState, PhaseStateError, Shutdown, Sum, Sum2, Unmask, Update};
use crate::storage::{
    LocalSeedDictAddError,
    MaskScoreIncrError,
    Storage,
    StorageError,
    SumPartAddError,
};

/// Error returned when the state machine fails to handle a request
#[derive(Debug, Error)]
pub enum RequestError {
    /// the message was rejected
    #[error("the message was rejected")]
    MessageRejected,

    /// the message was discarded
    #[error("the message was discarded")]
    MessageDiscarded,

    /// the model or scalar sent by the participant could not be aggregated
    #[error("invalid update: the model or scalar sent by the participant could not be aggregated")]
    AggregationFailed,

    /// the request could not be processed due to an internal error
    #[error("the request could not be processed due to an internal error: {0}")]
    InternalError(&'static str),

    /// a storage request failed
    #[error("storage request failed: {0}")]
    CoordinatorStorage(#[from] StorageError),

    /// adding a local seed dict to the seed dictionary failed
    #[error(transparent)]
    LocalSeedDictAdd(#[from] LocalSeedDictAddError),

    /// adding a sum participant to the sum dictionary failed
    #[error(transparent)]
    SumPartAdd(#[from] SumPartAddError),

    /// incrementing a mask score failed
    #[error(transparent)]
    MaskScoreIncr(#[from] MaskScoreIncrError),
}

pub type StateMachineResult = Result<(), RequestError>;

/// The state machine with all its states.
#[derive(From)]
pub enum StateMachine<S>
where
    S: Storage,
{
    Idle(PhaseState<Idle, S>),
    Sum(PhaseState<Sum, S>),
    Update(PhaseState<Update, S>),
    Sum2(PhaseState<Sum2, S>),
    Unmask(PhaseState<Unmask, S>),
    Error(PhaseState<PhaseStateError, S>),
    Shutdown(PhaseState<Shutdown, S>),
}

impl<S> StateMachine<S>
where
    PhaseState<Idle, S>: Phase<S>,
    PhaseState<Sum, S>: Phase<S>,
    PhaseState<Update, S>: Phase<S>,
    PhaseState<Sum2, S>: Phase<S>,
    PhaseState<Unmask, S>: Phase<S>,
    PhaseState<PhaseStateError, S>: Phase<S>,
    PhaseState<Shutdown, S>: Phase<S>,
    S: Storage,
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
    ///
    /// [`RequestSender`]: crate::state_machine::requests::RequestSender
    pub async fn run(mut self) -> Option<()> {
        loop {
            self = self.next().await?;
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
