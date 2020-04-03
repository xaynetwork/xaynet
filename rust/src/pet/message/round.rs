#![allow(dead_code)] // temporary

use std::ops::Range;

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{ROUND_TAG, TAG_RANGE};
use crate::pet::PetError;

// round box field ranges
const ENCR_PK_RANGE: Range<usize> = 1..33; // 32 bytes
const SIGN_PK_RANGE: Range<usize> = 33..65; // 32 bytes

#[derive(Debug)]
/// Mutable and immutable buffer access to round box fields.
struct RoundBoxBuffer<B> {
    bytes: B,
}

impl RoundBoxBuffer<Vec<u8>> {
    /// Create an empty round box buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }
}

impl<B: AsRef<[u8]>> RoundBoxBuffer<B> {
    /// Create a round box buffer from `bytes`. Fails if the `bytes` don't conform to the expected
    /// round box length `exp_len`.
    fn from(bytes: B, exp_len: usize) -> Result<Self, PetError> {
        (bytes.as_ref().len() == exp_len)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'b, B: AsRef<[u8]> + ?Sized> RoundBoxBuffer<&'b B> {
    /// Access the round box buffer by reference.
    fn bytes(&self) -> &'b [u8] {
        self.bytes.as_ref()
    }

    /// Access the tag field of the round box buffer by reference.
    fn tag(&self) -> &'b [u8] {
        &self.bytes()[TAG_RANGE]
    }

    /// Access the public encryption key field of the round box buffer by reference.
    fn encr_pk(&self) -> &'b [u8] {
        &self.bytes()[ENCR_PK_RANGE]
    }

    /// Access the public signature key field of the round box buffer by reference.
    fn sign_pk(&self) -> &'b [u8] {
        &self.bytes()[SIGN_PK_RANGE]
    }
}

impl<B: AsMut<[u8]>> RoundBoxBuffer<B> {
    /// Access the round box buffer by mutable reference.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }

    /// Access the tag field of the round box buffer by mutable reference.
    fn tag_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[TAG_RANGE]
    }

    /// Access the public encryption key field of the round box buffer by mutable reference.
    fn encr_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[ENCR_PK_RANGE]
    }

    /// Access the public signature key field of the round box buffer by mutable reference.
    fn sign_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[SIGN_PK_RANGE]
    }
}

#[derive(Debug, PartialEq)]
/// Encryption and decryption of round boxes.
pub struct RoundBox<E, S> {
    encr_pk: E,
    sign_pk: S,
}

impl<'b> RoundBox<&'b box_::PublicKey, &'b sign::PublicKey> {
    /// Create a round box.
    pub fn new(encr_pk: &'b box_::PublicKey, sign_pk: &'b sign::PublicKey) -> Self {
        Self { encr_pk, sign_pk }
    }

    /// Get the length of the serialized round box.
    pub fn len(&self) -> usize {
        1 + self.encr_pk.as_ref().len() + self.sign_pk.as_ref().len() // 65 bytes
    }

    /// Serialize the round box to bytes.
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = RoundBoxBuffer::new(self.len());
        buffer.tag_mut().copy_from_slice([ROUND_TAG; 1].as_ref());
        buffer.encr_pk_mut().copy_from_slice(self.encr_pk.as_ref());
        buffer.sign_pk_mut().copy_from_slice(self.sign_pk.as_ref());
        buffer.bytes
    }

    /// Encrypt the round box.
    pub fn seal(&self, pk: &box_::PublicKey) -> Vec<u8> {
        let bytes = self.serialize();
        sealedbox::seal(&bytes, pk)
    }
}

impl RoundBox<box_::PublicKey, sign::PublicKey> {
    /// Get the expected length of a serialized round box.
    pub fn exp_len() -> usize {
        1 + box_::PUBLICKEYBYTES + sign::PUBLICKEYBYTES // 65 bytes
    }

    /// Deserialize a round box from bytes. Fails if the `bytes` don't conform to the expected
    /// round box length.
    fn deserialize(bytes: &[u8]) -> Result<Self, PetError> {
        let buffer = RoundBoxBuffer::from(bytes, Self::exp_len())?;
        (buffer.tag() == [ROUND_TAG])
            .then_some(())
            .ok_or(PetError::InvalidMessage)?;
        let encr_pk = box_::PublicKey::from_slice(buffer.encr_pk()).unwrap();
        let sign_pk = sign::PublicKey::from_slice(buffer.sign_pk()).unwrap();
        Ok(Self { encr_pk, sign_pk })
    }

    /// Decrypt a round box. Fails if the `bytes` don't conform to a valid encrypted round box.
    pub fn open(
        bytes: &[u8],
        pk: &box_::PublicKey,
        sk: &box_::SecretKey,
    ) -> Result<Self, PetError> {
        let bytes = sealedbox::open(bytes, pk, sk).or(Err(PetError::InvalidMessage))?;
        Self::deserialize(&bytes)
    }

