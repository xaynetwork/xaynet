#![allow(dead_code)] // temporary

use std::ops::Range;

use sodiumoxide::crypto::{box_, sign};

use super::{MsgBoxBufMut, MsgBoxBufRef, MsgBoxDecr, MsgBoxEncr, SUM_TAG};
use crate::pet::PetError;

// sum box field ranges
const EPHM_PK_RANGE: Range<usize> = 65..97; // 32 bytes

#[derive(Debug)]
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

#[derive(Clone, Debug, PartialEq)]
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
        // 97 bytes
        1 + self.certificate.len() + self.signature_sum.as_ref().len() + self.ephm_pk.as_ref().len()
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

impl SumBox<Vec<u8>, sign::Signature, box_::PublicKey> {
    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &[u8] {
        &self.certificate
    }

    /// Get a reference to the sum signature.
    pub fn signature_sum(&self) -> &sign::Signature {
        &self.signature_sum
    }

    /// Get a reference to the public ephemeral key.
    pub fn ephm_pk(&self) -> &box_::PublicKey {
        &self.ephm_pk
    }
}

impl MsgBoxDecr for SumBox<Vec<u8>, sign::Signature, box_::PublicKey> {
    #[allow(clippy::identity_op)] // temporary
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

#[cfg(test)]
mod tests {
    use sodiumoxide::randombytes::randombytes;

    use super::*;

    #[test]
    fn test_sumbox_field_ranges() {
        assert_eq!(EPHM_PK_RANGE.end - EPHM_PK_RANGE.start, 32);
    }

    #[test]
    fn test_sumboxbuffer() {
        // new
        assert_eq!(SumBoxBuffer::new(10).bytes, vec![0_u8; 10]);

        // from
        let len = 97;
        let bytes = randombytes(len);
        let bytes_ = bytes.clone();
        let mut bytes_mut = bytes.clone();
        let mut bytes_mut_ = bytes.clone();
        assert_eq!(
            SumBoxBuffer::from(bytes.clone(), len).unwrap().bytes,
            bytes.clone(),
        );
        assert_eq!(
            SumBoxBuffer::from(&bytes, len).unwrap().bytes as *const Vec<u8>,
            &bytes as *const Vec<u8>,
        );
        assert_eq!(
            SumBoxBuffer::from(&mut bytes_mut, len).unwrap().bytes as *mut Vec<u8>,
            &mut bytes_mut as *mut Vec<u8>,
        );
        assert_eq!(
            SumBoxBuffer::from(&bytes, 10).unwrap_err(),
            PetError::InvalidMessage,
        );

        // bytes
        let buf = SumBoxBuffer::from(&bytes, len).unwrap();
        let mut buf_mut = SumBoxBuffer::from(&mut bytes_mut, len).unwrap();
        assert_eq!(buf.bytes(), &bytes_[..]);
        assert_eq!(buf_mut.bytes_mut(), &mut bytes_mut_[..]);

        // tag
        assert_eq!(buf.tag(), &bytes_[0..1]);
        assert_eq!(buf_mut.tag_mut(), &mut bytes_mut_[0..1]);

        // certificate
        assert_eq!(buf.certificate(), &bytes_[1..1]);
        assert_eq!(buf_mut.certificate_mut(), &mut bytes_mut_[1..1]);

        // signature sum
        assert_eq!(buf.signature_sum(), &bytes_[1..65]);
        assert_eq!(buf_mut.signature_sum_mut(), &mut bytes_mut_[1..65]);

        // ephm pk
        assert_eq!(buf.ephm_pk(), &bytes_[65..97]);
        assert_eq!(buf_mut.ephm_pk_mut(), &mut bytes_mut_[65..97]);
    }

    #[test]
    fn test_sumbox_ref() {
        // new
        let certificate = Vec::<u8>::new();
        let signature_sum = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let ephm_pk = &box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let sbox = SumBox::new(&certificate, signature_sum, ephm_pk);
        assert_eq!(sbox.certificate, certificate.as_slice());
        assert_eq!(
            sbox.signature_sum as *const sign::Signature,
            signature_sum as *const sign::Signature,
        );
        assert_eq!(
            sbox.ephm_pk as *const box_::PublicKey,
            ephm_pk as *const box_::PublicKey,
        );

        // len
        assert_eq!(sbox.len(), 97);

        // serialize
        assert_eq!(
            sbox.serialize(),
            [
                [101_u8; 1].as_ref(),
                certificate.as_slice(),
                signature_sum.as_ref(),
                ephm_pk.as_ref(),
            ]
            .concat(),
        );
    }

    #[test]
    fn test_sumbox_val() {
        // exp len
        let len = 97;
        assert_eq!(SumBox::exp_len(None), len);

        // deserialize
        let certificate = Vec::<u8>::new();
        let signature_sum = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let ephm_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let bytes = [
            [101_u8; 1].as_ref(),
            certificate.as_slice(),
            signature_sum.as_ref(),
            ephm_pk.as_ref(),
        ]
        .concat();
        let sbox = SumBox::deserialize(&bytes, len).unwrap();
        assert_eq!(
            sbox,
            SumBox {
                certificate: certificate.clone(),
                signature_sum,
                ephm_pk,
            },
        );
        assert_eq!(
            SumBox::deserialize(&vec![0_u8; 10], len).unwrap_err(),
            PetError::InvalidMessage,
        );
        assert_eq!(
            SumBox::deserialize(&vec![0_u8; len], len).unwrap_err(),
            PetError::InvalidMessage,
        );

        // certificate
        assert_eq!(sbox.certificate(), certificate.as_slice());

        // signature sum
        assert_eq!(sbox.signature_sum(), &signature_sum);

        // ephm pk
        assert_eq!(sbox.ephm_pk(), &ephm_pk);
    }

    #[test]
    fn test_sumbox() {
        let certificate = Vec::<u8>::new();
        let signature_sum = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let ephm_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let (pk, sk) = box_::gen_keypair();
        let (nonce, bytes) = SumBox::new(&certificate, &signature_sum, &ephm_pk).seal(&pk, &sk);
        let sbox = SumBox::open(&bytes, &nonce, &pk, &sk, 97).unwrap();
        assert_eq!(
            sbox,
            SumBox {
                certificate,
                signature_sum,
                ephm_pk,
            },
        );
    }
}
