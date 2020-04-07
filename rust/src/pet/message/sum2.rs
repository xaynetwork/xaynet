use std::ops::Range;

use sodiumoxide::crypto::sign;

use super::{MsgBoxBufMut, MsgBoxBufRef, MsgBoxDecr, MsgBoxEncr, SUM2_TAG};
use crate::pet::PetError;

// sum2 box field ranges
const MASK_URL_RANGE: Range<usize> = 65..97; // 32 bytes

#[derive(Debug)]
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

#[derive(Clone, Debug, PartialEq)]
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
    /// Get the length of the serialized sum2 box.
    fn len(&self) -> usize {
        // 97 bytes
        1 + self.certificate.len() + self.signature_sum.as_ref().len() + self.mask_url.len()
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

#[cfg(test)]
mod tests {
    use sodiumoxide::{crypto::box_, randombytes::randombytes};

    use super::*;

    #[test]
    fn test_sum2box_field_ranges() {
        assert_eq!(MASK_URL_RANGE.end - MASK_URL_RANGE.start, 32);
    }

    #[test]
    fn test_sum2boxbuffer() {
        // new
        assert_eq!(Sum2BoxBuffer::new(10).bytes, vec![0_u8; 10]);

        // from
        let len = 97;
        let bytes = randombytes(len);
        let bytes_ = bytes.clone();
        let mut bytes_mut = bytes.clone();
        let mut bytes_mut_ = bytes.clone();
        assert_eq!(
            Sum2BoxBuffer::from(bytes.clone(), len).unwrap().bytes,
            bytes.clone(),
        );
        assert_eq!(
            Sum2BoxBuffer::from(&bytes, len).unwrap().bytes as *const Vec<u8>,
            &bytes as *const Vec<u8>,
        );
        assert_eq!(
            Sum2BoxBuffer::from(&mut bytes_mut, len).unwrap().bytes as *mut Vec<u8>,
            &mut bytes_mut as *mut Vec<u8>,
        );
        assert_eq!(
            Sum2BoxBuffer::from(&bytes, 10).unwrap_err(),
            PetError::InvalidMessage,
        );

        // bytes
        let buf = Sum2BoxBuffer::from(&bytes, len).unwrap();
        let mut buf_mut = Sum2BoxBuffer::from(&mut bytes_mut, len).unwrap();
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
        assert_eq!(buf.mask_url(), &bytes_[65..97]);
        assert_eq!(buf_mut.mask_url_mut(), &mut bytes_mut_[65..97]);
    }

    #[test]
    fn test_sum2box_ref() {
        // new
        let certificate = Vec::<u8>::new();
        let signature_sum = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let mask_url = randombytes(32);
        let sbox = Sum2Box::new(&certificate, signature_sum, &mask_url);
        assert_eq!(sbox.certificate, certificate.as_slice());
        assert_eq!(
            sbox.signature_sum as *const sign::Signature,
            signature_sum as *const sign::Signature,
        );
        assert_eq!(sbox.mask_url, mask_url.as_slice());

        // len
        assert_eq!(sbox.len(), 97);

        // serialize
        assert_eq!(
            sbox.serialize(),
            [
                [103_u8; 1].as_ref(),
                certificate.as_slice(),
                signature_sum.as_ref(),
                mask_url.as_slice(),
            ]
            .concat(),
        );
    }

    #[test]
    fn test_sum2box_val() {
        // exp len
        let len = 97;
        assert_eq!(Sum2Box::exp_len(None), len);

        // deserialize
        let certificate = Vec::<u8>::new();
        let signature_sum = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let mask_url = randombytes(32);
        let bytes = [
            [103_u8; 1].as_ref(),
            certificate.as_slice(),
            signature_sum.as_ref(),
            mask_url.as_slice(),
        ]
        .concat();
        let sbox = Sum2Box::deserialize(&bytes, len).unwrap();
        assert_eq!(
            sbox,
            Sum2Box {
                certificate: certificate.clone(),
                signature_sum,
                mask_url: mask_url.clone(),
            },
        );
        assert_eq!(
            Sum2Box::deserialize(&vec![0_u8; 10], len).unwrap_err(),
            PetError::InvalidMessage,
        );
        assert_eq!(
            Sum2Box::deserialize(&vec![0_u8; len], len).unwrap_err(),
            PetError::InvalidMessage,
        );

        // certificate
        assert_eq!(sbox.certificate(), certificate.as_slice());

        // signature sum
        assert_eq!(sbox.signature_sum(), &signature_sum);

        // mask url
        assert_eq!(sbox.mask_url(), mask_url.as_slice());
    }

    #[test]
    fn test_sum2box() {
        let certificate = Vec::<u8>::new();
        let signature_sum = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let mask_url = randombytes(32);
        let (pk, sk) = box_::gen_keypair();
        let (nonce, bytes) = Sum2Box::new(&certificate, &signature_sum, &mask_url).seal(&pk, &sk);
        let sbox = Sum2Box::open(&bytes, &nonce, &pk, &sk, 97).unwrap();
        assert_eq!(
            sbox,
            Sum2Box {
                certificate,
                signature_sum,
                mask_url,
            },
        );
    }
}
