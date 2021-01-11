use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{
    state_machine::{IntoPhase, Phase, PhaseIo, SendingSum, State, Step, Sum2, TransitionOutcome},
    MessageEncoder,
};
use xaynet_core::{
    crypto::{EncryptKeyPair, Signature},
    message::Sum as SumMessage,
};

use super::Awaiting;

/// The state of the sum phase.
#[derive(Serialize, Deserialize, Debug)]
pub struct Sum {
    /// The sum participant ephemeral keys. They are used to decrypt
    /// the encrypted mask seeds.
    pub ephm_keys: EncryptKeyPair,
    /// Signature that proves that the participant has been selected
    /// for the sum task.
    pub sum_signature: Signature,
}

impl Sum {
    /// Creates a new sum state.
    pub fn new(sum_signature: Signature) -> Self {
        Sum {
            ephm_keys: EncryptKeyPair::generate(),
            sum_signature,
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
        let sending: Phase<SendingSum> = self.into();
        TransitionOutcome::Complete(sending.into())
    }
}

impl From<Phase<Sum>> for Phase<SendingSum> {
    fn from(sum: Phase<Sum>) -> Self {
        debug!("composing sum message");
        let message = sum.compose_message();

        debug!("going to sending phase");
        let sum2 = Sum2::new(sum.state.private.ephm_keys, sum.state.private.sum_signature);
        let sending = Box::new(SendingSum::new(message, sum2));
        let state = State::new(sum.state.shared, sending);
        state.into_phase(sum.io)
    }
}

impl From<Phase<Sum>> for Phase<Awaiting> {
    fn from(sum: Phase<Sum>) -> Self {
        State::new(sum.state.shared, Box::new(Awaiting)).into_phase(sum.io)
    }
}

impl Phase<Sum> {
    /// Creates and encodes the sum message from the sum state.
    pub fn compose_message(&self) -> MessageEncoder {
        let sum = SumMessage {
            sum_signature: self.state.private.sum_signature,
            ephm_pk: self.state.private.ephm_keys.public,
        };
        self.message_encoder(sum.into())
    }
}
