//! Provides the logic and functionality for a participant of the PET protocol.
//!
//! See the [client module] documentation since this is a private module anyways.
//!
//! [client module]: ../index.html
use derive_more::From;
use serde::de::{Deserialize, Deserializer, Error as SerdeError};
use thiserror::Error;
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

pub use xaynet_core::message::MESSAGE_HEADER_LENGTH;
/// The minimum message payload size
pub const MINIMUM_PAYLOAD_SIZE: usize = 1;
/// Length of the encryption header in encrypted messages
pub const ENCRYPTION_HEADER_LENGTH: usize = xaynet_core::crypto::SEALBYTES;
/// The minimum size a message can have
pub const MIN_MESSAGE_SIZE: usize =
    MESSAGE_HEADER_LENGTH + ENCRYPTION_HEADER_LENGTH + MINIMUM_PAYLOAD_SIZE;

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
    pub max_message_size: MaxMessageSize,
}

#[derive(Serialize, Deserialize)]
pub struct ParticipantSettings {
    pub secret_key: ParticipantSecretKey,
    pub aggregation_config: AggregationConfig,
    pub max_message_size: MaxMessageSize,
}

#[derive(Debug, Error)]
#[error("max message size must be at least {}", MIN_MESSAGE_SIZE)]
pub struct InvalidMaxMessageSize;

/// Represent the maximum size messages sent by a participant can
/// have. If a larger message needs to be sent, it will be chunked and
/// sent in several parts. Note that messages have a minimal size of
/// [`MIN_MESSAGE_SIZE`].
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct MaxMessageSize(
    #[serde(deserialize_with = "deserialize_max_message_size")] Option<usize>,
);

impl Default for MaxMessageSize {
    fn default() -> Self {
        MaxMessageSize(Some(
            4096 - MESSAGE_HEADER_LENGTH - ENCRYPTION_HEADER_LENGTH,
        ))
    }
}

impl MaxMessageSize {
    /// An arbitrary large maximum message size. With this setting,
    /// messages will never be split.
    pub fn unlimited() -> Self {
        MaxMessageSize(None)
    }

    /// Create a max message size of `size`.
    ///
    /// # Errors
    ///
    /// This method returns an [`InvalidMaxMessageSize`] error if
    /// `size` is smaller than [`MIN_MESSAGE_SIZE`];
    pub fn capped(size: usize) -> Result<Self, InvalidMaxMessageSize> {
        if size >= MIN_MESSAGE_SIZE {
            Ok(MaxMessageSize(Some(size)))
        } else {
            Err(InvalidMaxMessageSize)
        }
    }

    /// Get the maximum payload size corresponding to the maximum
    /// message size. `None` means that the payload size is unlimited.
    pub fn max_payload_size(&self) -> Option<usize> {
        self.0
            .map(|size| size - MESSAGE_HEADER_LENGTH - ENCRYPTION_HEADER_LENGTH)
    }
}

fn deserialize_max_message_size<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<usize> = Option::deserialize(deserializer)?;
    match value {
        Some(size) => {
            if size >= MIN_MESSAGE_SIZE {
                Ok(Some(size))
            } else {
                Err(SerdeError::custom(format!(
                    "max_message_size must be at least {} (got {})",
                    MIN_MESSAGE_SIZE, size
                )))
            }
        }
        None => Ok(None),
    }
}

impl From<ParticipantSettings> for ParticipantState {
    fn from(
        ParticipantSettings {
            secret_key,
            aggregation_config,
            max_message_size,
        }: ParticipantSettings,
    ) -> ParticipantState {
        ParticipantState {
            keys: SigningKeyPair {
                public: secret_key.public_key(),
                secret: secret_key,
            },
            aggregation_config,
            max_message_size,
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
    /// Get the maxiumum message size this participant can send
    pub fn max_message_size(&self) -> MaxMessageSize {
        self.state.max_message_size
    }

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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn max_message_size_deserialization_capped() {
        let input = r#"{"some":1000}"#;
        let expected = json!({"some": MaxMessageSize::capped(1000).unwrap()});
        let actual: serde_json::Value = serde_json::from_str(input).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn max_message_size_deserialization_unlimited() {
        let input = r#"{"none":null}"#;
        let expected = json!({ "none": MaxMessageSize::unlimited() });
        let actual: serde_json::Value = serde_json::from_str(input).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn max_message_size_deserialization_err() {
        // Use a dummy struct, otherwise, serde deserializes the value
        // as an integer.
        #[derive(Deserialize, Serialize, Debug)]
        struct Dummy {
            mms: MaxMessageSize,
        }
        let input = r#"{"mms":123}"#;
        let expected =
            "max_message_size must be at least 185 (got 123) at line 1 column 11".to_string();
        let actual = serde_json::from_str::<Dummy>(input).unwrap_err();
        assert_eq!(expected, format!("{}", actual));
    }

    #[test]
    fn max_message_size_serialization_capped() {
        let input = json!({"some": MaxMessageSize::capped(1000).unwrap()});
        let expected = r#"{"some":1000}"#;
        let actual = serde_json::to_string(&input).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn max_message_size_serialization_unlimited() {
        let input = json!({ "none": MaxMessageSize::unlimited() });
        let expected = r#"{"none":null}"#;
        let actual = serde_json::to_string(&input).unwrap();
        assert_eq!(expected, actual);
    }
}
