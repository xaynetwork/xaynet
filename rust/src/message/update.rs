use std::{borrow::Borrow, ops::Range};

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{Certificate, MessageBuffer, CERTIFICATE_BYTES, TAG_BYTES, UPDATE_TAG};
use crate::{
    CoordinatorPublicKey, CoordinatorSecretKey, LocalSeedDict, ParticipantTaskSignature, PetError,
    UpdateParticipantPublicKey, UpdateParticipantSecretKey,
};

// update message buffer field ranges
const UPDATE_SIGNATURE_RANGE: Range<usize> = 193..257; // 64 bytes
const MASKED_MODEL_BYTES: usize = 32;
const MASKED_MODEL_RANGE: Range<usize> = 257..289; // 32 bytes
const LOCAL_SEED_DICT_START: usize = 289;
const LOCAL_SEED_DICT_KEY_LENGTH: usize = 32; // 32 bytes
const LOCAL_SEED_DICT_ITEM_LENGTH: usize = 112; // 112 bytes

#[derive(Debug)]
/// Access to update message buffer fields.
struct UpdateMessageBuffer<B> {
    bytes: B,
}

impl UpdateMessageBuffer<Vec<u8>> {
    /// Create an empty update message buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }

    /// Create an update message buffer from `bytes`. Fails if the `bytes` don't conform to the
    /// expected update message length `exp_len`.
    fn try_from(bytes: Vec<u8>, exp_len: usize) -> Result<Self, PetError> {
        if bytes.len() != exp_len {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self { bytes })
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> MessageBuffer for UpdateMessageBuffer<B> {
    /// Get a reference to the update message buffer.
    fn bytes(&'_ self) -> &'_ [u8] {
        self.bytes.as_ref()
    }

    /// Get a mutable reference to the update message buffer.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> UpdateMessageBuffer<B> {
    /// Get a reference to the update signature field of the update message buffer.
    fn update_signature(&'_ self) -> &'_ [u8] {
        &self.bytes()[UPDATE_SIGNATURE_RANGE]
    }

    /// Get a mutable reference to the update signature field of the update message buffer.
    fn update_signature_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[UPDATE_SIGNATURE_RANGE]
    }

    /// Get a reference to the masked model field of the update message buffer.
    fn masked_model(&'_ self) -> &'_ [u8] {
        &self.bytes()[MASKED_MODEL_RANGE]
    }

    /// Get a mutable reference to the masked model field of the update message buffer.
    fn masked_model_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[MASKED_MODEL_RANGE]
    }

    /// Get a reference to the local seed dictionary field of the update message buffer.
    fn local_seed_dict(&'_ self) -> &'_ [u8] {
        &self.bytes()[LOCAL_SEED_DICT_START..]
    }

    /// Get a mutable reference to the local seed dictionary field of the update message buffer.
    fn local_seed_dict_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[LOCAL_SEED_DICT_START..]
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Encryption and decryption of update messages.
pub struct UpdateMessage<K, C, S, M, D>
where
    K: Borrow<UpdateParticipantPublicKey>,
    C: Borrow<Certificate>,
    S: Borrow<ParticipantTaskSignature>,
    M: Borrow<Vec<u8>>,
    D: Borrow<LocalSeedDict>,
{
    pk: K,
    certificate: C,
    sum_signature: S,
    update_signature: S,
    masked_model: M,
    local_seed_dict: D,
}

impl<K, C, S, M, D> UpdateMessage<K, C, S, M, D>
where
    K: Borrow<UpdateParticipantPublicKey>,
    C: Borrow<Certificate>,
    S: Borrow<ParticipantTaskSignature>,
    M: Borrow<Vec<u8>>,
    D: Borrow<LocalSeedDict>,
{
    /// Create an update message from its parts.
    pub fn from_parts(
        pk: K,
        certificate: C,
        sum_signature: S,
        update_signature: S,
        masked_model: M,
        local_seed_dict: D,
    ) -> Self {
        Self {
            pk,
            certificate,
            sum_signature,
            update_signature,
            masked_model,
            local_seed_dict,
        }
    }

    /// Get the expected length of a serialized sum message.
    fn exp_len(dict_length: usize) -> usize {
        sign::SIGNATUREBYTES
            + TAG_BYTES
            + box_::PUBLICKEYBYTES
            + sign::PUBLICKEYBYTES
            + CERTIFICATE_BYTES
            + sign::SIGNATUREBYTES
            + sign::SIGNATUREBYTES
            + MASKED_MODEL_BYTES
            + LOCAL_SEED_DICT_ITEM_LENGTH * dict_length
    }

    /// Serialize the local seed dictionary into bytes.
    fn serialize_local_seed_dict(&self) -> Vec<u8> {
        self.local_seed_dict
            .borrow()
            .iter()
            .flat_map(|(pk, seed)| [pk.as_ref(), seed].concat())
            .collect::<Vec<u8>>()
    }

    /// Serialize the update message into a buffer.
    fn serialize(&self, buffer: &mut UpdateMessageBuffer<Vec<u8>>, pk: &CoordinatorPublicKey) {
        buffer.tag_mut().copy_from_slice(&[UPDATE_TAG]);
        buffer.coord_pk_mut().copy_from_slice(pk.borrow().as_ref());
        buffer
            .part_pk_mut()
            .copy_from_slice(self.pk.borrow().as_ref());
        buffer
            .certificate_mut()
            .copy_from_slice(self.certificate.borrow().as_ref());
        buffer
            .sum_signature_mut()
            .copy_from_slice(self.sum_signature.borrow().as_ref());
        buffer
            .update_signature_mut()
            .copy_from_slice(self.update_signature.borrow().as_ref());
        buffer
            .masked_model_mut()
            .copy_from_slice(self.masked_model.borrow().as_ref());
        buffer
            .local_seed_dict_mut()
            .copy_from_slice(&self.serialize_local_seed_dict());
    }

    /// Sign and encrypt the update message.
    pub fn seal(&self, sk: &UpdateParticipantSecretKey, pk: &CoordinatorPublicKey) -> Vec<u8> {
        let mut buffer =
            UpdateMessageBuffer::new(Self::exp_len(self.local_seed_dict.borrow().len()));
        self.serialize(&mut buffer, pk);
        let signature = sign::sign_detached(buffer.message(), sk);
        buffer.signature_mut().copy_from_slice(signature.as_ref());
        sealedbox::seal(buffer.bytes(), pk)
    }
}

impl
    UpdateMessage<
        UpdateParticipantPublicKey,
        Certificate,
        ParticipantTaskSignature,
        Vec<u8>,
        LocalSeedDict,
    >
{
    /// Deserialize a local seed dictionary from bytes.
    fn deserialize_local_seed_dict(bytes: &[u8]) -> LocalSeedDict {
        bytes
            .chunks_exact(LOCAL_SEED_DICT_ITEM_LENGTH)
            .map(|chunk| {
                (
                    // safe unwrap: lengths of `chunk` slice is guaranteed by constants
                    sign::PublicKey::from_slice(&chunk[0..LOCAL_SEED_DICT_KEY_LENGTH]).unwrap(),
                    chunk[LOCAL_SEED_DICT_KEY_LENGTH..LOCAL_SEED_DICT_ITEM_LENGTH].to_vec(),
                )
            })
            .collect()
    }

    /// Deserialize an update message from a buffer. Fails if the `buffer` doesn't conform to the
    /// expected update message length `exp_len`.
    fn deserialize(buffer: UpdateMessageBuffer<Vec<u8>>) -> Self {
        // safe unwraps: lengths of `buffer` slices are guaranteed by constants
        let pk = sign::PublicKey::from_slice(buffer.part_pk()).unwrap();
        let certificate = buffer.certificate().into();
        let sum_signature = sign::Signature::from_slice(buffer.sum_signature()).unwrap();
        let update_signature = sign::Signature::from_slice(buffer.update_signature()).unwrap();
        let masked_model = buffer.masked_model().to_vec();
        let local_seed_dict = Self::deserialize_local_seed_dict(buffer.local_seed_dict());
        Self {
            pk,
            certificate,
            sum_signature,
            update_signature,
            masked_model,
            local_seed_dict,
        }
    }

    /// Decrypt and verify an update message. Fails if decryption or validation fails.
    pub fn open(
        bytes: &[u8],
        pk: &CoordinatorPublicKey,
        sk: &CoordinatorSecretKey,
        sum_dict_length: usize,
    ) -> Result<Self, PetError> {
        let buffer = UpdateMessageBuffer::try_from(
            sealedbox::open(bytes, pk, sk).or(Err(PetError::InvalidMessage))?,
            Self::exp_len(sum_dict_length),
        )?;
        if buffer.tag() != [UPDATE_TAG]
            || buffer.coord_pk() != pk.as_ref()
            || !sign::verify_detached(
                // safe unwraps: lengths of `buffer` slices are guaranteed by constants
                &sign::Signature::from_slice(buffer.signature()).unwrap(),
                buffer.message(),
                &sign::PublicKey::from_slice(buffer.part_pk()).unwrap(),
            )
        {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self::deserialize(buffer))
    }

    /// Get a reference to the public signature key.
    pub fn pk(&self) -> &UpdateParticipantPublicKey {
        &self.pk
    }

    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }

    /// Get a reference to the sum signature.
    pub fn sum_signature(&self) -> &ParticipantTaskSignature {
        &self.sum_signature
    }

    /// Get a reference to the update signature.
    pub fn update_signature(&self) -> &ParticipantTaskSignature {
        &self.update_signature
    }

    /// Get a reference to the masked model.
    pub fn masked_model(&self) -> &Vec<u8> {
        &self.masked_model
    }

    /// Get a reference to the local seed dictionary.
    pub fn local_seed_dict(&self) -> &LocalSeedDict {
        &self.local_seed_dict
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, iter};

    use sodiumoxide::randombytes::{randombytes, randombytes_uniform};

    use super::{
        super::{CERTIFICATE_RANGE, PART_PK_RANGE, SUM_SIGNATURE_RANGE},
        *,
    };

    #[test]
    fn test_ranges() {
        assert_eq!(
            UPDATE_SIGNATURE_RANGE.end - UPDATE_SIGNATURE_RANGE.start,
            sign::SIGNATUREBYTES,
        );
        assert_eq!(
            MASKED_MODEL_RANGE.end - MASKED_MODEL_RANGE.start,
            MASKED_MODEL_BYTES
        );
    }

    #[test]
    fn test_updatemessagebuffer() {
        // new
        assert_eq!(UpdateMessageBuffer::new(10).bytes, vec![0_u8; 10]);

        // try from
        let sum_dict_length = 1 + randombytes_uniform(10) as usize;
        let mut bytes = randombytes(289 + 112 * sum_dict_length);
        let mut buffer =
            UpdateMessageBuffer::try_from(bytes.clone(), 289 + 112 * sum_dict_length).unwrap();
        assert_eq!(buffer.bytes, bytes);
        assert_eq!(
            UpdateMessageBuffer::try_from(bytes.clone(), 10).unwrap_err(),
            PetError::InvalidMessage,
        );

        // update signature
        assert_eq!(buffer.update_signature(), &bytes[UPDATE_SIGNATURE_RANGE]);
        assert_eq!(
            buffer.update_signature_mut(),
            &mut bytes[UPDATE_SIGNATURE_RANGE],
        );

        // masked model
        assert_eq!(buffer.masked_model(), &bytes[MASKED_MODEL_RANGE]);
        assert_eq!(buffer.masked_model_mut(), &mut bytes[MASKED_MODEL_RANGE]);

        // local seed dictionary
        assert_eq!(buffer.local_seed_dict(), &bytes[LOCAL_SEED_DICT_START..]);
        assert_eq!(
            buffer.local_seed_dict_mut(),
            &mut bytes[LOCAL_SEED_DICT_START..]
        );
    }

    #[test]
    fn test_updatemessage_serialize() {
        // from parts
        let sum_dict_length = 1 + randombytes_uniform(10) as usize;
        let pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let certificate = &Vec::<u8>::new().into();
        let sum_signature = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let update_signature = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let masked_model = &randombytes(32);
        let local_seed_dict = &iter::repeat_with(|| {
            (
                sign::PublicKey::from_slice(&randombytes(32)).unwrap(),
                randombytes(80),
            )
        })
        .take(sum_dict_length)
        .collect();
        let msg = UpdateMessage::from_parts(
            pk,
            certificate,
            sum_signature,
            update_signature,
            masked_model,
            local_seed_dict,
        );
        assert_eq!(
            msg.pk as *const sign::PublicKey,
            pk as *const sign::PublicKey,
        );
        assert_eq!(
            msg.certificate as *const Certificate,
            certificate as *const Certificate,
        );
        assert_eq!(
            msg.sum_signature as *const sign::Signature,
            sum_signature as *const sign::Signature,
        );
        assert_eq!(
            msg.update_signature as *const sign::Signature,
            update_signature as *const sign::Signature,
        );
        assert_eq!(
            msg.masked_model as *const Vec<u8>,
            masked_model as *const Vec<u8>
        );
        assert_eq!(
            msg.local_seed_dict as *const HashMap<sign::PublicKey, Vec<u8>>,
            local_seed_dict as *const HashMap<sign::PublicKey, Vec<u8>>,
        );

        // serialize seed dictionary
        let local_seed_vec = msg.serialize_local_seed_dict();
        assert_eq!(
            local_seed_vec.len(),
            LOCAL_SEED_DICT_ITEM_LENGTH * sum_dict_length
        );
        assert!(local_seed_vec
            .chunks_exact(LOCAL_SEED_DICT_ITEM_LENGTH)
            .all(|chunk| {
                local_seed_dict
                    .get(
                        &sign::PublicKey::from_slice(&chunk[0..LOCAL_SEED_DICT_KEY_LENGTH])
                            .unwrap(),
                    )
                    .unwrap()
                    .as_slice()
                    == &chunk[LOCAL_SEED_DICT_KEY_LENGTH..LOCAL_SEED_DICT_ITEM_LENGTH]
            }));

        // serialize
        let mut buffer = UpdateMessageBuffer::new(289 + 112 * sum_dict_length);
        let coord_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        msg.serialize(&mut buffer, &coord_pk);
        assert_eq!(buffer.tag(), &[UPDATE_TAG]);
        assert_eq!(buffer.coord_pk(), coord_pk.as_ref());
        assert_eq!(buffer.part_pk(), pk.as_ref());
        assert_eq!(buffer.certificate(), certificate.as_ref());
        assert_eq!(buffer.sum_signature(), sum_signature.as_ref());
        assert_eq!(buffer.update_signature(), update_signature.as_ref());
        assert_eq!(buffer.masked_model(), masked_model.as_slice());
        assert_eq!(buffer.local_seed_dict(), local_seed_vec.as_slice());
    }

    #[test]
    fn test_updatemessage_deserialize() {
        // deserialize seed dictionary
        let sum_dict_length = 1 + randombytes_uniform(10) as usize;
        let local_seed_vec = randombytes(LOCAL_SEED_DICT_ITEM_LENGTH * sum_dict_length);
        let local_seed_dict = UpdateMessage::deserialize_local_seed_dict(&local_seed_vec);
        for chunk in local_seed_vec.chunks_exact(LOCAL_SEED_DICT_ITEM_LENGTH) {
            assert_eq!(
                local_seed_dict
                    .get(
                        &sign::PublicKey::from_slice(&chunk[0..LOCAL_SEED_DICT_KEY_LENGTH])
                            .unwrap()
                    )
                    .unwrap(),
                &chunk[LOCAL_SEED_DICT_KEY_LENGTH..LOCAL_SEED_DICT_ITEM_LENGTH].to_vec(),
            );
        }

        // deserialize
        let bytes = randombytes(289 + 112 * sum_dict_length);
        let buffer =
            UpdateMessageBuffer::try_from(bytes.clone(), 289 + 112 * sum_dict_length).unwrap();
        let msg = UpdateMessage::deserialize(buffer);
        assert_eq!(
            msg.pk(),
            &sign::PublicKey::from_slice(&bytes[PART_PK_RANGE]).unwrap(),
        );
        assert_eq!(msg.certificate(), &bytes[CERTIFICATE_RANGE].into());
        assert_eq!(
            msg.sum_signature(),
            &sign::Signature::from_slice(&bytes[SUM_SIGNATURE_RANGE]).unwrap(),
        );
        assert_eq!(
            msg.update_signature(),
            &sign::Signature::from_slice(&bytes[UPDATE_SIGNATURE_RANGE]).unwrap(),
        );
        assert_eq!(msg.masked_model(), &bytes[MASKED_MODEL_RANGE].to_vec());
        assert_eq!(
            msg.local_seed_dict(),
            &UpdateMessage::deserialize_local_seed_dict(&bytes[LOCAL_SEED_DICT_START..]),
        );
    }

    #[test]
    fn test_updatemessage() {
        // seal
        let sum_dict_length = 1 + randombytes_uniform(10) as usize;
        let (pk, sk) = sign::gen_keypair();
        let certificate = Vec::<u8>::new().into();
        let sum_signature = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let update_signature = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let masked_model = randombytes(32);
        let local_seed_dict = iter::repeat_with(|| {
            (
                sign::PublicKey::from_slice(&randombytes(32)).unwrap(),
                randombytes(80),
            )
        })
        .take(sum_dict_length)
        .collect();
        let (coord_pk, coord_sk) = box_::gen_keypair();
        let bytes = UpdateMessage::from_parts(
            &pk,
            &certificate,
            &sum_signature,
            &update_signature,
            &masked_model,
            &local_seed_dict,
        )
        .seal(&sk, &coord_pk);

        // open
        let msg = UpdateMessage::open(&bytes, &coord_pk, &coord_sk, sum_dict_length).unwrap();
        assert_eq!(msg.pk(), &pk);
        assert_eq!(msg.certificate(), &certificate);
        assert_eq!(msg.sum_signature(), &sum_signature);
        assert_eq!(msg.update_signature(), &update_signature);
        assert_eq!(msg.masked_model(), &masked_model);
        assert_eq!(msg.local_seed_dict(), &local_seed_dict);

        // wrong signature
        let mut buffer = UpdateMessageBuffer::new(289 + 112 * sum_dict_length);
        let msg = UpdateMessage::from_parts(
            &pk,
            &certificate,
            &sum_signature,
            &update_signature,
            &masked_model,
            &local_seed_dict,
        );
        msg.serialize(&mut buffer, &coord_pk);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk, sum_dict_length).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong receiver
        msg.serialize(
            &mut buffer,
            &box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
        );
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk, sum_dict_length).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong tag
        buffer.tag_mut().copy_from_slice(&[0_u8]);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk, sum_dict_length).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong length
        let buffer = UpdateMessageBuffer::new(10);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk, sum_dict_length).unwrap_err(),
            PetError::InvalidMessage,
        );
    }
}
