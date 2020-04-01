#![allow(dead_code)] // temporary

use std::ops::Range;

use sodiumoxide::crypto::{box_, sign};

use super::{MsgBoxBufMut, MsgBoxBufRef, MsgBoxDecr, MsgBoxEncr, SUM_TAG};
use crate::pet::PetError;

// sum box field ranges
const EPHM_PK_RANGE: Range<usize> = 65..97; // 32 bytes

/// Mutable and immutable buffer access to sum box fields.
struct SumBoxBuffer<B> {
    bytes: B,
}

impl SumBoxBuffer<Vec<u8>> {
    /// Create an empty sum box buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }
}

impl<B: AsRef<[u8]>> SumBoxBuffer<B> {
    /// Create a sum box buffer from `bytes`. Fails if the `bytes` don't conform to the expected
    /// sum box length `exp_len`.
    fn from(bytes: B, exp_len: usize) -> Result<Self, PetError> {
        (bytes.as_ref().len() == exp_len)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'b, B: AsRef<[u8]> + ?Sized> MsgBoxBufRef<'b> for SumBoxBuffer<&'b B> {
    /// Access the sum box buffer by reference.
    fn bytes(&self) -> &'b [u8] {
        self.bytes.as_ref()
    }
}

impl<'b, B: AsRef<[u8]> + ?Sized> SumBoxBuffer<&'b B> {
    /// Access the public ephemeral key field of the sum box buffer by reference.
    fn ephm_pk(&self) -> &'b [u8] {
        &self.bytes()[EPHM_PK_RANGE]
    }
}

impl<B: AsMut<[u8]>> MsgBoxBufMut for SumBoxBuffer<B> {
    /// Access the sum box buffer by mutable reference.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<B: AsMut<[u8]>> SumBoxBuffer<B> {
    /// Access the public ephemeral key field of the sum box buffer by mutable reference.
    fn ephm_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[EPHM_PK_RANGE]
    }
}

/// Encryption and decryption of sum boxes boxes.
pub struct SumBox<C, S, E> {
    certificate: C,
    signature_sum: S,
    ephm_pk: E,
}

impl<'b> SumBox<&'b [u8], &'b sign::Signature, &'b box_::PublicKey> {
    /// Create a sum box.
    pub fn new(
        certificate: &'b [u8],
        signature_sum: &'b sign::Signature,
        ephm_pk: &'b box_::PublicKey,
    ) -> Self {
        Self {
            certificate,
            signature_sum,
            ephm_pk,
        }
    }
}

impl MsgBoxEncr for SumBox<&[u8], &sign::Signature, &box_::PublicKey> {
    /// Get the length of the serialized sum box.
    fn len(&self) -> usize {
        1 + 0 + sign::SIGNATUREBYTES + box_::PUBLICKEYBYTES // 97 bytes
    }

    /// Serialize the sum box to bytes.
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = SumBoxBuffer::new(self.len());
        buffer.tag_mut().copy_from_slice([SUM_TAG].as_ref());
        buffer.certificate_mut().copy_from_slice(&self.certificate);
        buffer
            .signature_sum_mut()
            .copy_from_slice(self.signature_sum.as_ref());
        buffer.ephm_pk_mut().copy_from_slice(self.ephm_pk.as_ref());
        buffer.bytes
    }
}

impl MsgBoxDecr for SumBox<Vec<u8>, sign::Signature, box_::PublicKey> {
    /// Get the expected length of a serialized sum box.
    fn exp_len(_: Option<usize>) -> usize {
        1 + 0 + sign::SIGNATUREBYTES + box_::PUBLICKEYBYTES // 97 bytes
    }

    /// Deserialize a sum box from bytes. Fails if the `bytes` don't conform to the expected sum box
    /// length `exp_len`.
    fn deserialize(bytes: &[u8], exp_len: usize) -> Result<Self, PetError> {
        let buffer = SumBoxBuffer::from(bytes, exp_len)?;
        (buffer.tag() == [SUM_TAG])
            .then_some(())
            .ok_or(PetError::InvalidMessage)?;
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
