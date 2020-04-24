use crate::PetError;

#[derive(Debug, PartialEq)]
/// A dummy certificate.
pub struct Certificate(Vec<u8>);

#[allow(clippy::len_without_is_empty)]
impl Certificate {
    #[allow(clippy::new_without_default)]
    /// Create a certificate
    pub fn new() -> Self {
        Self(vec![0_u8; 32])
    }

    /// Get the length of the certificate.
    pub fn len(&self) -> usize {
        self.as_ref().len()
    }

    /// Validate a certificate
    pub fn validate(&self) -> Result<(), PetError> {
        if self.as_ref() == [0_u8; 32] {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

impl AsRef<[u8]> for Certificate {
    /// Get a reference to the certificate.
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for Certificate {
    /// Create a certificate from bytes.
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for Certificate {
    /// Create a certificate from a slice of bytes.
    fn from(slice: &[u8]) -> Self {
        Self(slice.to_vec())
    }
}
