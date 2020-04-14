use std::{collections::HashMap, ops::Range};

use sodiumoxide::crypto::{box_, sign};

use super::{MsgBoxBufMut, MsgBoxBufRef, MsgBoxDecr, MsgBoxEncr, UPDATE_TAG};
use crate::PetError;

// update box field ranges
const SIGN_UPDATE_RANGE: Range<usize> = 65..129; // 64 bytes
const MODEL_URL_RANGE: Range<usize> = 129..161; // 32 bytes
const DICT_SEED_START: usize = 161;
const DICT_SEED_KEY_LENGTH: usize = 32; // 32 bytes
const DICT_SEED_ITEM_LENGTH: usize = 112; // 112 bytes

#[derive(Debug)]
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

#[derive(Clone, Debug, PartialEq)]
/// Encryption and decryption of update boxes.
pub struct UpdateBox<C, S, M, D> {
    certificate: C,
    signature_sum: S,
    signature_update: S,
    model_url: M,
    dict_seed: D,
}

#[allow(clippy::implicit_hasher)]
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

#[allow(clippy::implicit_hasher)]
impl MsgBoxEncr for UpdateBox<&[u8], &sign::Signature, &[u8], &HashMap<box_::PublicKey, Vec<u8>>> {
    /// Get the length of the serialized update box.
    fn len(&self) -> usize {
        // 161 + 112 * len(dict_seed) bytes
        1 + self.certificate.len()
            + self.signature_sum.as_ref().len()
            + self.signature_update.as_ref().len()
            + self.model_url.len()
            + DICT_SEED_ITEM_LENGTH * self.dict_seed.len()
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

#[allow(clippy::implicit_hasher)]
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

#[allow(clippy::implicit_hasher)]
impl MsgBoxDecr
    for UpdateBox<Vec<u8>, sign::Signature, Vec<u8>, HashMap<box_::PublicKey, Vec<u8>>>
{
    #[allow(clippy::identity_op)] // temporary
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

#[cfg(test)]
mod tests {
    use std::iter;

    use sodiumoxide::randombytes::{randombytes, randombytes_uniform};

    use super::*;

    #[test]
    fn test_updatebox_field_ranges() {
        assert_eq!(SIGN_UPDATE_RANGE.end - SIGN_UPDATE_RANGE.start, 64);
        assert_eq!(MODEL_URL_RANGE.end - MODEL_URL_RANGE.start, 32);
        assert_eq!(DICT_SEED_START, 161);
        assert_eq!(DICT_SEED_KEY_LENGTH, 32);
        assert_eq!(DICT_SEED_ITEM_LENGTH, 112);
    }

    #[test]
    fn test_updateboxbuffer() {
        // new
        assert_eq!(UpdateBoxBuffer::new(10).bytes, vec![0_u8; 10]);

        // from
        let dict_sum_len = 1 + randombytes_uniform(10) as usize;
        let len = 161 + 112 * dict_sum_len;
        let bytes = randombytes(len);
        let bytes_ = bytes.clone();
        let mut bytes_mut = bytes.clone();
        let mut bytes_mut_ = bytes.clone();
        assert_eq!(
            UpdateBoxBuffer::from(bytes.clone(), len).unwrap().bytes,
            bytes.clone(),
        );
        assert_eq!(
            UpdateBoxBuffer::from(&bytes, len).unwrap().bytes as *const Vec<u8>,
            &bytes as *const Vec<u8>,
        );
        assert_eq!(
            UpdateBoxBuffer::from(&mut bytes_mut, len).unwrap().bytes as *mut Vec<u8>,
            &mut bytes_mut as *mut Vec<u8>,
        );
        assert_eq!(
            UpdateBoxBuffer::from(&bytes, 10).unwrap_err(),
            PetError::InvalidMessage,
        );

        // bytes
        let buf = UpdateBoxBuffer::from(&bytes, len).unwrap();
        let mut buf_mut = UpdateBoxBuffer::from(&mut bytes_mut, len).unwrap();
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

        // signature update
        assert_eq!(buf.signature_update(), &bytes_[65..129]);
        assert_eq!(buf_mut.signature_update_mut(), &mut bytes_mut_[65..129]);

        // signature update
        assert_eq!(buf.model_url(), &bytes_[129..161]);
        assert_eq!(buf_mut.model_url_mut(), &mut bytes_mut_[129..161]);

        // dict seed
        assert_eq!(buf.dict_seed(), &bytes_[161..len]);
        assert_eq!(buf_mut.dict_seed_mut(), &mut bytes_mut_[161..len]);
    }

    #[test]
    fn test_updatebox_ref() {
        // new
        let dict_sum_len = 1 + randombytes_uniform(10) as usize;
        let certificate = Vec::<u8>::new();
        let signature_sum = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let signature_update = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let model_url = randombytes(32);
        let dict_seed = &iter::repeat_with(|| {
            (
                box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
                randombytes(80),
            )
        })
        .take(dict_sum_len)
        .collect();
        let ubox = UpdateBox::new(
            &certificate,
            signature_sum,
            signature_update,
            &model_url,
            dict_seed,
        );
        assert_eq!(ubox.certificate, certificate.as_slice());
        assert_eq!(
            ubox.signature_sum as *const sign::Signature,
            signature_sum as *const sign::Signature,
        );
        assert_eq!(
            ubox.signature_update as *const sign::Signature,
            signature_update as *const sign::Signature,
        );
        assert_eq!(ubox.model_url, model_url.as_slice());
        assert_eq!(
            ubox.dict_seed as *const HashMap<box_::PublicKey, Vec<u8>>,
            dict_seed as *const HashMap<box_::PublicKey, Vec<u8>>,
        );

        // len
        assert_eq!(ubox.len(), 161 + 112 * dict_sum_len);

        // serialize dict seed
        let vec_seed = ubox.serialize_dict_seed();
        assert_eq!(vec_seed.len(), 112 * dict_sum_len);
        assert!(vec_seed.chunks_exact(112).all(|chunk| {
            dict_seed
                .get(&box_::PublicKey::from_slice(&chunk[0..32]).unwrap())
                .unwrap()
                .as_slice()
                == &chunk[32..112]
        }));

        // serialize
        assert_eq!(
            ubox.serialize(),
            [
                [102_u8; 1].as_ref(),
                certificate.as_slice(),
                signature_sum.as_ref(),
                signature_update.as_ref(),
                model_url.as_slice(),
                vec_seed.as_slice(),
            ]
            .concat(),
        );
    }

    #[test]
    fn test_updatebox_val() {
        // exp len
        let dict_sum_len = 1 + randombytes_uniform(10) as usize;
        let len = 161 + 112 * dict_sum_len;
        assert_eq!(UpdateBox::exp_len(Some(dict_sum_len)), len);

        // deserialize dict seed
        let certificate = Vec::<u8>::new();
        let signature_sum = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let signature_update = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let model_url = randombytes(32);
        let dict_seed = iter::repeat_with(|| {
            (
                box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
                randombytes(80),
            )
        })
        .take(dict_sum_len)
        .collect::<HashMap<box_::PublicKey, Vec<u8>>>();
        let vec_seed = UpdateBox::new(
            &certificate,
            &signature_sum,
            &signature_update,
            &model_url,
            &dict_seed,
        )
        .serialize_dict_seed();
        assert_eq!(UpdateBox::deserialize_dict_seed(&vec_seed), dict_seed);

        // deserialize
        let bytes = [
            [102_u8; 1].as_ref(),
            certificate.as_slice(),
            signature_sum.as_ref(),
            signature_update.as_ref(),
            model_url.as_slice(),
            vec_seed.as_slice(),
        ]
        .concat();
        let ubox = UpdateBox::deserialize(&bytes, len).unwrap();
        assert_eq!(
            ubox,
            UpdateBox {
                certificate: certificate.clone(),
                signature_sum,
                signature_update,
                model_url: model_url.clone(),
                dict_seed: dict_seed.clone(),
            },
        );
        assert_eq!(
            UpdateBox::deserialize(&vec![0_u8; 10], len).unwrap_err(),
            PetError::InvalidMessage,
        );
        assert_eq!(
            UpdateBox::deserialize(&vec![0_u8; len], len).unwrap_err(),
            PetError::InvalidMessage,
        );

        // certificate
        assert_eq!(ubox.certificate(), certificate.as_slice());

        // signature sum
        assert_eq!(ubox.signature_sum(), &signature_sum);

        // signature update
        assert_eq!(ubox.signature_update(), &signature_update);

        // model url
        assert_eq!(ubox.model_url(), model_url.as_slice());

        // dict seed
        assert_eq!(ubox.dict_seed(), &dict_seed);
    }

    #[test]
    fn test_updatebox() {
        let dict_sum_len = 1 + randombytes_uniform(10) as usize;
        let certificate = Vec::<u8>::new();
        let signature_sum = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let signature_update = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let model_url = randombytes(32);
        let dict_seed = iter::repeat_with(|| {
            (
                box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
                randombytes(80),
            )
        })
        .take(dict_sum_len)
        .collect::<HashMap<box_::PublicKey, Vec<u8>>>();
        let (pk, sk) = box_::gen_keypair();
        let (nonce, bytes) = UpdateBox::new(
            &certificate,
            &signature_sum,
            &signature_update,
            &model_url,
            &dict_seed,
        )
        .seal(&pk, &sk);
        let ubox = UpdateBox::open(&bytes, &nonce, &pk, &sk, 161 + 112 * dict_sum_len).unwrap();
        assert_eq!(
            ubox,
            UpdateBox {
                certificate,
                signature_sum,
                signature_update,
                model_url,
                dict_seed,
            },
        );
    }
}
