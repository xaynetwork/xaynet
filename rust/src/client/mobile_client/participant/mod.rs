//! Provides the logic and functionality for a participant of the PET protocol.
//!
//! See the [client module] documentation since this is a private module anyways.
//!
//! [client module]: ../index.html
use crate::{
    certificate::Certificate,
    mask::config::MaskConfig,
    message::message::{MessageOwned, MessageSeal},
    CoordinatorPublicKey,
    ParticipantPublicKey,
    ParticipantSecretKey,
};
use derive_more::From;

pub mod sum;
pub mod sum2;
pub mod undefined;
pub mod update;

pub use self::{sum::Sum, sum2::Sum2, undefined::Undefined, update::Update};

pub struct ParticipantState {
    // credentials
    pub pk: ParticipantPublicKey,
    pub sk: ParticipantSecretKey,
    // Mask config
    pub mask_config: MaskConfig,
    // Certificate
    pub certificate: Certificate, //(dummy)
}

pub struct ParticipantSettings {
    pub secret_key: ParticipantSecretKey,
    pub mask_config: MaskConfig,
    pub certificate: Certificate,
}

impl From<ParticipantSettings> for ParticipantState {
    fn from(
        ParticipantSettings {
            secret_key,
            mask_config,
            certificate,
        }: ParticipantSettings,
    ) -> ParticipantState {
        ParticipantState {
            pk: secret_key.public_key(),
            sk: secret_key,
            mask_config,
            certificate,
        }
    }
}

#[derive(From)]
pub enum Type {
    Unselected(Participant<Undefined>),
    Summer(Participant<Sum>),
    Updater(Participant<Update>),
}

pub struct Participant<Task> {
    inner: Task,
    state: ParticipantState,
}

impl<Task> Participant<Task> {
    /// Sign the given message with the participant secret key, and
    /// encrypt the signed message with the given public key.
    pub fn seal_message(&self, pk: &CoordinatorPublicKey, message: &MessageOwned) -> Vec<u8> {
        let message_seal = MessageSeal {
            recipient_pk: pk,
            sender_sk: &self.state.sk,
        };
        message_seal.seal(message)
    }

    /// Resets the client.
    pub fn reset(self) -> Participant<Undefined> {
        Participant::<Undefined>::new(self.state)
    }
}
