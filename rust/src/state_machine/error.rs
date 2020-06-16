use super::{
    idle::Idle,
    shutdown::Shutdown,
    CoordinatorState,
    PhaseState,
    Request,
    StateError,
    StateMachine,
};
use tokio::sync::mpsc;

impl PhaseState<StateError> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
        error: StateError,
    ) -> Self {
        info!("state transition");
        Self {
            inner: error,
            coordinator_state,
            request_rx,
        }
    }

    pub async fn next(self) -> Option<StateMachine> {
        error!("state transition failed! error: {:?}", self.inner);
        let next_state = match self.inner {
            StateError::ChannelError(_) => {
                PhaseState::<Shutdown>::new(self.coordinator_state, self.request_rx).into()
            }
            _ => PhaseState::<Idle>::new(self.coordinator_state, self.request_rx).into(),
        };

        Some(next_state)
    }
}
