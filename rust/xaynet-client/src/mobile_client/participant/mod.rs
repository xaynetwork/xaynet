//! Provides the logic and functionality for a participant of the PET protocol.
//!
//! See the [client module] documentation since this is a private module anyways.
//!
//! [client module]: ../index.html
use derive_more::From;
use xaynet_core::{
    crypto::SigningKeyPair,
    mask::MaskConfig,
    message::Message,
    CoordinatorPublicKey,
    ParticipantPublicKey,
    ParticipantSecretKey,
};

pub mod awaiting;
pub mod sum;
pub mod sum2;
pub mod update;

pub use self::{awaiting::Awaiting, sum::Sum, sum2::Sum2, update::Update};

#[derive(Serialize, Deserialize)]
pub struct AggregationConfig {
    pub mask: MaskConfig,
    pub scalar: f64,
}

#[derive(Serialize, Deserialize)]
pub struct ParticipantState {
    // credentials
    pub keys: SigningKeyPair,
    // Mask config
    pub aggregation_config: AggregationConfig,
}

#[derive(Serialize, Deserialize)]
pub struct ParticipantSettings {
    pub secret_key: ParticipantSecretKey,
    pub aggregation_config: AggregationConfig,
}

impl From<ParticipantSettings> for ParticipantState {
    fn from(
        ParticipantSettings {
            secret_key,
            aggregation_config,
        }: ParticipantSettings,
    ) -> ParticipantState {
        ParticipantState {
            keys: SigningKeyPair {
                public: secret_key.public_key(),
                secret: secret_key,
            },
            aggregation_config,
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
    /// Get the participant's public signing key
    pub fn public_key(&self) -> ParticipantPublicKey {
        self.state.keys.public
    }

    /// Serialize, sign and encrypt the given message.
    ///
    /// The message is signed with the participant secret signing
    /// key. `pk` is the coordinator public key, used to encrypt the
    /// final message
    pub fn seal_message(&self, pk: &CoordinatorPublicKey, message: &Message) -> Vec<u8> {
        let mut buf = vec![0; message.buffer_length()];
        message.to_bytes(&mut buf, &self.state.keys.secret);
        pk.encrypt(&buf[..])
    }

    /// Serialize and sign given message.
    ///
    /// The message is signed with the participant secret signing key.
    pub fn serialize_message(&self, message: &Message) -> Vec<u8> {
        let mut buf = vec![0; message.buffer_length()];
        message.to_bytes(&mut buf, &self.state.keys.secret);
        buf
    }

    /// Resets the client.
    pub fn reset(self) -> Participant<Awaiting> {
        Participant::<Awaiting>::new(self.state)
    }
}
