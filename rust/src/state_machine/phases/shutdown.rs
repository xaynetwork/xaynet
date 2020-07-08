use crate::state_machine::{
    coordinator::CoordinatorState,
    events::PhaseEvent,
    phases::{Phase, PhaseState},
    requests::RequestReceiver,
    StateMachine,
};

/// Shutdown state
#[derive(Debug)]
pub struct Shutdown;

#[async_trait]
impl<R> Phase<R> for PhaseState<R, Shutdown>
where
    R: Send,
{
    /// Shuts down the [`StateMachine`].
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn next(mut self) -> Option<StateMachine<R>> {
        warn!("shutdown state machine");

        info!("broadcasting shutdown phase event");
        self.coordinator_state.events.broadcast_phase(
            self.coordinator_state.round_params.seed.clone(),
            PhaseEvent::Shutdown,
        );

        // clear the request channel
        self.request_rx.close();
        while self.request_rx.recv().await.is_some() {}
        None
    }
}

impl<R> PhaseState<R, Shutdown> {
    /// Creates a new shutdown state.
    pub fn new(coordinator_state: CoordinatorState, request_rx: RequestReceiver<R>) -> Self {
        info!("state transition");
        Self {
            inner: Shutdown,
            coordinator_state,
            request_rx,
        }
    }
}
