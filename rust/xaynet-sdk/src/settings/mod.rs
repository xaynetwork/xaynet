mod max_message_size;

use serde::{Deserialize, Serialize};

pub use max_message_size::{InvalidMaxMessageSize, MaxMessageSize, MIN_MESSAGE_SIZE};
use xaynet_core::crypto::SigningKeyPair;

#[derive(Serialize, Deserialize, Debug)]
pub struct PetSettings {
    pub keys: SigningKeyPair,
    pub scalar: f64,
    pub max_message_size: MaxMessageSize,
}

impl PetSettings {
    pub fn new(keys: SigningKeyPair) -> Self {
        PetSettings {
            keys,
            scalar: 1.0,
            max_message_size: MaxMessageSize::default(),
        }
    }
}
