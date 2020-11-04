use crate::{
    state_machine::{
        phases::{Phase, PhaseName, PhaseState, Shared},
        PhaseStateError,
        StateMachine,
    },
    storage::api::{PersistentStorage, VolatileStorage},
};

/// Shutdown state
#[derive(Debug)]
pub struct Shutdown;

#[async_trait]
impl<V, P> Phase<V, P> for PhaseState<Shutdown, V, P>
where
    V: VolatileStorage,
    P: PersistentStorage,
{
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

    fn next(self) -> Option<StateMachine<V, P>> {
        None
    }
}

impl<V, P> PhaseState<Shutdown, V, P>
where
    V: VolatileStorage,
    P: PersistentStorage,
{
    /// Creates a new shutdown state.
    pub fn new(shared: Shared<V, P>) -> Self {
        Self {
            inner: Shutdown,
            shared,
        }
    }
}
