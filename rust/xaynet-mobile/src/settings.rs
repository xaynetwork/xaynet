//! This module provides utilities to configure a [`Participant`].
//!
//! [`Participant`]: crate::Participant

use std::convert::TryInto;
use thiserror::Error;
use xaynet_core::{crypto::SigningKeyPair, mask::Scalar};
use xaynet_sdk::settings::{MaxMessageSize, PetSettings};

/// A participant settings
#[derive(Clone, Debug)]
pub struct Settings {
    /// The Xaynet coordinator URL.
    url: Option<String>,
    /// The participant signing keys.
    keys: Option<SigningKeyPair>,
    /// The scalar used for masking.
    scalar: Scalar,
    /// The maximum possible size of a message.
    max_message_size: MaxMessageSize,
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    /// Create new empty settings.
    pub fn new() -> Self {
        Self {
            url: None,
            keys: None,
            scalar: Scalar::unit(),
            max_message_size: MaxMessageSize::default(),
        }
    }

    /// Set the participant signing keys
    pub fn set_keys(&mut self, keys: SigningKeyPair) {
        self.keys = Some(keys);
    }

    /// Set the scalar to use for masking
    pub fn set_scalar(&mut self, numer: u32, denom: u32) {
        self.scalar = Scalar::new(numer, denom)
    }

    /// Set the Xaynet coordinator address
    pub fn set_url(&mut self, url: String) {
        self.url = Some(url);
    }

    /// Sets the maximum possible size of a message.
    pub fn set_max_message_size(&mut self, size: MaxMessageSize) {
        self.max_message_size = size;
    }

    /// Check whether the settings are complete and valid
    pub fn check(&self) -> Result<(), SettingsError> {
        if self.url.is_none() {
            Err(SettingsError::MissingUrl)
        } else if self.keys.is_none() {
            Err(SettingsError::MissingKeys)
        } else {
            Ok(())
        }
    }
}

/// Error returned when the settings are invalid
#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("the Xaynet coordinator URL must be specified")]
    MissingUrl,
    #[error("the participant signing key pair must be specified")]
    MissingKeys,
}

impl TryInto<(String, PetSettings)> for Settings {
    type Error = SettingsError;

    fn try_into(self) -> Result<(String, PetSettings), Self::Error> {
        let Settings {
            keys,
            url,
            scalar,
            max_message_size,
        } = self;

        let url = url.ok_or(SettingsError::MissingUrl)?;

        let keys = keys.ok_or(SettingsError::MissingKeys)?;

        let pet_settings = PetSettings {
            keys,
            scalar,
            max_message_size,
        };

        Ok((url, pet_settings))
    }
}
