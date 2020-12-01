use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{
    state_machine::{IntoPhase, Phase, PhaseIo, Sending, State, Step, Sum2, TransitionOutcome},
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
}

impl Sum {
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
        TransitionOutcome::Complete(self.into_sending().into())
    }
}

impl Phase<Sum> {
    pub fn compose_sum_message(&self) -> MessageEncoder {
        let sum = SumMessage {
            sum_signature: self.state.private.sum_signature,
            ephm_pk: self.state.private.ephm_keys.public,
        };
        self.message_encoder(sum.into())
    }

    pub fn into_sending(self) -> Phase<Sending> {
        debug!("composing sum message");
        let message = self.compose_sum_message();

        debug!("going to sending phase");
        let sum2 = Box::new(Sum2::new(
            self.state.private.ephm_keys,
            self.state.private.sum_signature,
        ));
        let sending = Sending::from_sum(message, sum2);
        let state = State::new(self.state.shared, sending);
        state.into_phase(self.io)
    }
}
