//! Mask seed and mask generation.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]:  crate::mask

use std::iter;

use derive_more::{AsMut, AsRef};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::box_;
use thiserror::Error;

use crate::{
    crypto::{encrypt::SEALBYTES, prng::generate_integer, ByteObject},
    mask::{
        object::{MaskObject, MaskUnit, MaskVect},
        MaskConfigPair,
    },
    SumParticipantEphemeralPublicKey,
    SumParticipantEphemeralSecretKey,
};

#[derive(AsRef, AsMut, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// A seed to generate a mask.
///
/// When this goes out of scope, its contents will be zeroed out.
pub struct MaskSeed(box_::Seed);

impl ByteObject for MaskSeed {
    const LENGTH: usize = box_::SEEDBYTES;

    fn from_slice(bytes: &[u8]) -> Option<Self> {
        box_::Seed::from_slice(bytes).map(Self)
    }

    fn zeroed() -> Self {
        Self(box_::Seed([0_u8; Self::LENGTH]))
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl MaskSeed {
    /// Gets this seed as an array.
    pub fn as_array(&self) -> [u8; Self::LENGTH] {
        (self.0).0
    }

    /// Encrypts this seed with the given public key as an [`EncryptedMaskSeed`].
    pub fn encrypt(&self, pk: &SumParticipantEphemeralPublicKey) -> EncryptedMaskSeed {
        // safe unwrap: length of slice is guaranteed by constants
        EncryptedMaskSeed::from_slice_unchecked(pk.encrypt(self.as_slice()).as_slice())
    }

    /// Derives a mask of given length from this seed wrt the masking configurations.
    pub fn derive_mask(&self, len: usize, config: MaskConfigPair) -> MaskObject {
        let MaskConfigPair {
            vect: config_n,
            unit: config_1,
        } = config;
        let mut prng = ChaCha20Rng::from_seed(self.as_array());

        let rand_int = generate_integer(&mut prng, &config_1.order());
        let scalar_mask = MaskUnit::new_unchecked(config_1, rand_int);

        let order_n = config_n.order();
        let rand_ints = iter::repeat_with(|| generate_integer(&mut prng, &order_n))
            .take(len)
            .collect();
        let model_mask = MaskVect::new_unchecked(config_n, rand_ints);

        MaskObject::new_unchecked(model_mask, scalar_mask)
    }
}

#[derive(AsRef, AsMut, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// An encrypted mask seed.
pub struct EncryptedMaskSeed(Vec<u8>);

impl From<Vec<u8>> for EncryptedMaskSeed {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl ByteObject for EncryptedMaskSeed {
    const LENGTH: usize = SEALBYTES + MaskSeed::LENGTH;

    fn from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == Self::LENGTH {
            Some(Self(bytes.to_vec()))
        } else {
            None
        }
    }

    fn zeroed() -> Self {
        Self(vec![0_u8; Self::LENGTH])
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[derive(Debug, Error)]
pub enum InvalidMaskSeed {
    #[error("the encrypted mask seed could not be decrypted")]
    DecryptionFailed,
    #[error("the mask seed has an invalid length")]
    InvalidLength,
}

impl EncryptedMaskSeed {
    /// Decrypts this seed as a [`MaskSeed`].
    ///
    /// # Errors
    /// Fails if the decryption fails.
    pub fn decrypt(
        &self,
        pk: &SumParticipantEphemeralPublicKey,
        sk: &SumParticipantEphemeralSecretKey,
    ) -> Result<MaskSeed, InvalidMaskSeed> {
        MaskSeed::from_slice(
            sk.decrypt(self.as_slice(), pk)
                .or(Err(InvalidMaskSeed::DecryptionFailed))?
                .as_slice(),
        )
        .ok_or(InvalidMaskSeed::InvalidLength)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        crypto::encrypt::EncryptKeyPair,
        mask::config::{BoundType, DataType, GroupType, MaskConfig, ModelType},
    };

    #[test]
    fn test_constants() {
        assert_eq!(MaskSeed::LENGTH, 32);
        assert_eq!(
            MaskSeed::zeroed().as_slice(),
            [0_u8; 32].to_vec().as_slice(),
        );
        assert_eq!(EncryptedMaskSeed::LENGTH, 80);
        assert_eq!(
            EncryptedMaskSeed::zeroed().as_slice(),
            [0_u8; 80].to_vec().as_slice(),
        );
    }

    #[test]
    fn test_derive_mask() {
        let config = MaskConfig {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };
        let seed = MaskSeed::generate();
        let mask = seed.derive_mask(10, config.into());
        assert_eq!(mask.vect.data.len(), 10);
        assert!(mask
            .vect
            .data
            .iter()
            .all(|integer| integer < &config.order()));
    }

    #[test]
    fn test_encryption() {
        let seed = MaskSeed::generate();
        assert_eq!(seed.as_slice().len(), 32);
        assert_ne!(seed, MaskSeed::zeroed());
        let EncryptKeyPair { public, secret } = EncryptKeyPair::generate();
        let encr_seed = seed.encrypt(&public);
        assert_eq!(encr_seed.as_slice().len(), 80);
        let decr_seed = encr_seed.decrypt(&public, &secret).unwrap();
        assert_eq!(seed, decr_seed);
    }
}
