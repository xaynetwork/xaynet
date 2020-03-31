#![allow(dead_code)] // temporary

use std::{collections::HashMap, ops::Range};

use sodiumoxide::crypto::{box_, sign};

use super::{MessageBox, MessageBoxBufferMut, MessageBoxBufferRef, UPDATE_TAG};
use crate::pet::PetError;

// update box field ranges
const SIGN_UPDATE_RANGE: Range<usize> = 65..129;
const MODEL_URL_RANGE: Range<usize> = 129..161;
const DICT_SEED_START: usize = 161;
const DICT_SEED_KEY_LENGTH: usize = 32;
const DICT_SEED_ITEM_LENGTH: usize = 112;

fn dict_seed_range(dict_seed_end: usize) -> Range<usize> {
    DICT_SEED_START..dict_seed_end
}

fn message_length(dict_length: usize) -> usize {
    DICT_SEED_START + DICT_SEED_ITEM_LENGTH * dict_length
}

/// Mutable and immutable buffer access to update box fields.
struct UpdateBoxBuffer<T> {
    bytes: T,
}

impl UpdateBoxBuffer<Vec<u8>> {
    /// Create an empty update box buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }
}

impl<T: AsRef<[u8]>> UpdateBoxBuffer<T> {
    /// Create an update box buffer from `bytes`. Fails if the `bytes` don't conform to the expected
    /// update box length `exp_len`.
    fn from(bytes: T, exp_len: usize) -> Result<Self, PetError> {
        (bytes.as_ref().len() == exp_len)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> MessageBoxBufferRef<'a> for UpdateBoxBuffer<&'a T> {
    /// Access the update box buffer by reference.
    fn bytes(&self) -> &'a [u8] {
        self.bytes.as_ref()
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> UpdateBoxBuffer<&'a T> {
    /// Access the update signature field of the update box buffer by reference.
    fn signature_update(&self) -> &'a [u8] {
        &self.bytes()[SIGN_UPDATE_RANGE]
    }

    /// Access the model url field of the update box buffer by reference.
    fn model_url(&self) -> &'a [u8] {
        &self.bytes()[MODEL_URL_RANGE]
    }

    /// Access the seed dictionary field of the update box buffer by reference.
    fn dict_seed(&self) -> &'a [u8] {
        let dict_seed_end = self.bytes().len();
        &self.bytes()[dict_seed_range(dict_seed_end)]
    }
}

impl<T: AsMut<[u8]>> MessageBoxBufferMut for UpdateBoxBuffer<T> {
    /// Access the update box buffer by mutable reference.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<T: AsMut<[u8]>> UpdateBoxBuffer<T> {
    /// Access the update signature field of the update box buffer by mutable reference.
    fn signature_update_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[SIGN_UPDATE_RANGE]
    }

    /// Access the model url field of the update box buffer by mutable reference.
    fn model_url_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[MODEL_URL_RANGE]
    }

    /// Access the seed dictionary field of the update box buffer by mutable reference.
    fn dict_seed_mut(&mut self) -> &mut [u8] {
        let dict_seed_end = self.bytes_mut().len();
        &mut self.bytes_mut()[dict_seed_range(dict_seed_end)]
    }
}

/// Encryption and decryption of update boxes.
pub struct UpdateBox {
    certificate: Vec<u8>,
    signature_sum: sign::Signature,
    signature_update: sign::Signature,
    model_url: Vec<u8>,
    dict_seed: HashMap<box_::PublicKey, Vec<u8>>,
}

impl UpdateBox {
    /// Serialize the seed dictionary field of the update box to bytes.
    fn serialize_dict_seed(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        for (key, seed) in self.dict_seed.iter() {
            bytes.extend_from_slice(key.as_ref());
            bytes.extend_from_slice(seed);
        }
        bytes
    }

    /// Deserialize the seed dictionary field of the update box from bytes.
    fn deserialize_dict_seed(bytes: &[u8]) -> HashMap<box_::PublicKey, Vec<u8>> {
        let mut dict_seed: HashMap<box_::PublicKey, Vec<u8>> = HashMap::new();
        for idx in (0..bytes.len()).step_by(DICT_SEED_ITEM_LENGTH) {
            dict_seed.insert(
                box_::PublicKey::from_slice(&bytes[idx..idx + DICT_SEED_KEY_LENGTH]).unwrap(),
                bytes[idx + DICT_SEED_KEY_LENGTH..idx + DICT_SEED_ITEM_LENGTH].to_vec(),
            );
        }
        dict_seed
    }
}

impl MessageBox for UpdateBox {
    /// Get the length of the serialized update box.
    fn len(&self) -> usize {
        DICT_SEED_START + DICT_SEED_ITEM_LENGTH * self.dict_seed.len()
    }

    /// Get the expected length of a serialized update box.
    fn exp_len(param: Option<usize>) -> usize {
        DICT_SEED_START + DICT_SEED_ITEM_LENGTH * param.unwrap()
    }

    /// Serialize the update box to bytes.
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = UpdateBoxBuffer::new(self.len());
        buffer.tag_mut().copy_from_slice([UPDATE_TAG; 1].as_ref());
        buffer.certificate_mut().copy_from_slice(&self.certificate);
        buffer
            .signature_sum_mut()
            .copy_from_slice(self.signature_sum.as_ref());
        buffer
            .signature_update_mut()
            .copy_from_slice(self.signature_update.as_ref());
        buffer.model_url_mut().copy_from_slice(&self.model_url);
        buffer
            .dict_seed_mut()
            .copy_from_slice(&self.serialize_dict_seed());
        buffer.bytes
    }

    /// Deserialize an update box from bytes. Fails if the `bytes` don't conform to the expected
    /// update box length `exp_len`.
    fn deserialize(bytes: &[u8], exp_len: usize) -> Result<Self, PetError> {
        let buffer = UpdateBoxBuffer::from(bytes, exp_len)?;
        let certificate = buffer.certificate().to_vec();
        let signature_sum = sign::Signature::from_slice(buffer.signature_sum()).unwrap();
        let signature_update = sign::Signature::from_slice(buffer.signature_update()).unwrap();
        let model_url = buffer.model_url().to_vec();
        let dict_seed = Self::deserialize_dict_seed(buffer.dict_seed());
        Ok(Self {
            certificate,
            signature_sum,
            signature_update,
            model_url,
            dict_seed,
        })
    }
}
