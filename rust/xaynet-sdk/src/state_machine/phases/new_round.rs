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
        let sum = Sum::new(sum_signature);
        let state = State::new(self.state.shared, sum);
        state.into_phase(self.io)
    }

    fn into_update(self, sum_signature: Signature, update_signature: Signature) -> Phase<Update> {
        let update = Update::new(sum_signature, update_signature);
        let state = State::new(self.state.shared, update);
        state.into_phase(self.io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_machine::{
        testutils::{shared_state, SelectFor},
        MockIO,
        StateMachine,
    };

    #[tokio::test]
    async fn test_selected_for_sum() {
        let mut io = MockIO::new();
        io.expect_notify_sum().return_const(());
        let phase = make_phase(SelectFor::Sum, io);

        let outcome = <Phase<NewRound> as Step>::step(phase).await;
        matches!(outcome, TransitionOutcome::Complete(StateMachine::Sum(_)));
    }

    #[tokio::test]
    async fn test_selected_for_update() {
        let mut io = MockIO::new();
        io.expect_notify_update().times(1).return_const(());
        io.expect_notify_load_model().times(1).return_const(());
        let phase = make_phase(SelectFor::Update, io);

        let outcome = <Phase<NewRound> as Step>::step(phase).await;
        matches!(
            outcome,
            TransitionOutcome::Complete(StateMachine::Update(_))
        );
    }

    #[tokio::test]
    async fn test_not_selected() {
        let mut io = MockIO::new();
        io.expect_notify_idle().times(1).return_const(());
        let phase = make_phase(SelectFor::None, io);

        let outcome = <Phase<NewRound> as Step>::step(phase).await;
        matches!(
            outcome,
            TransitionOutcome::Complete(StateMachine::Awaiting(_))
        );
    }

    /// Instantiate a new round phase.
    ///
    /// - `task` is the task we want the simulated participant to be selected for. If you want a
    ///   sum participant, pass `SelectedFor::Sum` for example.
    /// - `io` is the mock the test wants to use. It should contains all the test expectations. The
    ///   reason for settings the mocked IO object in this helper is that once the phase is
    ///   created, `phase.io` is a `Box<dyn IO>`, not a `MockIO`. Therefore, it doesn't have any of
    ///   the mock methods (`expect_xxx()`, `checkpoint()`, etc.) so we cannot set any expectation
    ///   a posteriori
    fn make_phase(task: SelectFor, io: MockIO) -> Phase<NewRound> {
        let shared = shared_state(task);

        // Check IntoPhase<NewRound> implementation
        let mut mock = MockIO::new();
        mock.expect_notify_new_round().times(1).return_const(());
        let mut phase: Phase<NewRound> = State::new(shared, NewRound).into_phase(Box::new(mock));

        // Set `phase.io` to the mock the test wants to use. Note that this drops the `mock` we
        // created above, so the expectations we set on `mock` run now.
        let _ = std::mem::replace(&mut phase.io, Box::new(io));
        phase
    }
}
