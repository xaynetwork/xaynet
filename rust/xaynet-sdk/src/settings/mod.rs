use xaynet_core::{
    crypto::SigningKeyPair,
    mask::{MaskConfig, MaskConfigPair},
};

mod max_message_size;
pub use max_message_size::MaxMessageSize;

#[derive(Serialize, Deserialize, Debug)]
pub struct PetSettings {
    pub(crate) keys: SigningKeyPair,
    pub(crate) mask_config: MaskConfigPair,
    pub(crate) scalar: f64,
    pub(crate) max_message_size: MaxMessageSize,
}

impl PetSettings {
    pub fn new(keys: SigningKeyPair, mask_config: MaskConfig) -> Self {
        PetSettings {
            keys,
            mask_config: MaskConfigPair::from(mask_config),
            scalar: 1.0,
            max_message_size: MaxMessageSize::default(),
        }
    }
}
