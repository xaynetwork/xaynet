#![allow(dead_code)] // temporary

use std::ops::Range;

use sodiumoxide::crypto::{box_, sign};

use super::{MessageBox, MessageBoxBufferMut, MessageBoxBufferRef, SUM_TAG};
use crate::pet::PetError;

// sum box field ranges
const EPHM_PK_RANGE: Range<usize> = 65..97;
const MESSAGE_LENGTH: usize = 97;

/// Mutable and immutable buffer access to sum box fields.
struct SumBoxBuffer<T> {
    bytes: T,
}

impl SumBoxBuffer<Vec<u8>> {
    /// Create an empty sum box buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }
}

impl<T: AsRef<[u8]>> SumBoxBuffer<T> {
    /// Create a sum box buffer from `bytes`. Fails if the `bytes` don't conform to the expected
    /// sum box length `exp_len`.
    fn from(bytes: T, exp_len: usize) -> Result<Self, PetError> {
        (bytes.as_ref().len() == exp_len)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> MessageBoxBufferRef<'a> for SumBoxBuffer<&'a T> {
    /// Access the sum box buffer by reference.
    fn bytes(&self) -> &'a [u8] {
        self.bytes.as_ref()
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> SumBoxBuffer<&'a T> {
    /// Access the public ephemeral key field of the sum box buffer by reference.
    fn ephm_pk(&self) -> &'a [u8] {
        &self.bytes()[EPHM_PK_RANGE]
    }
}

impl<T: AsMut<[u8]>> MessageBoxBufferMut for SumBoxBuffer<T> {
    /// Access the sum box buffer by mutable reference.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<T: AsMut<[u8]>> SumBoxBuffer<T> {
    /// Access the public ephemeral key field of the sum box buffer by mutable reference.
    fn ephm_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[EPHM_PK_RANGE]
    }
}

/// Encryption and decryption of sum boxes boxes.
pub struct SumBox {
    certificate: Vec<u8>,
    signature_sum: sign::Signature,
    ephm_pk: box_::PublicKey,
}

impl MessageBox for SumBox {
    /// Get the length of the serialized sum box.
    fn len(&self) -> usize {
        MESSAGE_LENGTH
    }

    /// Get the expected length of a serialized sum box.
    fn exp_len(_: Option<usize>) -> usize {
        MESSAGE_LENGTH
    }

    /// Serialize the sum box to bytes.
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = SumBoxBuffer::new(self.len());
        buffer.tag_mut().copy_from_slice([SUM_TAG; 1].as_ref());
        buffer.certificate_mut().copy_from_slice(&self.certificate);
        buffer
            .signature_sum_mut()
            .copy_from_slice(self.signature_sum.as_ref());
        buffer.ephm_pk_mut().copy_from_slice(self.ephm_pk.as_ref());
        buffer.bytes
    }

    /// Deserialize a sum box from bytes. Fails if the `bytes` don't conform to the expected sum box
    /// length `exp_len`.
    fn deserialize(bytes: &[u8], exp_len: usize) -> Result<Self, PetError> {
        let buffer = SumBoxBuffer::from(bytes, exp_len)?;
        let certificate = buffer.certificate().to_vec();
        let signature_sum = sign::Signature::from_slice(buffer.signature_sum()).unwrap();
        let ephm_pk = box_::PublicKey::from_slice(buffer.ephm_pk()).unwrap();
        Ok(Self {
            certificate,
            signature_sum,
            ephm_pk,
        })
    }
}
