#![allow(dead_code)] // temporary

use std::ops::Range;

use sodiumoxide::crypto::{box_, sign};

use super::{BufferMut, BufferRef, SUM_TAG};
use crate::pet::PetError;

const MASK_URL_RANGE: Range<usize> = 65..97;
const MESSAGE_LENGTH: usize = 97;

struct Sum2BoxBuffer<T> {
    bytes: T,
}

impl Sum2BoxBuffer<Vec<u8>> {
    fn new() -> Self {
        Self {
            bytes: vec![0_u8; MESSAGE_LENGTH],
        }
    }
}

impl<T: AsRef<[u8]>> Sum2BoxBuffer<T> {
    fn from(bytes: T, len: usize) -> Result<Self, PetError> {
        (bytes.as_ref().len() == len)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> BufferRef<'a> for Sum2BoxBuffer<&'a T> {
    fn bytes(&self) -> &'a [u8] {
        self.bytes.as_ref()
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Sum2BoxBuffer<&'a T> {
    fn mask_url(&self) -> &'a [u8] {
        &self.bytes()[MASK_URL_RANGE]
    }
}

impl<T: AsMut<[u8]>> BufferMut for Sum2BoxBuffer<T> {
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<T: AsMut<[u8]>> Sum2BoxBuffer<T> {
    fn mask_url_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[MASK_URL_RANGE]
    }
}

pub struct Sum2Box {
    certificate: Vec<u8>,
    signature_sum: sign::Signature,
    mask_url: Vec<u8>,
}

impl Sum2Box {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Sum2BoxBuffer::new();
        buffer.tag_mut().copy_from_slice([SUM_TAG; 1].as_ref());
        buffer.certificate_mut().copy_from_slice(&self.certificate);
        buffer
            .signature_sum_mut()
            .copy_from_slice(self.signature_sum.as_ref());
        buffer.mask_url_mut().copy_from_slice(&self.mask_url);
        buffer.bytes
    }

    pub fn deserialize(bytes: &[u8], len: usize) -> Result<Self, PetError> {
        let buffer = Sum2BoxBuffer::from(bytes, len)?;
        let certificate = buffer.certificate().to_vec();
        let signature_sum = sign::Signature::from_slice(buffer.signature_sum()).unwrap();
        let mask_url = buffer.mask_url().to_vec();
        Ok(Self {
            certificate,
            signature_sum,
            mask_url,
        })
    }

    pub fn seal(&self, coord_encr_pk: &box_::PublicKey, part_encr_sk: &box_::SecretKey) -> Vec<u8> {
        let bytes = self.serialize();
        let nonce = box_::gen_nonce();
        let sumbox = box_::seal(&bytes, &nonce, coord_encr_pk, part_encr_sk);
        [nonce.as_ref(), &sumbox].concat()
    }

    fn open(
        cipher: &[u8],
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
        len: usize,
    ) -> Result<Self, PetError> {
        let nonce = (cipher.len() >= box_::NONCEBYTES)
            .then_some(box_::Nonce::from_slice(&cipher[0..box_::NONCEBYTES]).unwrap())
            .ok_or(PetError::InvalidMessage)?;
        let bytes = box_::open(cipher, &nonce, coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))?;
        Self::deserialize(&bytes, len)
    }
}
