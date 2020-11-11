use std::convert::TryInto;
use thiserror::Error;
use xaynet_core::{crypto::SigningKeyPair, mask::MaskConfig};
use xaynet_sdk::settings::{MaxMessageSize, PetSettings};

#[derive(Clone, Debug)]
pub struct Settings {
    mask_config: Option<MaskConfig>,
    keys: Option<SigningKeyPair>,
    url: Option<String>,
    scalar: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    pub fn new() -> Self {
        Self {
            mask_config: None,
            keys: None,
            url: None,
            scalar: 1.0,
        }
    }

    pub fn set_keys(&mut self, keys: SigningKeyPair) {
        self.keys = Some(keys);
    }

    pub fn set_mask_config(&mut self, mask_config: MaskConfig) {
        self.mask_config = Some(mask_config);
    }

    pub fn set_scalar(&mut self, scalar: f64) {
        self.scalar = scalar;
    }

    pub fn set_url(&mut self, url: String) {
        self.url = Some(url);
    }

    pub fn check(&self) -> Result<(), SettingsError> {
        if self.url.is_none() {
            Err(SettingsError::MissingUrl)
        } else if self.mask_config.is_none() {
            Err(SettingsError::MissingMaskConfig)
        } else if self.keys.is_none() {
            Err(SettingsError::MissingKeys)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("the Xaynet coordinator URL must be specified")]
    MissingUrl,
    #[error("the masking configuration must be specified")]
    MissingMaskConfig,
    #[error("the participant signing key pair must be specified")]
    MissingKeys,
}

impl TryInto<(String, PetSettings)> for Settings {
    type Error = SettingsError;

    fn try_into(self) -> Result<(String, PetSettings), Self::Error> {
        let Settings {
            mask_config,
            keys,
            url,
            scalar,
        } = self;

        let url = url.ok_or(SettingsError::MissingUrl)?;

        let keys = keys.ok_or(SettingsError::MissingKeys)?;
        let mask_config = mask_config.ok_or(SettingsError::MissingMaskConfig)?;

        let pet_settings = PetSettings {
            scalar,
            max_message_size: MaxMessageSize::default(),
            mask_config: mask_config.into(),
            keys,
        };

        Ok((url, pet_settings))
    }
}
