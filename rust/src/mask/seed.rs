use std::iter;

use derive_more::{AsMut, AsRef};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sodiumoxide::{crypto::box_, randombytes::randombytes};

use crate::{
    crypto::{ByteObject, SEALBYTES},
    mask::{config::MaskConfig, Mask, MaskIntegers},
    utils::generate_integer,
    PetError,
    SumParticipantEphemeralPublicKey,
    SumParticipantEphemeralSecretKey,
};

#[derive(AsRef, AsMut, Clone, Debug, PartialEq)]
/// A seed for a mask.
pub struct MaskSeed(box_::Seed);

impl ByteObject for MaskSeed {
    /// Create a mask seed from a slice of bytes. Fails if the length of the input is invalid.
    fn from_slice(bytes: &[u8]) -> Option<Self> {
        box_::Seed::from_slice(bytes).map(Self)
    }

    /// Create a mask seed initialized to zero.
    fn zeroed() -> Self {
        Self(box_::Seed([0_u8; Self::BYTES]))
    }

    /// Get the mask seed as a slice.
    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl MaskSeed {
    /// Get the number of bytes of a mask seed.
    pub const BYTES: usize = box_::SEEDBYTES;

    /// Generate a random mask seed.
    pub fn generate() -> Self {
        // safe unwrap: length of slice is guaranteed by constants
        Self::from_slice_unchecked(randombytes(Self::BYTES).as_slice())
    }

    /// Get the mask seed as an array.
    pub fn as_array(&self) -> [u8; Self::BYTES] {
        (self.0).0
    }

    /// Encrypt the mask seed.
    pub fn encrypt(&self, pk: &SumParticipantEphemeralPublicKey) -> EncryptedMaskSeed {
        // safe unwrap: length of slice is guaranteed by constants
        EncryptedMaskSeed::from_slice_unchecked(pk.encrypt(self.as_slice()).as_slice())
    }

    /// Derive a mask of given length from the seed wrt the mask configuration.
    pub fn derive_mask(&self, len: usize, config: &MaskConfig) -> Mask {
        let mut prng = ChaCha20Rng::from_seed(self.as_array());
        let integers = iter::repeat_with(|| generate_integer(&mut prng, config.order()))
            .take(len)
            .collect();
        // safe unwrap: integer conformity is guaranteed by number generator
        Mask::from_parts(integers, config.clone()).unwrap()
    }
}

#[derive(AsRef, AsMut, Clone, Debug, PartialEq, Serialize, Deserialize)]
/// An encrypted mask seed.
pub struct EncryptedMaskSeed(Vec<u8>);

impl ByteObject for EncryptedMaskSeed {
    /// Create an encrypted mask seed from a slice of bytes. Fails if the length of the input is
    /// invalid.
    fn from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == Self::BYTES {
            Some(Self(bytes.to_vec()))
        } else {
            None
        }
    }

    /// Create an encrypted mask seed initialized to zero.
    fn zeroed() -> Self {
        Self(vec![0_u8; Self::BYTES])
    }

    /// Get the encrypted mask seed as a slice.
    fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl EncryptedMaskSeed {
    /// Get the number of bytes of an encrypted mask seed.
    pub const BYTES: usize = SEALBYTES + MaskSeed::BYTES;

    /// Decrypt an encrypted mask seed. Fails if the decryption fails.
    pub fn decrypt(
        &self,
        pk: &SumParticipantEphemeralPublicKey,
        sk: &SumParticipantEphemeralSecretKey,
    ) -> Result<MaskSeed, PetError> {
        MaskSeed::from_slice(
            sk.decrypt(self.as_slice(), pk)
                .or(Err(PetError::InvalidMask))?
                .as_slice(),
        )
        .ok_or(PetError::InvalidMask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        crypto::generate_encrypt_key_pair,
        mask::config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
    };

    #[test]
    fn test_constants() {
        assert_eq!(MaskSeed::BYTES, 32);
        assert_eq!(
            MaskSeed::zeroed().as_slice(),
            [0_u8; 32].to_vec().as_slice(),
        );
        assert_eq!(EncryptedMaskSeed::BYTES, 80);
        assert_eq!(
            EncryptedMaskSeed::zeroed().as_slice(),
            [0_u8; 80].to_vec().as_slice(),
        );
    }

    #[test]
    fn test_derive_mask() {
        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        let seed = MaskSeed::generate();
        let mask = seed.derive_mask(10, &config);
        assert_eq!(mask.integers().len(), 10);
        assert!(mask
            .integers()
            .iter()
            .all(|integer| integer < config.order()));
    }

    #[test]
    fn test_encryption() {
        let seed = MaskSeed::generate();
        assert_eq!(seed.as_slice().len(), 32);
        assert_ne!(seed, MaskSeed::zeroed());
        let (pk, sk) = generate_encrypt_key_pair();
        let encr_seed = seed.encrypt(&pk);
        assert_eq!(encr_seed.as_slice().len(), 80);
        let decr_seed = encr_seed.decrypt(&pk, &sk).unwrap();
        assert_eq!(seed, decr_seed);
    }
}
