use derive_more::{AsMut, AsRef};

use crate::{crypto::ByteObject, PetError};

#[derive(AsRef, AsMut, Clone, Debug, PartialEq)]
/// A dummy certificate.
pub struct Certificate(Vec<u8>);

impl ByteObject for Certificate {
    /// Create a certificate a slice of bytes. Fails if the length of the input is invalid.
    fn from_slice(bytes: &[u8]) -> Option<Self> {
        Some(Self(bytes.to_vec()))
    }

    /// Create a certificate initialized to zero.
    fn zeroed() -> Self {
        Self(vec![0_u8; Self::BYTES])
    }

    /// Get the certificate as a slice.
    fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[allow(clippy::len_without_is_empty)]
impl Certificate {
    /// Get the number of bytes of a certificate.
    pub const BYTES: usize = 32;

    /// Get the length of the serialized certificate.
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// Serialize the certificate into bytes.
    pub fn serialize(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    /// Deserialize the certificate from bytes. Fails if the length of the input is invalid.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, PetError> {
        Self::from_slice(bytes).ok_or(PetError::InvalidMessage)
    }

    /// Validate the certificate.
    pub fn validate(&self) -> Result<(), PetError> {
        if self.as_slice() == [0_u8; 32] {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}
