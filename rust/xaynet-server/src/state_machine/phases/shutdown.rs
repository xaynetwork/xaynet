use async_trait::async_trait;

use crate::{
    state_machine::{
        phases::{Phase, PhaseName, PhaseState, Shared},
        PhaseStateError,
        StateMachine,
    },
    storage::{CoordinatorStorage, ModelStorage},
};

/// Shutdown state
#[derive(Debug)]
pub struct Shutdown;

#[async_trait]
impl<C, M> Phase<C, M> for PhaseState<Shutdown, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
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

    fn next(self) -> Option<StateMachine<C, M>> {
        None
    }
}

impl<C, M> PhaseState<Shutdown, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Creates a new shutdown state.
    pub fn new(shared: Shared<C, M>) -> Self {
        Self {
            private: Shutdown,
            shared,
        }
    }
}
