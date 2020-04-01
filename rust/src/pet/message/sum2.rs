#![allow(dead_code)] // temporary

use std::ops::Range;

use sodiumoxide::crypto::sign;

use super::{MessageBox, MessageBoxBufferMut, MessageBoxBufferRef, SUM2_TAG};
use crate::pet::PetError;

// sum2 box field ranges
const MASK_URL_RANGE: Range<usize> = 65..97;
const MESSAGE_LENGTH: usize = 97;

/// Mutable and immutable buffer access to sum2 box fields.
struct Sum2BoxBuffer<T> {
    bytes: T,
}

impl Sum2BoxBuffer<Vec<u8>> {
    /// Create an empty sum2 box buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }
}

impl<T: AsRef<[u8]>> Sum2BoxBuffer<T> {
    /// Create a sum2 box buffer from `bytes`. Fails if the `bytes` don't conform to the expected
    /// sum2 box length `exp_len`.
    fn from(bytes: T, exp_len: usize) -> Result<Self, PetError> {
        (bytes.as_ref().len() == exp_len)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> MessageBoxBufferRef<'a> for Sum2BoxBuffer<&'a T> {
    /// Access the sum2 box buffer by reference.
    fn bytes(&self) -> &'a [u8] {
        self.bytes.as_ref()
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Sum2BoxBuffer<&'a T> {
    /// Access the mask url field of the sum2 box buffer by reference.
    fn mask_url(&self) -> &'a [u8] {
        &self.bytes()[MASK_URL_RANGE]
    }
}

impl<T: AsMut<[u8]>> MessageBoxBufferMut for Sum2BoxBuffer<T> {
    /// Access the sum2 box buffer by mutable reference.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<T: AsMut<[u8]>> Sum2BoxBuffer<T> {
    /// Access the mask url field of the sum2 box buffer by mutable reference.
    fn mask_url_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[MASK_URL_RANGE]
    }
}

#[derive(Clone)]
/// Encryption and decryption of sum2 boxes.
pub struct Sum2Box {
    certificate: Vec<u8>,
    signature_sum: sign::Signature,
    mask_url: Vec<u8>,
}

impl Sum2Box {
    /// Create a sum2 box.
    pub fn new(certificate: &[u8], signature_sum: &sign::Signature, mask_url: &[u8]) -> Self {
        Self {
            certificate: Vec::from(certificate),
            signature_sum: signature_sum.clone(),
            mask_url: Vec::from(mask_url),
        }
    }
}

impl MessageBox for Sum2Box {
    /// Get the length of the serialized sum2 box.
    fn len(&self) -> usize {
        MESSAGE_LENGTH
    }

    /// Get the expected length of a serialized sum2 box.
    fn exp_len(_: Option<usize>) -> usize {
        MESSAGE_LENGTH
    }

    /// Serialize the sum2 box to bytes.
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = Sum2BoxBuffer::new(self.len());
        buffer.tag_mut().copy_from_slice([SUM2_TAG; 1].as_ref());
        buffer.certificate_mut().copy_from_slice(&self.certificate);
        buffer
            .signature_sum_mut()
            .copy_from_slice(self.signature_sum.as_ref());
        buffer.mask_url_mut().copy_from_slice(&self.mask_url);
        buffer.bytes
    }

    /// Deserialize a sum2 box from bytes. Fails if the `bytes` don't conform to the expected sum2
    /// box length `len`.
    fn deserialize(bytes: &[u8], exp_len: usize) -> Result<Self, PetError> {
        let buffer = Sum2BoxBuffer::from(bytes, exp_len)?;
        (buffer.tag() == [SUM2_TAG])
            .then_some(())
            .ok_or(PetError::InvalidMessage)?;
        let certificate = buffer.certificate().to_vec();
        let signature_sum = sign::Signature::from_slice(buffer.signature_sum()).unwrap();
        let mask_url = buffer.mask_url().to_vec();
        Ok(Self {
            certificate,
            signature_sum,
            mask_url,
        })
    }
}
