use std::{
    borrow::Borrow,
    convert::{TryFrom, TryInto},
    default::Default,
    ops::Range,
};

use sodiumoxide::crypto::{sealedbox, sign};

use super::{MessageBuffer, LEN_BYTES, PK_BYTES, SIGNATURE_BYTES, TAG_BYTES, UPDATE_TAG};
use crate::{
    certificate::Certificate,
    mask::{EncrMaskSeed, MaskedModel},
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    LocalSeedDict,
    ParticipantTaskSignature,
    PetError,
    SumParticipantEphemeralPublicKey,
    SumParticipantEphemeralSecretKey,
    UpdateParticipantPublicKey,
    UpdateParticipantSecretKey,
};

#[derive(Clone, Debug)]
/// Access to update message buffer fields.
struct UpdateMessageBuffer<B> {
    bytes: B,
    certificate_range: Range<usize>,
    sum_signature_range: Range<usize>,
    update_signature_range: Range<usize>,
    masked_model_len_range: Range<usize>,
    masked_model_range: Range<usize>,
    local_seed_dict_len_range: Range<usize>,
    local_seed_dict_range: Range<usize>,
}

impl Default for UpdateMessageBuffer<Vec<u8>> {
    fn default() -> Self {
        let bytes = Vec::<u8>::new();
        let certificate_range = 0..0;
        let sum_signature_range = 0..0;
        let update_signature_range = 0..0;
        let masked_model_len_range = 0..0;
        let masked_model_range = 0..0;
        let local_seed_dict_len_range = 0..0;
        let local_seed_dict_range = 0..0;
        Self {
            bytes,
            certificate_range,
            sum_signature_range,
            update_signature_range,
            masked_model_len_range,
            masked_model_range,
            local_seed_dict_len_range,
            local_seed_dict_range,
        }
    }
}

impl UpdateMessageBuffer<Vec<u8>> {
    /// Create an empty update message buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
            ..Default::default()
        }
    }
}

impl TryFrom<Vec<u8>> for UpdateMessageBuffer<Vec<u8>> {
    type Error = PetError;

