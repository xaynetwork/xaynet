#![allow(dead_code)] // temporary

use std::{collections::HashMap, ops::Range};

use sodiumoxide::crypto::{box_, sign};

use super::{MsgBoxBufMut, MsgBoxBufRef, MsgBoxDecr, MsgBoxEncr, UPDATE_TAG};
use crate::pet::PetError;

// update box field ranges
const SIGN_UPDATE_RANGE: Range<usize> = 65..129; // 64 bytes
const MODEL_URL_RANGE: Range<usize> = 129..161; // 32 bytes
const DICT_SEED_START: usize = 161;
const DICT_SEED_KEY_LENGTH: usize = 32; // 32 bytes
const DICT_SEED_ITEM_LENGTH: usize = 112; // 112 bytes

/// Mutable and immutable buffer access to update box fields.
struct UpdateBoxBuffer<B> {
    bytes: B,
}

impl UpdateBoxBuffer<Vec<u8>> {
    /// Create an empty update box buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }
}

impl<B: AsRef<[u8]>> UpdateBoxBuffer<B> {
    /// Create an update box buffer from `bytes`. Fails if the `bytes` don't conform to the expected
    /// update box length `exp_len`.
    fn from(bytes: B, exp_len: usize) -> Result<Self, PetError> {
        (bytes.as_ref().len() == exp_len)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'b, B: AsRef<[u8]> + ?Sized> MsgBoxBufRef<'b> for UpdateBoxBuffer<&'b B> {
    /// Access the update box buffer by reference.
    fn bytes(&self) -> &'b [u8] {
        self.bytes.as_ref()
    }
}

impl<'b, B: AsRef<[u8]> + ?Sized> UpdateBoxBuffer<&'b B> {
    /// Access the update signature field of the update box buffer by reference.
    fn signature_update(&self) -> &'b [u8] {
        &self.bytes()[SIGN_UPDATE_RANGE]
    }

    /// Access the model url field of the update box buffer by reference.
    fn model_url(&self) -> &'b [u8] {
        &self.bytes()[MODEL_URL_RANGE]
    }

    /// Access the seed dictionary field of the update box buffer by reference.
    fn dict_seed(&self) -> &'b [u8] {
        &self.bytes()[DICT_SEED_START..]
    }
}

impl<B: AsMut<[u8]>> MsgBoxBufMut for UpdateBoxBuffer<B> {
    /// Access the update box buffer by mutable reference.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<B: AsMut<[u8]>> UpdateBoxBuffer<B> {
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
        &mut self.bytes_mut()[DICT_SEED_START..]
    }
}

/// Encryption and decryption of update boxes.
pub struct UpdateBox<C, S, M, D> {
    certificate: C,
    signature_sum: S,
    signature_update: S,
    model_url: M,
    dict_seed: D,
}

impl<'b> UpdateBox<&'b [u8], &'b sign::Signature, &'b [u8], &'b HashMap<box_::PublicKey, Vec<u8>>> {
    /// Create an update box.
    pub fn new(
        certificate: &'b [u8],
        signature_sum: &'b sign::Signature,
        signature_update: &'b sign::Signature,
        model_url: &'b [u8],
        dict_seed: &'b HashMap<box_::PublicKey, Vec<u8>>,
    ) -> Self {
        Self {
            certificate,
            signature_sum,
            signature_update,
            model_url,
            dict_seed,
        }
    }

    /// Serialize the seed dictionary field of the update box to bytes.
    fn serialize_dict_seed(&self) -> Vec<u8> {
        self.dict_seed
            .iter()
            .flat_map(|(pk, seed)| [pk.as_ref(), seed].concat())
            .collect::<Vec<u8>>()
    }
}

impl MsgBoxEncr for UpdateBox<&[u8], &sign::Signature, &[u8], &HashMap<box_::PublicKey, Vec<u8>>> {
    /// Get the length of the serialized update box.
    fn len(&self) -> usize {
        // 161 + 112 * len(dict_seed) bytes
        1 + 0 + 2 * sign::SIGNATUREBYTES + 32 + DICT_SEED_ITEM_LENGTH * self.dict_seed.len()
    }

    /// Serialize the update box to bytes.
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = UpdateBoxBuffer::new(self.len());
        buffer.tag_mut().copy_from_slice([UPDATE_TAG].as_ref());
        buffer.certificate_mut().copy_from_slice(self.certificate);
        buffer
            .signature_sum_mut()
            .copy_from_slice(self.signature_sum.as_ref());
        buffer
            .signature_update_mut()
            .copy_from_slice(self.signature_update.as_ref());
        buffer.model_url_mut().copy_from_slice(self.model_url);
        buffer
            .dict_seed_mut()
            .copy_from_slice(&self.serialize_dict_seed());
        buffer.bytes
    }
}

impl UpdateBox<Vec<u8>, sign::Signature, Vec<u8>, HashMap<box_::PublicKey, Vec<u8>>> {
    /// Deserialize the seed dictionary field of the update box from bytes.
    fn deserialize_dict_seed(bytes: &[u8]) -> HashMap<box_::PublicKey, Vec<u8>> {
        bytes
            .chunks_exact(DICT_SEED_ITEM_LENGTH)
            .map(|chunk| {
                (
                    box_::PublicKey::from_slice(&chunk[0..DICT_SEED_KEY_LENGTH]).unwrap(),
                    chunk[DICT_SEED_KEY_LENGTH..DICT_SEED_ITEM_LENGTH].to_vec(),
                )
            })
            .collect()
    }

    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &[u8] {
        &self.certificate
    }

    /// Get a reference to the sum signature.
    pub fn signature_sum(&self) -> &sign::Signature {
        &self.signature_sum
    }

    /// Get a reference to the update signature.
    pub fn signature_update(&self) -> &sign::Signature {
        &self.signature_update
    }

    /// Get a reference to the model url.
    pub fn model_url(&self) -> &[u8] {
        &self.model_url
    }

    /// Get a reference to the seed dictionary.
    pub fn dict_seed(&self) -> &HashMap<box_::PublicKey, Vec<u8>> {
        &self.dict_seed
    }
}

impl MsgBoxDecr
    for UpdateBox<Vec<u8>, sign::Signature, Vec<u8>, HashMap<box_::PublicKey, Vec<u8>>>
{
    /// Get the expected length of a serialized update box, where `param` is the length of the
    /// dictionary of sum participants.
    fn exp_len(param: Option<usize>) -> usize {
        // 161 + 112 * len(dict_sum) bytes
        1 + 0 + 2 * sign::SIGNATUREBYTES + 32 + DICT_SEED_ITEM_LENGTH * param.unwrap()
    }

    /// Deserialize an update box from bytes. Fails if the `bytes` don't conform to the expected
    /// update box length `exp_len`.
    fn deserialize(bytes: &[u8], exp_len: usize) -> Result<Self, PetError> {
        let buffer = UpdateBoxBuffer::from(bytes, exp_len)?;
        (buffer.tag() == [UPDATE_TAG])
            .then_some(())
            .ok_or(PetError::InvalidMessage)?;
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
