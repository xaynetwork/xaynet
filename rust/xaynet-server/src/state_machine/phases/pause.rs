use crate::{
    state_machine::{
        events::DictionaryUpdate,
        phases::{Idle, Phase, PhaseName, PhaseState, Shared},
        PhaseStateError, StateMachine,
    },
    storage::Storage,
};
use async_trait::async_trait;
use tracing::{debug, error_span, info, warn};

#[derive(Debug)]
pub struct Pause;

#[async_trait]
impl<S> Phase<S> for PhaseState<Pause, S>
where
    S: Storage,
{
    const NAME: PhaseName = PhaseName::Pause;

    // only provide round_param and global_model
    async fn run(&mut self) -> Result<(), PhaseStateError> {
        info!("broadcasting invalidation of sum dictionary from previous round");
        self.shared
            .events
            .broadcast_sum_dict(DictionaryUpdate::Invalidate);

        info!("broadcasting invalidation of seed dictionary from previous round");
        self.shared
            .events
            .broadcast_seed_dict(DictionaryUpdate::Invalidate);

            
        Ok(())
    }

    fn next(self) -> Option<StateMachine<S>> {
        Some(PhaseState::<Idle, _>::new(self.shared).into())
    }
}

impl<S> PhaseState<Pause, S>
where
    S: Storage,
{
    pub fn new(shared: Shared<S>) -> Self {
        Self {
            private: Pause,
            shared,
        }
    }
}
