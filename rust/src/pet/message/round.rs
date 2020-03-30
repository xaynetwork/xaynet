#![allow(dead_code)] // temporary

use std::ops::Range;

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{ROUND_TAG, TAG_RANGE};
use crate::pet::PetError;

const ENCR_PK_RANGE: Range<usize> = 1..33;
const SIGN_PK_RANGE: Range<usize> = 33..65;
const MESSAGE_LENGTH: usize = 65;

struct RoundBoxBuffer<T> {
    bytes: T,
}

impl<T: AsRef<[u8]>> RoundBoxBuffer<T> {
    fn new(bytes: T) -> Result<Self, PetError> {
        (bytes.as_ref().len() == MESSAGE_LENGTH)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> RoundBoxBuffer<&'a T> {
    fn tag(&self) -> &'a [u8] {
        &self.bytes.as_ref()[TAG_RANGE]
    }

    fn encr_pk(&self) -> &'a [u8] {
        &self.bytes.as_ref()[ENCR_PK_RANGE]
    }

    fn sign_pk(&self) -> &'a [u8] {
        &self.bytes.as_ref()[SIGN_PK_RANGE]
    }
}

impl<T: AsMut<[u8]>> RoundBoxBuffer<T> {
    fn tag_mut(&mut self) -> &mut [u8] {
        &mut self.bytes.as_mut()[TAG_RANGE]
    }

    fn encr_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes.as_mut()[ENCR_PK_RANGE]
    }

    fn sign_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes.as_mut()[SIGN_PK_RANGE]
    }
}

pub struct RoundBox {
    encr_pk: box_::PublicKey,
    sign_pk: sign::PublicKey,
}

impl RoundBox {
    pub fn serialize(&self, bytes: &mut [u8]) -> Result<(), PetError> {
        let mut buffer = RoundBoxBuffer::new(bytes)?;
        buffer.tag_mut().copy_from_slice([ROUND_TAG; 1].as_ref());
        buffer.encr_pk_mut().copy_from_slice(self.encr_pk.as_ref());
        buffer.sign_pk_mut().copy_from_slice(self.sign_pk.as_ref());
        Ok(())
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, PetError> {
        let buffer = RoundBoxBuffer::new(bytes)?;
        let encr_pk = box_::PublicKey::from_slice(buffer.encr_pk()).unwrap();
        let sign_pk = sign::PublicKey::from_slice(buffer.sign_pk()).unwrap();
        Ok(Self { encr_pk, sign_pk })
    }

    pub fn seal(&self, encr_pk: &box_::PublicKey) -> Result<Vec<u8>, PetError> {
        let mut bytes = vec![0_u8; MESSAGE_LENGTH];
        self.serialize(&mut bytes)?;
        Ok(sealedbox::seal(&bytes, encr_pk))
    }

    pub fn open(
        cipher: &[u8],
        encr_pk: &box_::PublicKey,
        encr_sk: &box_::SecretKey,
    ) -> Result<Self, PetError> {
        let bytes = sealedbox::open(cipher, encr_pk, encr_sk).or(Err(PetError::InvalidMessage))?;
        Self::deserialize(&bytes)
    }
}
