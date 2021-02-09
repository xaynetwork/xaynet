use tracing::Span;

use crate::{
    state_machine::{
        events::DictionaryUpdate,
        phases::{self, PhaseState},
        requests::{RequestError, RequestSender},
        StateMachine,
    },
    storage::Storage,
};
use xaynet_core::message::Message;

impl RequestSender {
    pub async fn msg(&self, msg: &Message) -> Result<(), RequestError> {
        self.request(msg.clone().into(), Span::none()).await
    }
}

impl<S> StateMachine<S>
where
    S: Storage,
{
    pub fn is_update(&self) -> bool {
        matches!(self, StateMachine::Update(_))
    }

    pub fn into_update_phase_state(self) -> PhaseState<phases::Update, S> {
        match self {
            StateMachine::Update(state) => state,
            _ => panic!("not in update state"),
        }
    }

    pub fn is_sum(&self) -> bool {
        matches!(self, StateMachine::Sum(_))
    }

    pub fn into_sum_phase_state(self) -> PhaseState<phases::Sum, S> {
        match self {
            StateMachine::Sum(state) => state,
            _ => panic!("not in sum state"),
        }
    }

    pub fn is_sum2(&self) -> bool {
        matches!(self, StateMachine::Sum2(_))
    }

    pub fn into_sum2_phase_state(self) -> PhaseState<phases::Sum2, S> {
        match self {
            StateMachine::Sum2(state) => state,
            _ => panic!("not in sum2 state"),
        }
    }

    pub fn is_idle(&self) -> bool {
        matches!(self, StateMachine::Idle(_))
    }

    pub fn into_idle_phase_state(self) -> PhaseState<phases::Idle, S> {
        match self {
            StateMachine::Idle(state) => state,
            _ => panic!("not in idle state"),
        }
    }

    pub fn is_unmask(&self) -> bool {
        matches!(self, StateMachine::Unmask(_))
    }

    pub fn into_unmask_phase_state(self) -> PhaseState<phases::Unmask, S> {
        match self {
            StateMachine::Unmask(state) => state,
            _ => panic!("not in unmask state"),
        }
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, StateMachine::Failure(_))
    }

    pub fn into_failure_phase_state(self) -> PhaseState<phases::Failure, S> {
        match self {
            StateMachine::Failure(state) => state,
            _ => panic!("not in error state"),
        }
    }

    pub fn is_shutdown(&self) -> bool {
        matches!(self, StateMachine::Shutdown(_))
    }

    pub fn into_shutdown_phase_state(self) -> PhaseState<phases::Shutdown, S> {
        match self {
            StateMachine::Shutdown(state) => state,
            _ => panic!("not in shutdown state"),
        }
    }
}

impl<D> DictionaryUpdate<D> {
    pub fn unwrap(self) -> std::sync::Arc<D> {
        if let DictionaryUpdate::New(inner) = self {
            inner
        } else {
            panic!("DictionaryUpdate::Invalidate");
        }
    }
}