    /// Get a reference to the public encryption key.
    pub fn encr_pk(&self) -> &box_::PublicKey {
        &self.encr_pk
    }

    /// Get a reference to the public signature key.
    pub fn sign_pk(&self) -> &sign::PublicKey {
        &self.sign_pk
    }
}

#[cfg(test)]
mod tests {
    use sodiumoxide::randombytes::randombytes;

    use super::*;

    #[test]
    fn test_roundbox_field_ranges() {
        assert_eq!(ENCR_PK_RANGE.end - ENCR_PK_RANGE.start, 32);
        assert_eq!(SIGN_PK_RANGE.end - SIGN_PK_RANGE.start, 32);
    }

    #[test]
    fn test_roundboxbuffer() {
        // new
        assert_eq!(RoundBoxBuffer::new(10).bytes, vec![0_u8; 10]);

        // from
        let bytes = randombytes(65);
        let bytes_ = bytes.clone();
        let mut bytes_mut = bytes.clone();
        let mut bytes_mut_ = bytes.clone();
        assert_eq!(
            RoundBoxBuffer::from(bytes.clone(), 65).unwrap().bytes,
            bytes.clone()
        );
        assert_eq!(
            RoundBoxBuffer::from(&bytes, 65).unwrap().bytes as *const Vec<u8>,
            &bytes as *const Vec<u8>
        );
        assert_eq!(
            RoundBoxBuffer::from(&mut bytes_mut, 65).unwrap().bytes as *mut Vec<u8>,
            &mut bytes_mut as *mut Vec<u8>
        );
        assert_eq!(
            RoundBoxBuffer::from(&bytes, 10).unwrap_err(),
            PetError::InvalidMessage
        );

        // bytes
        let buf = RoundBoxBuffer::from(&bytes, 65).unwrap();
        let mut buf_mut = RoundBoxBuffer::from(&mut bytes_mut, 65).unwrap();
        assert_eq!(buf.bytes(), &bytes_[0..65]);
        assert_eq!(buf_mut.bytes_mut(), &mut bytes_mut_[0..65]);

        // tag
        assert_eq!(buf.tag(), &bytes_[0..1]);
        assert_eq!(buf_mut.tag_mut(), &mut bytes_mut_[0..1]);

        // encr pk
        assert_eq!(buf.encr_pk(), &bytes_[1..33]);
        assert_eq!(buf_mut.encr_pk_mut(), &mut bytes_mut_[1..33]);

        // sign pk
        assert_eq!(buf.sign_pk(), &bytes_[33..65]);
        assert_eq!(buf_mut.sign_pk_mut(), &mut bytes_mut_[33..65]);
    }

    #[test]
    fn test_roundbox_ref() {
        // new
        let encr_pk = &box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let sign_pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let rbox = RoundBox::new(encr_pk, sign_pk);
        assert_eq!(
            rbox.encr_pk as *const box_::PublicKey,
            encr_pk as *const box_::PublicKey
        );
        assert_eq!(
            rbox.sign_pk as *const sign::PublicKey,
            sign_pk as *const sign::PublicKey
        );

        // len
        assert_eq!(rbox.len(), 65);

        // serialize
        assert_eq!(
            rbox.serialize(),
            [[100_u8; 1].as_ref(), encr_pk.as_ref(), sign_pk.as_ref()].concat()
        );
    }

    #[test]
    fn test_roundbox_val() {
        // exp len
        assert_eq!(RoundBox::exp_len(), 65);

        // deserialize
        let encr_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let sign_pk = sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let bytes = [[100_u8; 1].as_ref(), encr_pk.as_ref(), sign_pk.as_ref()].concat();
        let rbox = RoundBox { encr_pk, sign_pk };
        assert_eq!(RoundBox::deserialize(&bytes).unwrap(), rbox);
        assert_eq!(
            RoundBox::deserialize(&vec![0_u8; 10]).unwrap_err(),
            PetError::InvalidMessage
        );
        assert_eq!(
            RoundBox::deserialize(
                &[[0_u8; 1].as_ref(), encr_pk.as_ref(), sign_pk.as_ref()].concat()
            )
            .unwrap_err(),
            PetError::InvalidMessage
        );

        // encr pk
        assert_eq!(rbox.encr_pk(), &encr_pk);

        // sign pk
        assert_eq!(rbox.sign_pk(), &sign_pk);
    }

    #[test]
    fn test_roundbox() {
        let encr_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let sign_pk = sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let (pk, sk) = box_::gen_keypair();
        let bytes = RoundBox::new(&encr_pk, &sign_pk).seal(&pk);
        let rbox = RoundBox::open(&bytes, &pk, &sk).unwrap();
        assert_eq!(rbox, RoundBox { encr_pk, sign_pk });
    }
}
