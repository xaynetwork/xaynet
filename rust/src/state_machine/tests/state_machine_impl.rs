use crate::state_machine::StateMachine;

impl<T> StateMachine<T> {
    pub fn is_update(&self) -> bool {
        match self {
            StateMachine::Update(_) => true,
            _ => false,
        }
    }

    pub fn is_sum(&self) -> bool {
        match self {
            StateMachine::Sum(_) => true,
            _ => false,
        }
    }

    pub fn is_sum2(&self) -> bool {
        match self {
            StateMachine::Sum2(_) => true,
            _ => false,
        }
    }

    pub fn is_idle(&self) -> bool {
        match self {
            StateMachine::Idle(_) => true,
            _ => false,
        }
    }

    pub fn is_unmask(&self) -> bool {
        match self {
            StateMachine::Unmask(_) => true,
            _ => false,
        }
    }

    pub fn is_error(&self) -> bool {
        match self {
            StateMachine::Error(_) => true,
            _ => false,
        }
    }

    pub fn is_shutdown(&self) -> bool {
        match self {
            StateMachine::Shutdown(_) => true,
            _ => false,
        }
    }
}
