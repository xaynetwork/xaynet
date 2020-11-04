use crate::{
    state_machine::{
        phases::{Phase, PhaseName, PhaseState, Shared},
        PhaseStateError, StateMachine,
    },
    storage::api::Store,
};

/// Shutdown state
#[derive(Debug)]
pub struct Shutdown;

#[async_trait]
impl<St> Phase<St> for PhaseState<Shutdown, St>
where
    St: Store,
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

    fn next(self) -> Option<StateMachine<St>> {
        None
    }
}

impl<St> PhaseState<Shutdown, St>
where
    St: Store,
{
    /// Creates a new shutdown state.
    pub fn new(shared: Shared<St>) -> Self {
        Self {
            inner: Shutdown,
            shared,
        }
    }
}
