use super::{CoordinatorState, PhaseState, Request, StateMachine};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Shutdown;

impl PhaseState<Shutdown> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
    ) -> Self {
        info!("state transition");
        Self {
            inner: Shutdown,
            coordinator_state,
            request_rx,
        }
    }

    pub async fn next(mut self) -> Option<StateMachine> {
        info!("shutdown state machine");

        // clear the request channel
        self.request_rx.close();
        while let Some(_) = self.request_rx.recv().await {}
        None
    }
}
