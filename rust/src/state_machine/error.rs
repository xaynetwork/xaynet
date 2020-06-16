use super::{idle::Idle, CoordinatorState, PhaseState, Request, StateError, StateMachine};
use tokio::sync::mpsc;

impl PhaseState<StateError> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
        error: StateError,
    ) -> StateMachine {
        info!("state transition");
        StateMachine::Error(Self {
            inner: error,
            coordinator_state,
            request_rx,
        })
    }

    pub async fn next(self) -> StateMachine {
        error!("state transition failed! error: {:?}", self.inner);
        if let StateError::ChannelError(e) = self.inner {
            panic!(e)
        };

        PhaseState::<Idle>::new(self.coordinator_state, self.request_rx)
    }
}
