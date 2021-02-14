//! This module provides utilities to configure a [`Participant`].
//!
//! [`Participant`]: crate::Participant

use std::convert::TryInto;
use thiserror::Error;
use xaynet_core::{
    crypto::SigningKeyPair,
    mask::{FromPrimitive, PrimitiveCastError, Scalar},
};
use xaynet_sdk::settings::{MaxMessageSize, PetSettings};

/// A participant settings
#[derive(Clone, Debug)]
pub struct Settings {
    /// The Xaynet coordinator URL.
    url: Option<String>,
    /// The participant signing keys.
    keys: Option<SigningKeyPair>,
    /// The scalar used for masking.
    scalar: Result<Scalar, PrimitiveCastError<f64>>,
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
            scalar: Ok(Scalar::unit()),
            max_message_size: MaxMessageSize::default(),
        }
    }

    /// Set the participant signing keys
    pub fn set_keys(&mut self, keys: SigningKeyPair) {
        self.keys = Some(keys);
    }

    /// Set the scalar to use for masking
    pub fn set_scalar(&mut self, scalar: f64) {
        self.scalar = Scalar::from_primitive(scalar)
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
        } else if let Err(e) = &self.scalar {
            Err(e.clone().into())
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
    #[error("float not within range of scalar: {0}")]
    OutOfScalarRange(#[from] PrimitiveCastError<f64>),
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
        let scalar = scalar.map_err(SettingsError::OutOfScalarRange)?;

        let pet_settings = PetSettings {
            keys,
            scalar,
            max_message_size,
        };

        Ok((url, pet_settings))
    }
}

#[cfg(test)]
mod tests {
    use inline_c::assert_c;

    #[test]
    fn test_settings_new() {
        (assert_c! {
            #include "xaynet_ffi.h"

            int main() {
                Settings *settings = xaynet_ffi_settings_new();
                xaynet_ffi_settings_destroy(settings);
                return 0;
            }
        })
        .success();
    }

    #[test]
    fn test_settings_set_keys() {
        (assert_c! {
            #include <assert.h>

            #include "xaynet_ffi.h"

            int main() {
                assert(xaynet_ffi_crypto_init() == OK); // "failed to init crypto"
                Settings *settings = xaynet_ffi_settings_new();
                const KeyPair *keys = xaynet_ffi_generate_key_pair();
                int err = xaynet_ffi_settings_set_keys(settings, keys);
                assert(!err); // "failed to set keys"
                xaynet_ffi_forget_key_pair(keys);

                xaynet_ffi_settings_destroy(settings);
                return 0;
            }
        })
        .success();
    }

    #[test]
    fn test_settings_set_url() {
        (assert_c! {
            #include <assert.h>
            #include <string.h>

            #include "xaynet_ffi.h"

            int main() {
                Settings *settings = xaynet_ffi_settings_new();

                int err = xaynet_ffi_settings_set_url(settings, NULL);
                assert(err == ERR_INVALID_URL); // "settings invalid URL should fail"

                char *url = "http://localhost:1234";
                err = xaynet_ffi_settings_set_url(settings, url);
                assert(!err); // "failed to set url"

                char *url2 = strdup(url);
                err = xaynet_ffi_settings_set_url(settings, url2);
                assert(!err); // "failed to set url from allocated string"

                // cleanup
                free(url2);
                xaynet_ffi_settings_destroy(settings);

                return 0;
            }
        })
        .success();
    }

    #[test]
    fn test_settings() {
        (assert_c! {
            #include <assert.h>

            #include "xaynet_ffi.h"

            void with_keys(Settings *settings) {
                const KeyPair *keys = xaynet_ffi_generate_key_pair();
                int err = xaynet_ffi_settings_set_keys(settings, keys);
                assert(!err);
                xaynet_ffi_forget_key_pair(keys);
              }

            void with_url(Settings *settings) {
                int err = xaynet_ffi_settings_set_url(settings, "http://localhost:1234");
                assert(!err);
            }

            int main() {
                Settings *settings = xaynet_ffi_settings_new();
                with_keys(settings);
                int err = xaynet_ffi_check_settings(settings);
                assert(err == ERR_SETTINGS_URL); // "expected missing url error"
                xaynet_ffi_settings_destroy(settings);

                settings = xaynet_ffi_settings_new();
                with_url(settings);
                err = xaynet_ffi_check_settings(settings);
                assert(err == ERR_SETTINGS_KEYS); // "expected missing keys error"
                xaynet_ffi_settings_destroy(settings);

                return 0;
            }
        })
        .success();
    }
}
