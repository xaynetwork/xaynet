use async_trait::async_trait;

use crate::{
    state_machine::{
        phases::{Phase, PhaseName, PhaseState, Shared},
        PhaseStateError,
        StateMachine,
    },
    storage::Storage,
};

/// Shutdown state
#[derive(Debug)]
pub struct Shutdown;

#[async_trait]
impl<S> Phase<S> for PhaseState<Shutdown, S>
where
    S: Storage,
{
    const NAME: PhaseName = PhaseName::Shutdown;

    async fn run(&mut self) -> Result<(), PhaseStateError> {
        <Self as Phase<S>>::process(self).await
    }

    async fn process(&mut self) -> Result<(), PhaseStateError> {
        // clear the request channel
        self.shared.request_rx.close();
        while self.shared.request_rx.recv().await.is_some() {}

        Ok(())
    }

    fn next(self) -> Option<StateMachine<S>> {
        None
    }
}

impl<S> PhaseState<Shutdown, S>
where
    S: Storage,
{
    /// Creates a new shutdown state.
    pub fn new(shared: Shared<S>) -> Self {
        Self {
            private: Shutdown,
            shared,
        }
    }
}
