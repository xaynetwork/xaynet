use async_trait::async_trait;

use crate::{
    state_machine::{
        phases::{Idle, Phase, PhaseName, PhaseState, Shared},
        PhaseStateError, StateMachine,
    },
    storage::Storage,
};

/// Shutdown state
#[derive(Debug)]
pub struct Init;

#[async_trait]
impl<S> Phase<S> for PhaseState<Init, S>
where
    S: Storage,
{
    const NAME: PhaseName = PhaseName::Init;

    async fn run(&mut self) -> Result<(), PhaseStateError> {
        self.shared.events.broadcast_readiness(true);
        Ok(())
    }

    fn next(self) -> Option<StateMachine<S>> {
        Some(PhaseState::<Idle, _>::new(self.shared).into())
    }
}

impl<S> PhaseState<Init, S>
where
    S: Storage,
{
    /// Creates a new shutdown state.
    pub fn new(shared: Shared<S>) -> Self {
        Self {
            private: Init,
            shared,
        }
    }
}
