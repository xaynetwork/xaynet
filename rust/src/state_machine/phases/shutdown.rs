use crate::state_machine::{
    coordinator::CoordinatorState,
    phases::{Phase, PhaseName, PhaseState},
    requests::RequestReceiver,
    StateError,
    StateMachine,
};

/// Shutdown state
#[derive(Debug)]
pub struct Shutdown;

#[async_trait]
impl Phase for PhaseState<Shutdown> {
    const NAME: PhaseName = PhaseName::Shutdown;

    /// Shuts down the [`StateMachine`].
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn run(&mut self) -> Result<(), StateError> {
        // clear the request channel
        self.request_rx.close();
        while self.request_rx.recv().await.is_some() {}
        Ok(())
    }

    fn next(self) -> Option<StateMachine> {
        None
    }
}

impl PhaseState<Shutdown> {
    /// Creates a new shutdown state.
    pub fn new(coordinator_state: CoordinatorState, request_rx: RequestReceiver) -> Self {
        info!("state transition");
        Self {
            inner: Shutdown,
            coordinator_state,
            request_rx,
        }
    }
}
