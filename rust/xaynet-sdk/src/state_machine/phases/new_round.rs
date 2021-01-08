use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;
use xaynet_core::crypto::{ByteObject, Signature};

use crate::state_machine::{
    IntoPhase,
    Phase,
    PhaseIo,
    State,
    Step,
    Sum,
    TransitionOutcome,
    Update,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct NewRound;

impl IntoPhase<NewRound> for State<NewRound> {
    fn into_phase(self, mut io: PhaseIo) -> Phase<NewRound> {
        io.notify_new_round();
        Phase::<_>::new(self, io)
    }
}

#[async_trait]
impl Step for Phase<NewRound> {
    async fn step(mut self) -> TransitionOutcome {
        info!("new_round task");

        info!("checking eligibility for sum task");
        let sum_signature = self.sign(b"sum");
        if sum_signature.is_eligible(self.state.shared.round_params.sum) {
            info!("eligible for sum task");
            return TransitionOutcome::Complete(self.into_sum(sum_signature).into());
        }

        info!("not eligible for sum task, checking eligibility for update task");
        let update_signature = self.sign(b"update");
        if update_signature.is_eligible(self.state.shared.round_params.update) {
            info!("eligible for update task");
            return TransitionOutcome::Complete(
                self.into_update(sum_signature, update_signature).into(),
            );
        }

        info!("not eligible for update task, going to sleep until next round");
        TransitionOutcome::Complete(self.into_awaiting().into())
    }
}

impl Phase<NewRound> {
    fn sign(&self, data: &[u8]) -> Signature {
        let sk = &self.state.shared.keys.secret;
        let seed = self.state.shared.round_params.seed.as_slice();
        sk.sign_detached(&[seed, data].concat())
    }

    fn into_sum(self, sum_signature: Signature) -> Phase<Sum> {
        let sum = Box::new(Sum::new(sum_signature));
        let state = State::new(self.state.shared, sum);
        state.into_phase(self.io)
    }

    fn into_update(self, sum_signature: Signature, update_signature: Signature) -> Phase<Update> {
        let update = Box::new(Update::new(sum_signature, update_signature));
        let state = State::new(self.state.shared, update);
        state.into_phase(self.io)
    }
}
