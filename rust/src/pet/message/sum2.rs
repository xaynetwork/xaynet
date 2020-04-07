#![allow(dead_code)] // temporary

use std::ops::Range;

use sodiumoxide::crypto::sign;

use super::{MsgBoxBufMut, MsgBoxBufRef, MsgBoxDecr, MsgBoxEncr, SUM2_TAG};
use crate::pet::PetError;

// sum2 box field ranges
const MASK_URL_RANGE: Range<usize> = 65..97; // 32 bytes

/// Mutable and immutable buffer access to sum2 box fields.
struct Sum2BoxBuffer<B> {
    bytes: B,
}

impl Sum2BoxBuffer<Vec<u8>> {
    /// Create an empty sum2 box buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }
}

impl<B: AsRef<[u8]>> Sum2BoxBuffer<B> {
    /// Create a sum2 box buffer from `bytes`. Fails if the `bytes` don't conform to the expected
    /// sum2 box length `exp_len`.
    fn from(bytes: B, exp_len: usize) -> Result<Self, PetError> {
        (bytes.as_ref().len() == exp_len)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'b, B: AsRef<[u8]> + ?Sized> MsgBoxBufRef<'b> for Sum2BoxBuffer<&'b B> {
    /// Access the sum2 box buffer by reference.
    fn bytes(&self) -> &'b [u8] {
        self.bytes.as_ref()
    }
}

impl<'b, B: AsRef<[u8]> + ?Sized> Sum2BoxBuffer<&'b B> {
    /// Access the mask url field of the sum2 box buffer by reference.
    fn mask_url(&self) -> &'b [u8] {
        &self.bytes()[MASK_URL_RANGE]
    }
}

impl<B: AsMut<[u8]>> MsgBoxBufMut for Sum2BoxBuffer<B> {
    /// Access the sum2 box buffer by mutable reference.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<B: AsMut<[u8]>> Sum2BoxBuffer<B> {
    /// Access the mask url field of the sum2 box buffer by mutable reference.
    fn mask_url_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[MASK_URL_RANGE]
    }
}

/// Encryption and decryption of sum2 boxes.
pub struct Sum2Box<C, S, M> {
    certificate: C,
    signature_sum: S,
    mask_url: M,
}

impl<'b> Sum2Box<&'b [u8], &'b sign::Signature, &'b [u8]> {
    /// Create a sum2 box.
    pub fn new(
        certificate: &'b [u8],
        signature_sum: &'b sign::Signature,
        mask_url: &'b [u8],
    ) -> Self {
        Self {
            certificate,
            signature_sum,
            mask_url,
        }
    }
}

impl MsgBoxEncr for Sum2Box<&[u8], &sign::Signature, &[u8]> {
    #[allow(clippy::identity_op)] // temporary
    /// Get the length of the serialized sum2 box.
    fn len(&self) -> usize {
        1 + 0 + sign::SIGNATUREBYTES + 32 // 97 bytes
    }

    /// Serialize the sum2 box to bytes.
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = Sum2BoxBuffer::new(self.len());
        buffer.tag_mut().copy_from_slice([SUM2_TAG; 1].as_ref());
        buffer.certificate_mut().copy_from_slice(self.certificate);
        buffer
            .signature_sum_mut()
            .copy_from_slice(self.signature_sum.as_ref());
        buffer.mask_url_mut().copy_from_slice(self.mask_url);
        buffer.bytes
    }
}

impl Sum2Box<Vec<u8>, sign::Signature, Vec<u8>> {
    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &[u8] {
        &self.certificate
    }

    /// Get a reference to the sum signature.
    pub fn signature_sum(&self) -> &sign::Signature {
        &self.signature_sum
    }

    /// Get a reference to the mask url.
    pub fn mask_url(&self) -> &[u8] {
        &self.mask_url
    }
}

impl MsgBoxDecr for Sum2Box<Vec<u8>, sign::Signature, Vec<u8>> {
    #[allow(clippy::identity_op)] // temporary
    /// Get the expected length of a serialized sum2 box.
    fn exp_len(_: Option<usize>) -> usize {
        1 + 0 + sign::SIGNATUREBYTES + 32 // 97 bytes
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
