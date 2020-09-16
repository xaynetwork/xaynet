use super::{Participant, ParticipantState};
use crate::mobile_client::participant::Sum2;
use xaynet_core::{
    crypto::EncryptKeyPair,
    message::{Message, Sum as SumMessage},
    CoordinatorPublicKey,
    ParticipantTaskSignature,
    SumParticipantEphemeralPublicKey,
    SumParticipantEphemeralSecretKey,
};
#[derive(Serialize, Deserialize, Clone)]
pub struct Sum {
    ephm_pk: SumParticipantEphemeralPublicKey,
    ephm_sk: SumParticipantEphemeralSecretKey,
    sum_signature: ParticipantTaskSignature,
}

impl Participant<Sum> {
    pub fn new(state: ParticipantState, sum_signature: ParticipantTaskSignature) -> Self {
        // Generate an ephemeral encryption key pair.
        let EncryptKeyPair { public, secret } = EncryptKeyPair::generate();
        Self {
            inner: Sum {
                ephm_pk: public,
                ephm_sk: secret,
                sum_signature,
            },
            state,
        }
    }

    /// Compose a sum message given the coordinator public key.
    pub fn compose_sum_message(&mut self, coordinator_pk: CoordinatorPublicKey) -> Message {
        Message::new_sum(
            self.state.keys.public,
            coordinator_pk,
            SumMessage {
                sum_signature: self.inner.sum_signature,
                ephm_pk: self.inner.ephm_pk,
            },
        )
    }
}

impl Into<Participant<Sum2>> for Participant<Sum> {
    fn into(self) -> Participant<Sum2> {
        Participant::<Sum2>::new(
            self.state,
            self.inner.sum_signature,
            self.inner.ephm_pk,
            self.inner.ephm_sk,
        )
    }
}
