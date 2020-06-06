use super::{CoordinatorState, State, StateMachine};
use crate::{coordinator_async::idle::Idle, PetError};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Error {
    error: PetError,
}

impl State<Error> {
    pub fn new(
        coordinator_state: CoordinatorState,
        message_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        error: PetError,
    ) -> StateMachine {
        StateMachine::Error(Self {
            _inner: Error { error },
            coordinator_state,
            message_rx,
        })
    }

    pub async fn next(self) -> StateMachine {
        error!("Error phase! Error: {:?}", self._inner.error);
        State::<Idle>::new(self.coordinator_state, self.message_rx)
    }
}
