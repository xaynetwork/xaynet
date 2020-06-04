use super::{State, StateMachine};
use crate::coordinator_async::idle::Idle;

#[derive(Debug)]
pub struct Error;

impl State<Error> {
    pub async fn next(self) -> StateMachine {
        println!("Error phase!");
        // perform some clean up?
        StateMachine::Idle(State {
            _inner: Idle {},
            coordinator_state: self.coordinator_state,
            message_rx: self.message_rx,
        })
    }
}
