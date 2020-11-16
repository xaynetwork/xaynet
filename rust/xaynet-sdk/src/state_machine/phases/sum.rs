use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{
    state_machine::{IntoPhase, Phase, PhaseIo, Progress, State, Step, Sum2, TransitionOutcome},
    MessageEncoder,
};
use xaynet_core::{
    crypto::{EncryptKeyPair, Signature},
    message::Sum as SumMessage,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Sum {
    pub ephm_keys: EncryptKeyPair,
    pub sum_signature: Signature,
    pub message: Option<MessageEncoder>,
}

impl Sum {
    pub fn new(sum_signature: Signature) -> Self {
        Sum {
            ephm_keys: EncryptKeyPair::generate(),
            sum_signature,
            message: None,
        }
    }
}

impl IntoPhase<Sum> for State<Sum> {
    fn into_phase(self, mut io: PhaseIo) -> Phase<Sum> {
        io.notify_sum();
        Phase::<_>::new(self, io)
    }
}

#[async_trait]
impl Step for Phase<Sum> {
    async fn step(mut self) -> TransitionOutcome {
        info!("sum task");

        self = try_progress!(self.compose_sum_message());

        // FIXME: currently if sending fails, we lose the message,
        // thus wasting all the work we've done in this phase
        let message = self.state.private.message.take().unwrap();
        match self.send_message(message).await {
            Ok(_) => {
                info!("sent sum message, going to sum2 phase");
                TransitionOutcome::Complete(self.into_sum2().into())
            }
            Err(e) => {
                warn!("failed to send sum message: {}", e);
                warn!("sum phase failed, going back to awaiting phase");
                TransitionOutcome::Complete(self.into_awaiting().into())
            }
        }
    }
}

impl Phase<Sum> {
    pub fn compose_sum_message(mut self) -> Progress<Sum> {
        if self.state.private.message.is_some() {
            return Progress::Continue(self);
        }

        let sum = SumMessage {
            sum_signature: self.state.private.sum_signature,
            ephm_pk: self.state.private.ephm_keys.public,
        };
        self.state.private.message = Some(self.message_encoder(sum.into()));
        Progress::Updated(self.into())
    }

    pub fn into_sum2(self) -> Phase<Sum2> {
        let sum2 = Sum2::new(
            self.state.private.ephm_keys,
            self.state.private.sum_signature,
        );
        let state = State::new(self.state.shared, sum2);
        state.into_phase(self.io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::state_machine::{
        testutils::{shared_state, SelectFor},
        MockIO,
        SharedState,
        StateMachine,
    };
    use thiserror::Error;
    use xaynet_core::crypto::{ByteObject, EncryptKeySeed};

    /// Instantiate a sum phase.
    fn make_phase(io: MockIO) -> Phase<Sum> {
        let shared = shared_state(SelectFor::Sum);
        let sum = make_sum(&shared);

        // Check IntoPhase<Sum> implementation
        let mut mock = MockIO::new();
        mock.expect_notify_sum().times(1).return_const(());
        let mut phase: Phase<Sum> = State::new(shared, sum).into_phase(Box::new(mock));

        // Set `phase.io` to the mock the test wants to use. Note that this drops the `mock` we
        // created above, so the expectations we set on `mock` run now.
        let _ = std::mem::replace(&mut phase.io, Box::new(io));
        phase
    }

    fn make_sum(shared: &SharedState) -> Sum {
        let ephm_keys = EncryptKeyPair::derive_from_seed(&EncryptKeySeed::zeroed());
        let sk = &shared.keys.secret;
        let seed = shared.round_params.seed.as_slice();
        let signature = sk.sign_detached(&[seed, b"sum"].concat());
        Sum {
            ephm_keys,
            sum_signature: signature,
            message: None,
        }
    }

    async fn check_step_1() -> Phase<Sum> {
        let io = MockIO::new();
        let phase = make_phase(io);
        let outcome = <Phase<Sum> as Step>::step(phase).await;
        match outcome {
            TransitionOutcome::Complete(StateMachine::Sum(phase)) => {
                assert!(phase.state.private.message.is_some());
                phase
            }
            _ => panic!("unexpected outcome {:?}", outcome),
        }
    }

    #[tokio::test]
    async fn test_phase() {
        let mut phase = check_step_1().await;

        let mut io = MockIO::new();
        io.expect_send_message().times(1).returning(|_| Ok(()));
        let _ = std::mem::replace(&mut phase.io, Box::new(io));

        let outcome = <Phase<Sum> as Step>::step(phase).await;
        matches!(outcome, TransitionOutcome::Complete(StateMachine::Sum2(_)));
    }

    #[derive(Error, Debug)]
    #[error("error")]
    struct DummyErr;

    #[tokio::test]
    async fn test_send_sum_message_fails() {
        let mut phase = check_step_1().await;

        let mut io = MockIO::new();
        io.expect_send_message()
            .times(1)
            .returning(|_| Err(Box::new(DummyErr)));
        io.expect_notify_idle().times(1).return_const(());
        let _ = std::mem::replace(&mut phase.io, Box::new(io));

        let outcome = <Phase<Sum> as Step>::step(phase).await;
        matches!(
            outcome,
            TransitionOutcome::Complete(StateMachine::Awaiting(_))
        );
    }
}
