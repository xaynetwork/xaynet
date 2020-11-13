use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::state_machine::{IntoPhase, Phase, PhaseIo, State, Step, TransitionOutcome};

#[derive(Serialize, Deserialize, Debug)]
pub struct Awaiting;

#[async_trait]
impl Step for Phase<Awaiting> {
    async fn step(mut self) -> TransitionOutcome {
        info!("awaiting task");
        TransitionOutcome::Pending(self.into())
    }
}

impl IntoPhase<Awaiting> for State<Awaiting> {
    fn into_phase(self, mut io: PhaseIo) -> Phase<Awaiting> {
        io.notify_idle();
        Phase::<_>::new(self, io)
    }
}
