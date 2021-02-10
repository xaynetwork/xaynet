//! The state machine that controls the execution of the PET protocol.
//!
//! # Overview
//!
//! ![State Machine](https://mermaid.ink/svg/eyJjb2RlIjoic3RhdGVEaWFncmFtXG5cdFsqXSAtLT4gSWRsZVxuXG4gICAgSWRsZSAtLT4gU3VtXG4gICAgU3VtIC0tPiBVcGRhdGVcbiAgICBVcGRhdGUgLS0-IFN1bTJcbiAgICBTdW0yIC0tPiBVbm1hc2tcbiAgICBVbm1hc2sgLS0-IElkbGVcblxuICAgIFN1bSAtLT4gRmFpbHVyZVxuICAgIFVwZGF0ZSAtLT4gRmFpbHVyZVxuICAgIFN1bTIgLS0-IEZhaWx1cmVcbiAgICBVbm1hc2sgLS0-IEZhaWx1cmVcbiAgICBGYWlsdXJlIC0tPiBJZGxlXG4gICAgRmFpbHVyZSAtLT4gU2h1dGRvd25cblxuICAgIFNodXRkb3duIC0tPiBbKl1cbiIsIm1lcm1haWQiOnsidGhlbWUiOiJuZXV0cmFsIn0sInVwZGF0ZUVkaXRvciI6ZmFsc2V9)
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
//! Publishes [`PhaseName::Idle`] and increments the `round_id` by `1`. Invalidates the [`SumDict`],
//! [`SeedDict`], `scalar` and `mask length`. Updates the [`EncryptKeyPair`], `probabilities` for
//! the tasks and the `seed`. Publishes the [`EncryptKeyPair`] and the [`RoundParameters`].
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
//! **Failure**
//!
//! Publishes [`PhaseName::Failure`] and handles [`PhaseError`]s that can occur during the
//! execution of the [`StateMachine`]. In most cases, the error is handled by restarting the round.
//! However, if a [`PhaseError::RequestChannel`] occurs, the [`StateMachine`] will shut down.
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
//! [`PhaseName::Failure`]: crate::state_machine::phases::PhaseName::Failure
//! [`PhaseName::Shutdown`]: crate::state_machine::phases::PhaseName::Shutdown
//! [`PhaseError`]: crate::state_machine::phases::PhaseError
//! [`PhaseError::RequestChannel`]: crate::state_machine::phases::PhaseError::RequestChannel
//! [`SumDict`]: xaynet_core::SumDict
//! [`SeedDict`]: xaynet_core::SeedDict
//! [`EncryptKeyPair`]: xaynet_core::crypto::EncryptKeyPair
//! [`RoundParameters`]: xaynet_core::common::RoundParameters
//! [`StateMachineInitializer::init()`]: crate::state_machine::initializer::StateMachineInitializer::init
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

use derive_more::From;

use crate::{
    state_machine::phases::{
        Failure,
        Idle,
        Phase,
        PhaseState,
        Shutdown,
        Sum,
        Sum2,
        Unmask,
        Update,
    },
    storage::Storage,
};

/// The state machine with all its states.
#[derive(From)]
pub enum StateMachine<T> {
    /// The [`Idle`] phase.
    Idle(PhaseState<Idle, T>),
    /// The [`Sum`] phase.
    Sum(PhaseState<Sum, T>),
    /// The [`Update`] phase.
    Update(PhaseState<Update, T>),
    /// The [`Sum2`] phase.
    Sum2(PhaseState<Sum2, T>),
    /// The [`Unmask`] phase.
    Unmask(PhaseState<Unmask, T>),
    /// The [`Failure`] phase.
    Failure(PhaseState<Failure, T>),
    /// The [`Shutdown`] phase.
    Shutdown(PhaseState<Shutdown, T>),
}

impl<T> StateMachine<T>
where
    T: Storage,
    PhaseState<Idle, T>: Phase<T>,
    PhaseState<Sum, T>: Phase<T>,
    PhaseState<Update, T>: Phase<T>,
    PhaseState<Sum2, T>: Phase<T>,
    PhaseState<Unmask, T>: Phase<T>,
    PhaseState<Failure, T>: Phase<T>,
    PhaseState<Shutdown, T>: Phase<T>,
{
    /// Moves the [`StateMachine`] to the next state and consumes the current one.
    ///
    /// Returns the next state or `None` if the [`StateMachine`] reached the state [`Shutdown`].
    pub async fn next(self) -> Option<Self> {
        match self {
            StateMachine::Idle(state) => state.run_phase().await,
            StateMachine::Sum(state) => state.run_phase().await,
            StateMachine::Update(state) => state.run_phase().await,
            StateMachine::Sum2(state) => state.run_phase().await,
            StateMachine::Unmask(state) => state.run_phase().await,
            StateMachine::Failure(state) => state.run_phase().await,
            StateMachine::Shutdown(state) => state.run_phase().await,
        }
    }

    /// Runs the state machine until it shuts down.
    ///
    /// The [`StateMachine`] shuts down once all [`RequestSender`] have been dropped.
    ///
    /// [`RequestSender`]: crate::state_machine::requests::RequestSender
    pub async fn run(mut self) -> Option<()> {
        loop {
            self = self.next().await?;
        }
    }
}

/// Records a message accepted metric.
#[doc(hidden)]
#[macro_export]
macro_rules! accepted {
    ($round_id: expr, $phase: expr $(,)?) => {
        crate::metric!(
            crate::metrics::Measurement::MessageAccepted,
            1,
            ("round_id", $round_id),
            ("phase", $phase as u8),
        );
    };
}

/// Records a message rejected metric.
#[doc(hidden)]
#[macro_export]
macro_rules! rejected {
    ($round_id: expr, $phase: expr $(,)?) => {
        crate::metric!(
            crate::metrics::Measurement::MessageRejected,
            1,
            ("round_id", $round_id),
            ("phase", $phase as u8),
        );
    };
}

/// Records a message discared metric.
#[doc(hidden)]
#[macro_export]
macro_rules! discarded {
    ($round_id: expr, $phase: expr $(,)?) => {
        crate::metric!(
            crate::metrics::Measurement::MessageDiscarded,
            1,
            ("round_id", $round_id),
            ("phase", $phase as u8),
        );
    };
}

#[cfg(test)]
pub(crate) mod tests;
