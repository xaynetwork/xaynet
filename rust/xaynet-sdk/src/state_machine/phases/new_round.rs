use xaynet_core::crypto::{ByteObject, Signature};

use crate::state_machine::{Phase, State, Step, Sum, TransitionOutcome, Update};

#[derive(Serialize, Deserialize, Debug)]
pub struct NewRound;

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

    fn into_sum(mut self, sum_signature: Signature) -> Phase<Sum> {
        let sum = Sum::new(sum_signature);
        self.io.notify_sum();
        Phase::<Sum>::new(State::new(self.state.shared, sum), self.io)
    }

    fn into_update(
        mut self,
        sum_signature: Signature,
        update_signature: Signature,
    ) -> Phase<Update> {
        let update = Update::new(sum_signature, update_signature);
        self.io.notify_update();
        Phase::<Update>::new(State::new(self.state.shared, update), self.io)
    }
}
