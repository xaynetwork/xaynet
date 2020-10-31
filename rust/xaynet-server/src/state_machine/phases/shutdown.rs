use crate::{
    state_machine::{
        phases::{Phase, PhaseName, PhaseState, Shared},
        PhaseStateError,
        StateMachine,
    },
    storage::api::Storage,
};

/// Shutdown state
#[derive(Debug)]
pub struct Shutdown;

#[async_trait]
impl<Store: Storage> Phase<Store> for PhaseState<Shutdown, Store> {
    const NAME: PhaseName = PhaseName::Shutdown;

    /// Shuts down the [`StateMachine`].
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn run(&mut self) -> Result<(), PhaseStateError> {
        // clear the request channel
        self.shared.request_rx.close();
        while self.shared.request_rx.recv().await.is_some() {}
        Ok(())
    }

    fn next(self) -> Option<StateMachine<Store>> {
        None
    }
}

impl<Store: Storage> PhaseState<Shutdown, Store> {
    /// Creates a new shutdown state.
    pub fn new(shared: Shared<Store>) -> Self {
        Self {
            inner: Shutdown,
            shared,
        }
    }
}
