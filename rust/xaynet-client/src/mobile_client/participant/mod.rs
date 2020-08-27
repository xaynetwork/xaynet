//! Provides the logic and functionality for a participant of the PET protocol.
//!
//! See the [client module] documentation since this is a private module anyways.
//!
//! [client module]: ../index.html
use derive_more::From;
use xaynet_core::{
    certificate::Certificate,
    crypto::SigningKeyPair,
    mask::MaskConfig,
    message::{Message, MessageSeal},
    CoordinatorPublicKey,
    ParticipantSecretKey,
};

pub mod awaiting;
pub mod sum;
pub mod sum2;
pub mod update;

pub use self::{awaiting::Awaiting, sum::Sum, sum2::Sum2, update::Update};

#[derive(Serialize, Deserialize)]
pub struct ParticipantState {
    // credentials
    pub keys: SigningKeyPair,
    // Mask config
    pub mask_config: MaskConfig,
    // Certificate
    pub certificate: Certificate, //(dummy)
}

#[derive(Serialize, Deserialize)]
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
            keys: SigningKeyPair {
                public: secret_key.public_key(),
                secret: secret_key,
            },
            mask_config,
            certificate,
        }
    }
}

#[derive(From)]
pub enum Role {
    Unselected(Participant<Awaiting>),
    Summer(Participant<Sum>),
    Updater(Participant<Update>),
}

#[derive(Serialize, Deserialize)]
pub struct Participant<Task> {
    inner: Task,
    state: ParticipantState,
}

impl<Task> Participant<Task> {
    /// Sign the given message with the participant secret key, and
    /// encrypt the signed message with the given public key.
    pub fn seal_message(&self, pk: &CoordinatorPublicKey, message: &Message) -> Vec<u8> {
        let message_seal = MessageSeal {
            recipient_pk: pk,
            sender_sk: &self.state.keys.secret,
        };
        message_seal.seal(message)
    }

    /// Resets the client.
    pub fn reset(self) -> Participant<Awaiting> {
        Participant::<Awaiting>::new(self.state)
    }
}
