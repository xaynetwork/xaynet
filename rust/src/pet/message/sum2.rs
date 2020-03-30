#![allow(dead_code)] // temporary

use std::ops::Range;

use sodiumoxide::crypto::{box_, sign};

use super::{SUM_TAG, TAG_RANGE};
use crate::pet::PetError;

const CERTIFICATE_RANGE: Range<usize> = 1..1;
const SIGN_SUM_RANGE: Range<usize> = 1..65;
const MASK_URL_RANGE: Range<usize> = 65..97;
const MESSAGE_LENGTH: usize = 97;

struct Sum2BoxBuffer<T> {
    bytes: T,
}

impl<T: AsRef<[u8]>> Sum2BoxBuffer<T> {
    fn new(bytes: T) -> Result<Self, PetError> {
        (bytes.as_ref().len() == MESSAGE_LENGTH)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Sum2BoxBuffer<&'a T> {
    fn tag(&self) -> &'a [u8] {
        &self.bytes.as_ref()[TAG_RANGE]
    }

    fn certificate(&self) -> &'a [u8] {
        &self.bytes.as_ref()[CERTIFICATE_RANGE]
    }

    fn signature_sum(&self) -> &'a [u8] {
        &self.bytes.as_ref()[SIGN_SUM_RANGE]
    }

    fn mask_url(&self) -> &'a [u8] {
        &self.bytes.as_ref()[MASK_URL_RANGE]
    }
}

impl<T: AsMut<[u8]>> Sum2BoxBuffer<T> {
    fn tag_mut(&mut self) -> &mut [u8] {
        &mut self.bytes.as_mut()[TAG_RANGE]
    }

    fn certificate_mut(&mut self) -> &mut [u8] {
        &mut self.bytes.as_mut()[CERTIFICATE_RANGE]
    }

    fn signature_sum_mut(&mut self) -> &mut [u8] {
        &mut self.bytes.as_mut()[SIGN_SUM_RANGE]
    }

    fn mask_url_mut(&mut self) -> &mut [u8] {
        &mut self.bytes.as_mut()[MASK_URL_RANGE]
    }
}

pub struct Sum2Box {
    certificate: Vec<u8>,
    signature_sum: sign::Signature,
    mask_url: Vec<u8>,
}

impl Sum2Box {
    pub fn serialize(&self, bytes: &mut [u8]) -> Result<(), PetError> {
        let mut buffer = Sum2BoxBuffer::new(bytes)?;
        buffer.tag_mut().copy_from_slice([SUM_TAG; 1].as_ref());
        buffer.certificate_mut().copy_from_slice(&self.certificate);
        buffer
            .signature_sum_mut()
            .copy_from_slice(self.signature_sum.as_ref());
        buffer.mask_url_mut().copy_from_slice(&self.mask_url);
        Ok(())
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, PetError> {
        let buffer = Sum2BoxBuffer::new(bytes)?;
        let certificate = buffer.certificate().to_vec();
        let signature_sum = sign::Signature::from_slice(buffer.signature_sum()).unwrap();
        let mask_url = buffer.mask_url().to_vec();
        Ok(Self {
            certificate,
            signature_sum,
            mask_url,
        })
    }

    pub fn seal(
        &self,
        coord_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        let mut bytes = vec![0_u8; MESSAGE_LENGTH];
        self.serialize(&mut bytes)?;
        let nonce = box_::gen_nonce();
        let sumbox = box_::seal(&bytes, &nonce, coord_encr_pk, part_encr_sk);
        Ok([nonce.as_ref(), &sumbox].concat())
    }

    fn open(
        cipher: &[u8],
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Self, PetError> {
        let nonce = (cipher.len() >= box_::NONCEBYTES)
            .then_some(box_::Nonce::from_slice(&cipher[0..box_::NONCEBYTES]).unwrap())
            .ok_or(PetError::InvalidMessage)?;
        let bytes = box_::open(cipher, &nonce, coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))?;
        Self::deserialize(&bytes)
    }
}