    /// Create an update message buffer from `bytes`. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let mut buffer = Self {
            bytes,
            ..Default::default()
        };
        let mut offset = buffer.certificate_len_range().end;
        if buffer.len() >= offset {
            buffer.certificate_range = offset..offset + buffer.certificate_bytes();
            offset = buffer.certificate_range().end;
            buffer.sum_signature_range = offset..offset + SIGNATURE_BYTES;
            offset = buffer.sum_signature_range().end;
            buffer.update_signature_range = offset..offset + SIGNATURE_BYTES;
            offset = buffer.update_signature_range().end;
            buffer.masked_model_len_range = offset..offset + LEN_BYTES;
            offset = buffer.masked_model_len_range().end;
        } else {
            return Err(PetError::InvalidMessage);
        }
        if buffer.len() >= offset {
            buffer.masked_model_range = offset..offset + buffer.masked_model_bytes();
            offset = buffer.masked_model_range().end;
            buffer.local_seed_dict_len_range = offset..offset + LEN_BYTES;
            offset = buffer.local_seed_dict_len_range().end;
        } else {
            return Err(PetError::InvalidMessage);
        }
        if buffer.len() >= offset
            && buffer.local_seed_dict_bytes() % (PK_BYTES + EncrMaskSeed::BYTES) == 0
        {
            buffer.local_seed_dict_range = offset..offset + buffer.local_seed_dict_bytes();
            offset = buffer.local_seed_dict_range().end;
        } else {
            return Err(PetError::InvalidMessage);
        }
        if buffer.len() == offset {
            Ok(buffer)
        } else {
            Err(PetError::InvalidMessage)
        }
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

    /// Get the range of the certificate field.
    fn certificate_range(&self) -> Range<usize> {
        self.certificate_range.clone()
    }

    /// Get the range of the sum signature field.
    fn sum_signature_range(&self) -> Range<usize> {
        self.sum_signature_range.clone()
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> UpdateMessageBuffer<B> {
    /// Get the range of the update signature field.
    fn update_signature_range(&self) -> Range<usize> {
        self.update_signature_range.clone()
    }

    /// Get a reference to the update signature field.
    fn update_signature(&'_ self) -> &'_ [u8] {
        let range = self.update_signature_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the update signature field.
    fn update_signature_mut(&mut self) -> &mut [u8] {
        let range = self.update_signature_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the range of the masked model length field.
    fn masked_model_len_range(&self) -> Range<usize> {
        self.masked_model_len_range.clone()
    }

    /// Get a reference to the masked model length field.
    fn masked_model_len(&'_ self) -> &'_ [u8] {
        let range = self.masked_model_len_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the masked model length field.
    fn masked_model_len_mut(&mut self) -> &mut [u8] {
        let range = self.masked_model_len_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the number of bytes of the masked model field.
    fn masked_model_bytes(&self) -> usize {
        // safe unwrap: length of slice is guaranteed by constants
        usize::from_le_bytes(self.masked_model_len().try_into().unwrap())
    }

    /// Get the range of the masked model field.
    fn masked_model_range(&self) -> Range<usize> {
        self.masked_model_range.clone()
    }

    /// Get a reference to the masked model field.
    fn masked_model(&'_ self) -> &'_ [u8] {
        let range = self.masked_model_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the masked model field.
    fn masked_model_mut(&mut self) -> &mut [u8] {
        let range = self.masked_model_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the range of the local seed dictionary length field.
    fn local_seed_dict_len_range(&self) -> Range<usize> {
        self.local_seed_dict_len_range.clone()
    }

    /// Get a reference to the local seed dictionary length field.
    fn local_seed_dict_len(&'_ self) -> &'_ [u8] {
        let range = self.local_seed_dict_len_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the local seed dictionary length field.
    fn local_seed_dict_len_mut(&mut self) -> &mut [u8] {
        let range = self.local_seed_dict_len_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the number of bytes of the local seed dictionary field.
    fn local_seed_dict_bytes(&self) -> usize {
        // safe unwrap: length of slice is guaranteed by constants
        usize::from_le_bytes(self.local_seed_dict_len().try_into().unwrap())
    }

    /// Get the range of the local seed dictionary field.
    fn local_seed_dict_range(&self) -> Range<usize> {
        self.local_seed_dict_range.clone()
    }

    /// Get a reference to the local seed dictionary field.
    fn local_seed_dict(&'_ self) -> &'_ [u8] {
        let range = self.local_seed_dict_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the local seed dictionary field.
    fn local_seed_dict_mut(&mut self) -> &mut [u8] {
        let range = self.local_seed_dict_range();
        &mut self.bytes_mut()[range]
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Encryption and decryption of update messages.
pub struct UpdateMessage<K, C, S, M, D>
where
    K: Borrow<UpdateParticipantPublicKey>,
    C: Borrow<Certificate>,
    S: Borrow<ParticipantTaskSignature>,
    M: Borrow<MaskedModel>,
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
    M: Borrow<MaskedModel>,
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

    /// Get the length of a serialized update message.
    fn len(&self) -> usize {
        SIGNATURE_BYTES
            + TAG_BYTES
            + PK_BYTES
            + PK_BYTES
            + LEN_BYTES
            + self.certificate.borrow().len()
            + SIGNATURE_BYTES
            + SIGNATURE_BYTES
            + LEN_BYTES
            + self.masked_model.borrow().len()
            + LEN_BYTES
            + (PK_BYTES + EncrMaskSeed::BYTES) * self.local_seed_dict.borrow().len()
    }

    /// Serialize the local seed dictionary into bytes.
    fn serialize_local_seed_dict(&self) -> Vec<u8> {
        self.local_seed_dict
            .borrow()
            .iter()
            .flat_map(|(pk, seed)| [pk.as_ref(), seed.as_ref()].concat())
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
            .certificate_len_mut()
            .copy_from_slice(&self.certificate.borrow().len().to_le_bytes());
        let mut offset = buffer.certificate_len_range().end;
        buffer.certificate_range = offset..offset + buffer.certificate_bytes();
        buffer
            .certificate_mut()
            .copy_from_slice(self.certificate.borrow().as_ref());
        offset = buffer.certificate_range().end;
        buffer.sum_signature_range = offset..offset + SIGNATURE_BYTES;
        buffer
            .sum_signature_mut()
            .copy_from_slice(self.sum_signature.borrow().as_ref());
        offset = buffer.sum_signature_range().end;
        buffer.update_signature_range = offset..offset + SIGNATURE_BYTES;
        buffer
            .update_signature_mut()
            .copy_from_slice(self.update_signature.borrow().as_ref());
        offset = buffer.update_signature_range().end;
        buffer.masked_model_len_range = offset..offset + LEN_BYTES;
        buffer
            .masked_model_len_mut()
            .copy_from_slice(&self.masked_model.borrow().len().to_le_bytes());
        offset = buffer.masked_model_len_range().end;
        buffer.masked_model_range = offset..offset + buffer.masked_model_bytes();
        buffer
            .masked_model_mut()
            .copy_from_slice(self.masked_model.borrow().as_ref());
        offset = buffer.masked_model_range().end;
        buffer.local_seed_dict_len_range = offset..offset + LEN_BYTES;
        buffer.local_seed_dict_len_mut().copy_from_slice(
            &((PK_BYTES + EncrMaskSeed::BYTES) * self.local_seed_dict.borrow().len()).to_le_bytes(),
        );
        offset = buffer.local_seed_dict_len_range().end;
        buffer.local_seed_dict_range = offset..offset + buffer.local_seed_dict_bytes();
        buffer
            .local_seed_dict_mut()
            .copy_from_slice(&self.serialize_local_seed_dict());
    }

    /// Sign and encrypt the update message.
    pub fn seal(&self, sk: &UpdateParticipantSecretKey, pk: &CoordinatorPublicKey) -> Vec<u8> {
        let mut buffer = UpdateMessageBuffer::new(self.len());
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
        MaskedModel,
        LocalSeedDict,
    >
{
    /// Deserialize a local seed dictionary from bytes.
    fn deserialize_local_seed_dict(bytes: &[u8]) -> LocalSeedDict {
        bytes
            .chunks_exact(PK_BYTES + EncrMaskSeed::BYTES)
            .map(|chunk| {
                (
                    // safe unwraps: lengths of slices are guaranteed by constants
                    sign::PublicKey::from_slice(&chunk[..PK_BYTES]).unwrap(),
                    EncrMaskSeed::try_from(&chunk[PK_BYTES..]).unwrap(),
                )
            })
            .collect()
    }

    /// Deserialize an update message from a buffer.
    fn deserialize(buffer: UpdateMessageBuffer<Vec<u8>>) -> Self {
        // safe unwraps: lengths of slices are guaranteed by constants
        let pk = sign::PublicKey::from_slice(buffer.part_pk()).unwrap();
        let certificate = buffer.certificate().into();
        let sum_signature = sign::Signature::from_slice(buffer.sum_signature()).unwrap();
        let update_signature = sign::Signature::from_slice(buffer.update_signature()).unwrap();
        let masked_model = buffer.masked_model().into();
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
    ) -> Result<Self, PetError> {
        let buffer = UpdateMessageBuffer::try_from(
            sealedbox::open(bytes, pk, sk).or(Err(PetError::InvalidMessage))?,
        )?;
        if buffer.tag() == [UPDATE_TAG]
            && buffer.coord_pk() == pk.as_ref()
            && sign::verify_detached(
                // safe unwraps: lengths of slices are guaranteed by constants
                &sign::Signature::from_slice(buffer.signature()).unwrap(),
                buffer.message(),
                &sign::PublicKey::from_slice(buffer.part_pk()).unwrap(),
            )
        {
            Ok(Self::deserialize(buffer))
        } else {
            Err(PetError::InvalidMessage)
        }
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
    pub fn masked_model(&self) -> &MaskedModel {
        &self.masked_model
    }

    /// Get a reference to the local seed dictionary.
    pub fn local_seed_dict(&self) -> &LocalSeedDict {
        &self.local_seed_dict
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use sodiumoxide::{
        crypto::box_,
        randombytes::{randombytes, randombytes_uniform},
    };

    use super::*;

    fn auxiliary_bytes(sum_dict_len: usize) -> Vec<u8> {
        [
            randombytes(129).as_slice(),
            &(0 as usize).to_le_bytes(),
            randombytes(128).as_slice(),
            &(32 as usize).to_le_bytes(),
            randombytes(32).as_slice(),
            &(112 * sum_dict_len as usize).to_le_bytes(),
            randombytes(112 * sum_dict_len).as_slice(),
        ]
        .concat()
    }

    #[test]
    fn test_updatemessagebuffer() {
        // new
        assert_eq!(UpdateMessageBuffer::new(10).bytes, vec![0_u8; 10]);

        // try from
        assert_eq!(
            UpdateMessageBuffer::try_from(vec![0_u8; 10]).unwrap_err(),
            PetError::InvalidMessage,
        );
        let sum_dict_len = 1 + randombytes_uniform(10) as usize;
        let mut bytes = auxiliary_bytes(sum_dict_len);
        let mut buffer = UpdateMessageBuffer::try_from(bytes.clone()).unwrap();
        assert_eq!(buffer.bytes, bytes);

        // update signature
        let range = buffer.update_signature_range();
        assert_eq!(buffer.update_signature(), &bytes[range.clone()]);
        assert_eq!(buffer.update_signature_mut(), &mut bytes[range]);

        // masked model length
        let range = buffer.masked_model_len_range();
        assert_eq!(buffer.masked_model_len(), &bytes[range.clone()]);
        assert_eq!(buffer.masked_model_len_mut(), &mut bytes[range]);
        assert_eq!(buffer.masked_model_bytes(), 32);

        // masked model
        let range = buffer.masked_model_range();
        assert_eq!(buffer.masked_model(), &bytes[range.clone()]);
        assert_eq!(buffer.masked_model_mut(), &mut bytes[range]);

        // local seed dictionary length
        let range = buffer.local_seed_dict_len_range();
        assert_eq!(buffer.local_seed_dict_len(), &bytes[range.clone()]);
        assert_eq!(buffer.local_seed_dict_len_mut(), &mut bytes[range]);
        assert_eq!(buffer.local_seed_dict_bytes(), 112 * sum_dict_len);

        // local seed dictionary
        let range = buffer.local_seed_dict_range();
        assert_eq!(buffer.local_seed_dict(), &bytes[range.clone()]);
        assert_eq!(buffer.local_seed_dict_mut(), &mut bytes[range]);
    }

    #[test]
    fn test_updatemessage_serialize() {
        // from parts
        let sum_dict_len = 1 + randombytes_uniform(10) as usize;
        let pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let certificate = &Certificate::new();
        let sum_signature = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let update_signature = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let masked_model = &randombytes(32).into();
        let local_seed_dict = &iter::repeat_with(|| {
            (
                sign::PublicKey::from_slice(&randombytes(32)).unwrap(),
                EncrMaskSeed::try_from(randombytes(80)).unwrap(),
            )
        })
        .take(sum_dict_len)
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
            msg.masked_model as *const MaskedModel,
            masked_model as *const MaskedModel
        );
        assert_eq!(
            msg.local_seed_dict as *const LocalSeedDict,
            local_seed_dict as *const LocalSeedDict,
        );
        assert_eq!(msg.len(), 289 + 3 * LEN_BYTES + 112 * sum_dict_len);

        // serialize seed dictionary
        let local_seed_vec = msg.serialize_local_seed_dict();
        assert_eq!(
            local_seed_vec.len(),
            (PK_BYTES + EncrMaskSeed::BYTES) * sum_dict_len
        );
        assert!(local_seed_vec
            .chunks_exact(PK_BYTES + EncrMaskSeed::BYTES)
            .all(|chunk| {
                local_seed_dict
                    .get(&sign::PublicKey::from_slice(&chunk[..PK_BYTES]).unwrap())
                    .unwrap()
                    .as_ref()
                    == &chunk[PK_BYTES..]
            }));

        // serialize
        let mut buffer = UpdateMessageBuffer::new(289 + 3 * LEN_BYTES + 112 * sum_dict_len);
        let coord_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        msg.serialize(&mut buffer, &coord_pk);
        assert_eq!(buffer.tag(), &[UPDATE_TAG]);
        assert_eq!(buffer.coord_pk(), coord_pk.as_ref());
        assert_eq!(buffer.part_pk(), pk.as_ref());
        assert_eq!(buffer.certificate_len(), &(0 as usize).to_le_bytes());
        assert_eq!(buffer.certificate(), certificate.as_ref());
        assert_eq!(buffer.sum_signature(), sum_signature.as_ref());
        assert_eq!(buffer.update_signature(), update_signature.as_ref());
        assert_eq!(buffer.masked_model_len(), &(32 as usize).to_le_bytes());
        assert_eq!(buffer.masked_model(), masked_model.as_ref());
        assert_eq!(
            buffer.local_seed_dict_len(),
            &(112 * sum_dict_len as usize).to_le_bytes(),
        );
        assert_eq!(buffer.local_seed_dict(), local_seed_vec.as_slice());
    }

    #[test]
    fn test_updatemessage_deserialize() {
        // deserialize seed dictionary
        let sum_dict_len = 1 + randombytes_uniform(10) as usize;
        let local_seed_vec = randombytes((PK_BYTES + EncrMaskSeed::BYTES) * sum_dict_len);
        let local_seed_dict = UpdateMessage::deserialize_local_seed_dict(&local_seed_vec);
        for chunk in local_seed_vec.chunks_exact(PK_BYTES + EncrMaskSeed::BYTES) {
            assert_eq!(
                local_seed_dict
                    .get(&sign::PublicKey::from_slice(&chunk[..PK_BYTES]).unwrap())
                    .unwrap(),
                &EncrMaskSeed::try_from(&chunk[PK_BYTES..]).unwrap(),
            );
        }

        // deserialize
        let bytes = auxiliary_bytes(sum_dict_len);
        let buffer = UpdateMessageBuffer::try_from(bytes.clone()).unwrap();
        let msg = UpdateMessage::deserialize(buffer.clone());
        assert_eq!(
            msg.pk(),
            &sign::PublicKey::from_slice(&bytes[buffer.part_pk_range()]).unwrap(),
        );
        assert_eq!(msg.certificate(), &bytes[buffer.certificate_range()].into());
        assert_eq!(
            msg.sum_signature(),
            &sign::Signature::from_slice(&bytes[buffer.sum_signature_range()]).unwrap(),
        );
        assert_eq!(
            msg.update_signature(),
            &sign::Signature::from_slice(&bytes[buffer.update_signature_range()]).unwrap(),
        );
        assert_eq!(
            msg.masked_model(),
            &bytes[buffer.masked_model_range()].into(),
        );
        assert_eq!(
            msg.local_seed_dict(),
            &UpdateMessage::deserialize_local_seed_dict(&bytes[buffer.local_seed_dict_range()]),
        );
    }

    #[test]
    fn test_updatemessage() {
        // seal
        let sum_dict_len = 1 + randombytes_uniform(10) as usize;
        let (pk, sk) = sign::gen_keypair();
        let certificate = Certificate::new();
        let sum_signature = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let update_signature = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let masked_model = randombytes(32).into();
        let local_seed_dict = iter::repeat_with(|| {
            (
                sign::PublicKey::from_slice(&randombytes(32)).unwrap(),
                EncrMaskSeed::try_from(randombytes(80)).unwrap(),
            )
        })
        .take(sum_dict_len)
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
        let msg = UpdateMessage::open(&bytes, &coord_pk, &coord_sk).unwrap();
        assert_eq!(msg.pk(), &pk);
        assert_eq!(msg.certificate(), &certificate);
        assert_eq!(msg.sum_signature(), &sum_signature);
        assert_eq!(msg.update_signature(), &update_signature);
        assert_eq!(msg.masked_model(), &masked_model);
        assert_eq!(msg.local_seed_dict(), &local_seed_dict);

        // wrong signature
        let bytes = auxiliary_bytes(sum_dict_len);
        let mut buffer = UpdateMessageBuffer::try_from(bytes).unwrap();
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
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong receiver
        msg.serialize(
            &mut buffer,
            &box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
        );
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong tag
        buffer.tag_mut().copy_from_slice(&[0_u8]);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong length
        let buffer = UpdateMessageBuffer::new(10);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );
    }
}
