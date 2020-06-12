use super::{idle::Idle, CoordinatorState, Request, State, StateError, StateMachine};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Error {
    error: StateError,
}

impl State<Error> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
        error: StateError,
    ) -> StateMachine {
        info!("state transition");
        StateMachine::Error(Self {
            _inner: Error { error },
            coordinator_state,
            request_rx,
        })
    }

    pub async fn next(self) -> StateMachine {
        error!("state transition failed! error: {:?}", self._inner.error);
        if let StateError::ChannelError(e) = self._inner.error {
            panic!(e)
        };

        State::<Idle>::new(self.coordinator_state, self.request_rx)
    }
}
