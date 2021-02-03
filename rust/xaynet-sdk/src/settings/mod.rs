mod max_message_size;

use serde::{Deserialize, Serialize};

pub use max_message_size::{InvalidMaxMessageSize, MaxMessageSize, MIN_MESSAGE_SIZE};
use xaynet_core::{crypto::SigningKeyPair, mask::Scalar};

#[derive(Serialize, Deserialize, Debug)]
pub struct PetSettings {
    pub keys: SigningKeyPair,
    pub scalar: Scalar,
    pub max_message_size: MaxMessageSize,
}

impl PetSettings {
    pub fn new(keys: SigningKeyPair) -> Self {
        PetSettings {
            keys,
            scalar: Scalar::unit(),
            max_message_size: MaxMessageSize::default(),
        }
    }
}
