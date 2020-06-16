use super::{idle::Idle, CoordinatorState, PhaseState, Request, StateError, StateMachine};
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

    pub async fn next(self) -> StateMachine {
        error!("state transition failed! error: {:?}", self.inner);
        if let StateError::ChannelError(e) = self.inner {
            panic!(e)
        };

        PhaseState::<Idle>::new(self.coordinator_state, self.request_rx).into()
    }
}
