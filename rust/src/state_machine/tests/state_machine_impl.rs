use crate::state_machine::{
    phases::{self, PhaseState},
    StateMachine,
};

impl<T> StateMachine<T> {
    pub fn is_update(&self) -> bool {
        match self {
            StateMachine::Update(_) => true,
            _ => false,
        }
    }

    pub fn into_update_phase_state(self) -> PhaseState<T, phases::Update> {
        match self {
            StateMachine::Update(state) => state,
            _ => panic!("not in update state"),
        }
    }

    pub fn is_sum(&self) -> bool {
        match self {
            StateMachine::Sum(_) => true,
            _ => false,
        }
    }

    pub fn into_sum_phase_state(self) -> PhaseState<T, phases::Sum> {
        match self {
            StateMachine::Sum(state) => state,
            _ => panic!("not in sum state"),
        }
    }

    pub fn is_sum2(&self) -> bool {
        match self {
            StateMachine::Sum2(_) => true,
            _ => false,
        }
    }

    pub fn into_sum2_phase_state(self) -> PhaseState<T, phases::Sum2> {
        match self {
            StateMachine::Sum2(state) => state,
            _ => panic!("not in sum2 state"),
        }
    }

    pub fn is_idle(&self) -> bool {
        match self {
            StateMachine::Idle(_) => true,
            _ => false,
        }
    }

    pub fn into_idle_phase_state(self) -> PhaseState<T, phases::Idle> {
        match self {
            StateMachine::Idle(state) => state,
            _ => panic!("not in idle state"),
        }
    }

    pub fn is_unmask(&self) -> bool {
        match self {
            StateMachine::Unmask(_) => true,
            _ => false,
        }
    }

    pub fn into_unmask_phase_state(self) -> PhaseState<T, phases::Unmask> {
        match self {
            StateMachine::Unmask(state) => state,
            _ => panic!("not in unmask state"),
        }
    }

    pub fn is_error(&self) -> bool {
        match self {
            StateMachine::Error(_) => true,
            _ => false,
        }
    }

    pub fn into_error_phase_state(self) -> PhaseState<T, phases::StateError> {
        match self {
            StateMachine::Error(state) => state,
            _ => panic!("not in error state"),
        }
    }

    pub fn is_shutdown(&self) -> bool {
        match self {
            StateMachine::Shutdown(_) => true,
            _ => false,
        }
    }

    pub fn into_shutdown_phase_state(self) -> PhaseState<T, phases::Shutdown> {
        match self {
            StateMachine::Shutdown(state) => state,
            _ => panic!("not in shutdown state"),
        }
    }
}
