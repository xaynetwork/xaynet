use derive_more::From;

use super::{
    boxed_io,
    Awaiting,
    IntoPhase,
    NewRound,
    Phase,
    SerializableState,
    SharedState,
    State,
    Sum,
    Sum2,
    Update,
};
use crate::{settings::PetSettings, ModelStore, Notify, XaynetClient};

/// Outcome of a state machine transition attempt.
pub enum TransitionOutcome {
    /// Outcome when the state machine cannot make immediate progress. The state machine
    /// is returned unchanged.
    Pending(StateMachine),
    /// Outcome when a transition occured and the state machine was updated.
    Complete(StateMachine),
}

/// PET state machine.
#[derive(From)]
#[allow(clippy::large_enum_variant)]
pub enum StateMachine {
    /// PET state machine in the "new round" phase
    NewRound(Phase<NewRound>),
    /// PET state machine in the "awaiting" phase
    Awaiting(Phase<Awaiting>),
    /// PET state machine in the "sum" phase
    Sum(Phase<Sum>),
    /// PET state machine in the "update" phase
    // FIXME: box this
    Update(Phase<Update>),
    /// PET state machine in the "sum2" phase
    Sum2(Phase<Sum2>),
}

impl StateMachine {
    /// Try to make progress in the PET protocol
    pub async fn transition(self) -> TransitionOutcome {
        match self {
            StateMachine::NewRound(phase) => phase.step().await,
            StateMachine::Awaiting(phase) => phase.step().await,
            StateMachine::Sum(phase) => phase.step().await,
            StateMachine::Update(phase) => phase.step().await,
            StateMachine::Sum2(phase) => phase.step().await,
        }
    }

    /// Convert the state machine into a serializable data structure so
    /// that it can be saved.
    pub fn save(self) -> SerializableState {
        match self {
            StateMachine::NewRound(phase) => phase.state.into(),
            StateMachine::Awaiting(phase) => phase.state.into(),
            StateMachine::Sum(phase) => phase.state.into(),
            StateMachine::Update(phase) => phase.state.into(),
            StateMachine::Sum2(phase) => phase.state.into(),
        }
    }
}

impl StateMachine {
    /// Instantiate a new PET state machine.
    ///
    /// # Args
    ///
    /// - `settings`: PET settings
    /// - `xaynet_client`: a client for communicating with the Xaynet coordinator
    /// - `model_store`: a store from which the trained model can be
    ///   loaded, when the participant is selected for the update task
    /// - `notifier`: a type that the state machine can use to emit notifications
    pub fn new<X, M, N>(
        settings: PetSettings,
        xaynet_client: X,
        model_store: M,
        notifier: N,
    ) -> Self
    where
        X: XaynetClient + Send + 'static,
        M: ModelStore + Send + 'static,
        N: Notify + Send + 'static,
    {
        let io = boxed_io(xaynet_client, model_store, notifier);
        let state = State::new(SharedState::new(settings), Awaiting);
        state.into_phase(io).into()
    }

    /// Restore the PET state machine from the given `state`.
    pub fn restore<X, M, N>(
        state: SerializableState,
        xaynet_client: X,
        model_store: M,
        notifier: N,
    ) -> Self
    where
        X: XaynetClient + Send + 'static,
        M: ModelStore + Send + 'static,
        N: Notify + Send + 'static,
    {
        let io = boxed_io(xaynet_client, model_store, notifier);
        match state {
            SerializableState::NewRound(state) => state.into_phase(io).into(),
            SerializableState::Awaiting(state) => state.into_phase(io).into(),
            SerializableState::Sum(state) => state.into_phase(io).into(),
            SerializableState::Sum2(state) => state.into_phase(io).into(),
            SerializableState::Update(state) => state.into_phase(io).into(),
        }
    }
}
