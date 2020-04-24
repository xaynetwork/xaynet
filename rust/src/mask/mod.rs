pub mod config;

use std::convert::TryFrom;

use sodiumoxide::{crypto::sealedbox, randombytes::randombytes};

use crate::{PetError, SumParticipantEphemeralPublicKey, SumParticipantEphemeralSecretKey};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// A mask seed.
pub struct MaskSeed(Vec<u8>);

impl MaskSeed {
    pub const BYTES: usize = 32;

    #[allow(clippy::new_without_default)]
    /// Create a mask seed.
    pub fn new() -> Self {
        Self(randombytes(Self::BYTES))
    }

    /// Encrypt a mask seed.
    pub fn seal(&self, pk: &SumParticipantEphemeralPublicKey) -> EncrMaskSeed {
        // safe unwrap: length of slice is guaranteed by constants
        EncrMaskSeed::try_from(sealedbox::seal(self.as_ref(), pk)).unwrap()
    }
}

impl AsRef<[u8]> for MaskSeed {
    /// Get a reference to the mask seed.
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl TryFrom<Vec<u8>> for MaskSeed {
    type Error = PetError;

    /// Create a mask seed from bytes. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() == Self::BYTES {
            Ok(Self(bytes))
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

impl TryFrom<&[u8]> for MaskSeed {
    type Error = PetError;

    /// Create a mask seed from a slice of bytes. Fails if the length of the input is invalid.
    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() == Self::BYTES {
            Ok(Self(slice.to_vec()))
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// An encrypted mask seed.
pub struct EncrMaskSeed(Vec<u8>);

impl EncrMaskSeed {
    pub const BYTES: usize = sealedbox::SEALBYTES + MaskSeed::BYTES;

    /// Decrypt an encrypted mask seed. Fails if the decryption fails.
    pub fn open(
        &self,
        pk: &SumParticipantEphemeralPublicKey,
        sk: &SumParticipantEphemeralSecretKey,
    ) -> Result<MaskSeed, PetError> {
        MaskSeed::try_from(
            sealedbox::open(self.as_ref(), pk, sk).or(Err(PetError::InvalidMessage))?,
        )
    }
}

impl AsRef<[u8]> for EncrMaskSeed {
    /// Get a reference to the encrypted mask seed.
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl TryFrom<Vec<u8>> for EncrMaskSeed {
    type Error = PetError;

    /// Create an encrypted mask seed from bytes. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() == Self::BYTES {
            Ok(Self(bytes))
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

impl TryFrom<&[u8]> for EncrMaskSeed {
    type Error = PetError;

    /// Create an encrypted mask seed from a slice of bytes. Fails if the length of the input is
    /// invalid.
    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() == Self::BYTES {
            Ok(Self(slice.to_vec()))
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A mask.
pub struct Mask(Vec<u8>);

#[allow(clippy::len_without_is_empty)]
impl Mask {
    /// Get the length of the mask.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl AsRef<[u8]> for Mask {
    /// Get a reference to the mask.
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for Mask {
    /// Create a mask from bytes.
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for Mask {
    /// Create a mask from a slice of bytes.
    fn from(slice: &[u8]) -> Self {
        Self(slice.to_vec())
    }
}

#[derive(Debug, PartialEq)]
/// A masked model.
pub struct MaskedModel(Vec<u8>);

#[allow(clippy::len_without_is_empty)]
impl MaskedModel {
    /// Get the length of the masked model.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl AsRef<[u8]> for MaskedModel {
    /// Get a reference to the masked model.
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for MaskedModel {
    /// Create a masked model from bytes.
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for MaskedModel {
    /// Create a masked model from a slice of bytes.
    fn from(slice: &[u8]) -> Self {
        Self(slice.to_vec())
    }
}
