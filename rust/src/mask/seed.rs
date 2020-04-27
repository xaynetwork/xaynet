use std::{
    convert::{TryFrom, TryInto},
    iter,
};

use derive_more::{AsMut, AsRef};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sodiumoxide::{crypto::box_, randombytes::randombytes};

use crate::{
    crypto::SEALBYTES,
    mask::{config::MaskConfig, Mask},
    utils::generate_integer,
    PetError,
    SumParticipantEphemeralPublicKey,
    SumParticipantEphemeralSecretKey,
};

#[derive(AsRef, AsMut, Clone, Debug, PartialEq)]
pub struct MaskSeed(box_::Seed);

impl TryFrom<Vec<u8>> for MaskSeed {
    type Error = PetError;

    /// Create a mask seed from bytes. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(
            box_::Seed::from_slice(bytes.as_slice()).ok_or(Self::Error::InvalidMessage)?,
        ))
    }
}

impl TryFrom<&[u8]> for MaskSeed {
    type Error = PetError;

    /// Create a mask seed from a slice of bytes. Fails if the length of the input is invalid.
    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(
            box_::Seed::from_slice(slice).ok_or(Self::Error::InvalidMessage)?,
        ))
    }
}

impl MaskSeed {
    pub const BYTES: usize = box_::SEEDBYTES;

    /// Generate a random mask seed.
    pub fn generate() -> Self {
        // safe unwrap: length of slice is guaranteed by constants
        Self(box_::Seed::from_slice(&randombytes(Self::BYTES)).unwrap())
    }

    /// Get the mask seed as a slice.
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    /// Get the mask seed as an array.
    pub fn as_array(&self) -> [u8; Self::BYTES] {
        (self.0).0
    }

    /// Encrypt the mask seed.
    pub fn encrypt(&self, pk: &SumParticipantEphemeralPublicKey) -> EncryptedMaskSeed {
        // safe unwrap: length of slice is guaranteed by constants
        pk.encrypt(self.as_slice()).try_into().unwrap()
    }

    /// Derive a mask of given length from the seed wrt the mask configuration.
    pub fn derive_mask(&self, len: usize, config: &MaskConfig) -> Mask {
        let mut prng = ChaCha20Rng::from_seed(self.as_array());
        let integers = iter::repeat_with(|| generate_integer(&mut prng, config.order()))
            .take(len)
            .collect();
        Mask {
            integers,
            config: config.clone(),
        }
    }
}

#[derive(AsRef, AsMut, Clone, Debug, PartialEq, Serialize, Deserialize)]
/// An encrypted mask seed.
pub struct EncryptedMaskSeed(Vec<u8>);

impl TryFrom<Vec<u8>> for EncryptedMaskSeed {
    type Error = PetError;

    /// Create an encrypted mask seed from bytes. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() == Self::BYTES {
            Ok(Self(bytes))
        } else {
            Err(Self::Error::InvalidMessage)
        }
    }
}

impl TryFrom<&[u8]> for EncryptedMaskSeed {
    type Error = PetError;

    /// Create an encrypted mask seed from a slice of bytes. Fails if the length of the input is
    /// invalid.
    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() == Self::BYTES {
            Ok(Self(slice.to_vec()))
        } else {
            Err(Self::Error::InvalidMessage)
        }
    }
}

impl EncryptedMaskSeed {
    pub const BYTES: usize = SEALBYTES + MaskSeed::BYTES;

    /// Get the encrypted mask seed as a slice.
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Decrypt an encrypted mask seed. Fails if the decryption fails.
    pub fn decrypt(
        &self,
        pk: &SumParticipantEphemeralPublicKey,
        sk: &SumParticipantEphemeralSecretKey,
    ) -> Result<MaskSeed, PetError> {
        MaskSeed::try_from(
            sk.decrypt(self.as_slice(), pk)
                .or(Err(PetError::InvalidMessage))?,
        )
    }
}
