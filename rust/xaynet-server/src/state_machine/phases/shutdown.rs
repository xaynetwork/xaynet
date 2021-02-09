use async_trait::async_trait;

use crate::{
    state_machine::{
        phases::{Phase, PhaseError, PhaseName, PhaseState, Shared},
        StateMachine,
    },
    storage::Storage,
};

/// The shutdown state.
#[derive(Debug)]
pub struct Shutdown;

#[async_trait]
impl<T> Phase<T> for PhaseState<Shutdown, T>
where
    T: Storage,
{
    const NAME: PhaseName = PhaseName::Shutdown;

    async fn process(&mut self) -> Result<(), PhaseError> {
        // clear the request channel
        self.shared.request_rx.close();
        while self.shared.request_rx.recv().await.is_some() {}

        Ok(())
    }

    async fn next(self) -> Option<StateMachine<T>> {
        None
    }
}

impl<T> PhaseState<Shutdown, T> {
    /// Creates a new shutdown state.
    pub fn new(shared: Shared<T>) -> Self {
        Self {
            private: Shutdown,
            shared,
        }
    }
}
