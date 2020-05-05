use std::{
    borrow::Borrow,
    convert::{TryFrom, TryInto},
    ops::Range,
};

use super::{MessageBuffer, Tag, LEN_BYTES, PK_BYTES, SIGNATURE_BYTES};
use crate::{
    certificate::Certificate,
    crypto::{ByteObject, Signature},
    mask::{seed::EncryptedMaskSeed, Integers, MaskedModel},
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    LocalSeedDict,
    ParticipantTaskSignature,
    PetError,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
    UpdateParticipantSecretKey,
};

#[derive(Clone, Debug)]
/// Access to update message buffer fields.
struct UpdateMessageBuffer<B> {
    bytes: B,
    certificate_range: Range<usize>,
    masked_model_range: Range<usize>,
    local_seed_dict_range: Range<usize>,
}

impl UpdateMessageBuffer<Vec<u8>> {
    /// Create an empty update message buffer.
    fn new(certificate_len: usize, masked_model_len: usize, local_seed_dict_len: usize) -> Self {
        let bytes = [
            vec![0_u8; Self::UPDATE_SIGNATURE_RANGE.end],
            certificate_len.to_le_bytes().to_vec(),
            masked_model_len.to_le_bytes().to_vec(),
            local_seed_dict_len.to_le_bytes().to_vec(),
            vec![0_u8; certificate_len + masked_model_len + local_seed_dict_len],
        ]
        .concat();
        let certificate_range = Self::LOCAL_SEED_DICT_LEN_RANGE.end
            ..Self::LOCAL_SEED_DICT_LEN_RANGE.end + certificate_len;
        let masked_model_range = certificate_range.end..certificate_range.end + masked_model_len;
        let local_seed_dict_range =
            masked_model_range.end..masked_model_range.end + local_seed_dict_len;
        Self {
            bytes,
            certificate_range,
            masked_model_range,
            local_seed_dict_range,
        }
    }
}

impl TryFrom<Vec<u8>> for UpdateMessageBuffer<Vec<u8>> {
    type Error = PetError;

    /// Create an update message buffer from `bytes`. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let mut buffer = Self {
            bytes,
            certificate_range: 0..0,
            masked_model_range: 0..0,
            local_seed_dict_range: 0..0,
        };
        if buffer.len() >= Self::LOCAL_SEED_DICT_LEN_RANGE.end {
            // safe unwraps: lengths of slices are guaranteed by constants
            buffer.certificate_range = Self::LOCAL_SEED_DICT_LEN_RANGE.end
                ..Self::LOCAL_SEED_DICT_LEN_RANGE.end
                    + usize::from_le_bytes(buffer.certificate_len().try_into().unwrap());
            buffer.masked_model_range = buffer.certificate_range.end
                ..buffer.certificate_range.end
                    + usize::from_le_bytes(buffer.masked_model_len().try_into().unwrap());
            buffer.local_seed_dict_range = buffer.masked_model_range.end
                ..buffer.masked_model_range.end
                    + usize::from_le_bytes(buffer.local_seed_dict_len().try_into().unwrap());
        } else {
            return Err(PetError::InvalidMessage);
        }
        if buffer.len() == buffer.local_seed_dict_range.end {
            Ok(buffer)
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> MessageBuffer for UpdateMessageBuffer<B> {
    /// Get a reference to the message buffer.
    fn bytes(&'_ self) -> &'_ [u8] {
        self.bytes.as_ref()
    }

    /// Get a mutable reference to the message buffer.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> UpdateMessageBuffer<B> {
    /// Get the range of the update signature field.
    const UPDATE_SIGNATURE_RANGE: Range<usize> =
        Self::SUM_SIGNATURE_RANGE.end..Self::SUM_SIGNATURE_RANGE.end + SIGNATURE_BYTES;

    /// Get the range of the certificate length field.
    const CERTIFICATE_LEN_RANGE: Range<usize> =
        Self::UPDATE_SIGNATURE_RANGE.end..Self::UPDATE_SIGNATURE_RANGE.end + LEN_BYTES;

    /// Get the range of the masked model length field.
    const MASKED_MODEL_LEN_RANGE: Range<usize> =
        Self::CERTIFICATE_LEN_RANGE.end..Self::CERTIFICATE_LEN_RANGE.end + LEN_BYTES;

    /// Get the range of the local seed dictionary length field.
    const LOCAL_SEED_DICT_LEN_RANGE: Range<usize> =
        Self::MASKED_MODEL_LEN_RANGE.end..Self::MASKED_MODEL_LEN_RANGE.end + LEN_BYTES;

    /// Get a reference to the update signature field.
    fn update_signature(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::UPDATE_SIGNATURE_RANGE]
    }

    /// Get a mutable reference to the update signature field.
    fn update_signature_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[Self::UPDATE_SIGNATURE_RANGE]
    }

    /// Get a reference to the certificate length field.
    fn certificate_len(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::CERTIFICATE_LEN_RANGE]
    }

    /// Get a reference to the certificate field.
    fn certificate(&'_ self) -> &'_ [u8] {
        &self.bytes()[self.certificate_range.clone()]
    }

    /// Get a mutable reference to the certificate field.
    fn certificate_mut(&mut self) -> &mut [u8] {
        let range = self.certificate_range.clone();
        &mut self.bytes_mut()[range]
    }

    /// Get a reference to the masked model length field.
    fn masked_model_len(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::MASKED_MODEL_LEN_RANGE]
    }

    /// Get a reference to the masked model field.
    fn masked_model(&'_ self) -> &'_ [u8] {
        &self.bytes()[self.masked_model_range.clone()]
    }

    /// Get a mutable reference to the masked model field.
    fn masked_model_mut(&mut self) -> &mut [u8] {
        let range = self.masked_model_range.clone();
        &mut self.bytes_mut()[range]
    }

    /// Get a reference to the local seed dictionary length field.
    fn local_seed_dict_len(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::LOCAL_SEED_DICT_LEN_RANGE]
    }

    /// Get a reference to the local seed dictionary field.
    fn local_seed_dict(&'_ self) -> &'_ [u8] {
        &self.bytes()[self.local_seed_dict_range.clone()]
    }

    /// Get a mutable reference to the local seed dictionary field.
    fn local_seed_dict_mut(&mut self) -> &mut [u8] {
        let range = self.local_seed_dict_range.clone();
        &mut self.bytes_mut()[range]
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Encryption and decryption of update messages.
pub struct UpdateMessage<K, S, C, M, D>
where
    K: Borrow<UpdateParticipantPublicKey>,
    S: Borrow<ParticipantTaskSignature>,
    C: Borrow<Certificate>,
    M: Borrow<MaskedModel>,
    D: Borrow<LocalSeedDict>,
{
    pk: K,
    sum_signature: S,
    update_signature: S,
    certificate: C,
    masked_model: M,
    local_seed_dict: D,
}

impl<K, S, C, M, D> UpdateMessage<K, S, C, M, D>
where
    K: Borrow<UpdateParticipantPublicKey>,
    S: Borrow<ParticipantTaskSignature>,
    C: Borrow<Certificate>,
    M: Borrow<MaskedModel>,
    D: Borrow<LocalSeedDict>,
{
    /// Create an update message from its parts.
    pub fn from_parts(
        pk: K,
        sum_signature: S,
        update_signature: S,
        certificate: C,
        masked_model: M,
        local_seed_dict: D,
    ) -> Self {
        Self {
            pk,
            sum_signature,
            update_signature,
            certificate,
            masked_model,
            local_seed_dict,
        }
    }

    /// Serialize the local seed dictionary into bytes.
    fn serialize_local_seed_dict(&self) -> Vec<u8> {
        self.local_seed_dict
            .borrow()
            .iter()
            .flat_map(|(pk, seed)| [pk.as_slice(), seed.as_ref()].concat())
            .collect::<Vec<u8>>()
    }

    /// Serialize the update message into a buffer.
    fn serialize<B: AsRef<[u8]> + AsMut<[u8]>>(
        &self,
        buffer: &mut UpdateMessageBuffer<B>,
        pk: &CoordinatorPublicKey,
    ) {
        buffer
            .tag_mut()
            .copy_from_slice([Tag::Update as u8].as_ref());
        buffer
            .coord_pk_mut()
            .copy_from_slice(pk.borrow().as_slice());
        buffer
            .part_pk_mut()
            .copy_from_slice(self.pk.borrow().as_slice());
        buffer
            .sum_signature_mut()
            .copy_from_slice(self.sum_signature.borrow().as_slice());
        buffer
            .update_signature_mut()
            .copy_from_slice(self.update_signature.borrow().as_slice());
        buffer
            .certificate_mut()
            .copy_from_slice(self.certificate.borrow().as_ref());
        buffer
            .masked_model_mut()
            .copy_from_slice(self.masked_model.borrow().serialize().as_slice());
        buffer
            .local_seed_dict_mut()
            .copy_from_slice(self.serialize_local_seed_dict().as_slice());
    }

    /// Sign and encrypt the update message.
    pub fn seal(&self, sk: &UpdateParticipantSecretKey, pk: &CoordinatorPublicKey) -> Vec<u8> {
        let mut buffer = UpdateMessageBuffer::new(
            self.certificate.borrow().len(),
            self.masked_model.borrow().len(),
            self.local_seed_dict.borrow().len() * (PK_BYTES + EncryptedMaskSeed::BYTES),
        );
        self.serialize(&mut buffer, pk);
        let signature = sk.sign_detached(buffer.message());
        buffer.signature_mut().copy_from_slice(signature.as_slice());
        pk.encrypt(buffer.bytes())
    }
}

impl
    UpdateMessage<
        UpdateParticipantPublicKey,
        ParticipantTaskSignature,
        Certificate,
        MaskedModel,
        LocalSeedDict,
    >
{
    /// Deserialize a local seed dictionary from bytes. Fails if the length of the input is invalid.
    fn deserialize_local_seed_dict(bytes: &[u8]) -> Result<LocalSeedDict, PetError> {
        if bytes.len() % (PK_BYTES + EncryptedMaskSeed::BYTES) == 0 {
            let local_seed_dict = bytes
                .chunks_exact(PK_BYTES + EncryptedMaskSeed::BYTES)
                .map(|chunk| {
                    if let (Some(pk), Some(seed)) = (
                        SumParticipantPublicKey::from_slice(&chunk[..PK_BYTES]),
                        EncryptedMaskSeed::from_slice(&chunk[PK_BYTES..]),
                    ) {
                        Ok((pk, seed))
                    } else {
                        Err(PetError::InvalidMessage)
                    }
                })
                .collect::<Result<LocalSeedDict, PetError>>()?;
            Ok(local_seed_dict)
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Deserialize an update message from a buffer. Fails if the length of a part is invalid.
    fn deserialize(buffer: UpdateMessageBuffer<Vec<u8>>) -> Result<Self, PetError> {
        let pk = UpdateParticipantPublicKey::from_slice(buffer.part_pk())
            .ok_or(PetError::InvalidMessage)?;
        let sum_signature =
            Signature::from_slice(buffer.sum_signature()).ok_or(PetError::InvalidMessage)?;
        let update_signature =
            Signature::from_slice(buffer.update_signature()).ok_or(PetError::InvalidMessage)?;
        let certificate = Certificate::deserialize(buffer.certificate())?;
        let masked_model = MaskedModel::deserialize(buffer.masked_model())?;
        let local_seed_dict = Self::deserialize_local_seed_dict(buffer.local_seed_dict())?;
        Ok(Self {
            pk,
            sum_signature,
            update_signature,
            certificate,
            masked_model,
            local_seed_dict,
        })
    }

    /// Decrypt and verify an update message. Fails if decryption or validation fails.
    pub fn open(
        bytes: &[u8],
        pk: &CoordinatorPublicKey,
        sk: &CoordinatorSecretKey,
    ) -> Result<Self, PetError> {
        let buffer = UpdateMessageBuffer::try_from(
            sk.decrypt(bytes, pk).or(Err(PetError::InvalidMessage))?,
        )?;
        if buffer.tag() == [Tag::Update as u8]
            && buffer.coord_pk() == pk.as_slice()
            && UpdateParticipantPublicKey::from_slice(buffer.part_pk())
                .ok_or(PetError::InvalidMessage)?
                .verify_detached(
                    &Signature::from_slice(buffer.signature()).ok_or(PetError::InvalidMessage)?,
                    buffer.message(),
                )
        {
            Ok(Self::deserialize(buffer)?)
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    derive_struct_fields!(
        pk, SumParticipantPublicKey;
        sum_signature, ParticipantTaskSignature;
        update_signature, ParticipantTaskSignature;
        certificate, Certificate;
        masked_model, MaskedModel;
        local_seed_dict, LocalSeedDict;
    );
}

#[cfg(test)]
mod tests {
    use std::iter;

    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use sodiumoxide::randombytes::{randombytes, randombytes_uniform};

    use super::*;
    use crate::{
        crypto::{generate_encrypt_key_pair, generate_integer, generate_signing_key_pair},
        mask::{
            config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
            MaskedModel,
        },
        message::TAG_BYTES,
    };

    type MB = UpdateMessageBuffer<Vec<u8>>;

    fn auxiliary_bytes(sum_dict_len: usize) -> Vec<u8> {
        let masked_model = auxiliary_masked_model();
        [
            randombytes(257),
            (32 as usize).to_le_bytes().to_vec(),
            masked_model.len().to_le_bytes().to_vec(),
            (112 * sum_dict_len as usize).to_le_bytes().to_vec(),
            randombytes(32),
            masked_model.serialize(),
            randombytes(112 * sum_dict_len),
        ]
        .concat()
    }

    fn auxiliary_masked_model() -> MaskedModel {
        let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        let integers = iter::repeat_with(|| generate_integer(&mut prng, config.order()))
            .take(10)
            .collect();
        MaskedModel::from_parts(integers, config).unwrap()
    }

    #[test]
    fn test_updatemessagebuffer_ranges() {
        assert_eq!(MB::SIGNATURE_RANGE, ..SIGNATURE_BYTES);
        assert_eq!(MB::MESSAGE_RANGE, SIGNATURE_BYTES..);
        assert_eq!(MB::TAG_RANGE, 64..64 + TAG_BYTES);
        assert_eq!(MB::COORD_PK_RANGE, 65..65 + PK_BYTES);
        assert_eq!(MB::PART_PK_RANGE, 97..97 + PK_BYTES);
        assert_eq!(MB::SUM_SIGNATURE_RANGE, 129..129 + SIGNATURE_BYTES);
        assert_eq!(MB::UPDATE_SIGNATURE_RANGE, 193..193 + SIGNATURE_BYTES);
        assert_eq!(MB::CERTIFICATE_LEN_RANGE, 257..257 + LEN_BYTES);
        assert_eq!(
            MB::MASKED_MODEL_LEN_RANGE,
            257 + LEN_BYTES..257 + 2 * LEN_BYTES,
        );
        assert_eq!(
            MB::LOCAL_SEED_DICT_LEN_RANGE,
            257 + 2 * LEN_BYTES..257 + 3 * LEN_BYTES,
        );
        let sum_dict_len = 1 + randombytes_uniform(10) as usize;
        let buffer = UpdateMessageBuffer::new(32, 32, 112 * sum_dict_len);
        assert_eq!(
            buffer.certificate_range,
            257 + 3 * LEN_BYTES..257 + 3 * LEN_BYTES + 32,
        );
        assert_eq!(
            buffer.masked_model_range,
            257 + 3 * LEN_BYTES + 32..257 + 3 * LEN_BYTES + 32 + 32,
        );
        assert_eq!(
            buffer.local_seed_dict_range,
            257 + 3 * LEN_BYTES + 32 + 32..257 + 3 * LEN_BYTES + 32 + 32 + 112 * sum_dict_len,
        );
    }

    #[test]
    fn test_updatemessagebuffer_fields() {
        // new
        let sum_dict_len = 1 + randombytes_uniform(10) as usize;
        assert_eq!(
            UpdateMessageBuffer::new(32, 32, 112 * sum_dict_len).bytes,
            [
                vec![0_u8; 257],
                (32 as usize).to_le_bytes().to_vec(),
                (32 as usize).to_le_bytes().to_vec(),
                (112 * sum_dict_len as usize).to_le_bytes().to_vec(),
                vec![0_u8; 64 + 112 * sum_dict_len],
            ]
            .concat(),
        );

        // try from
        let mut bytes = auxiliary_bytes(sum_dict_len);
        let mut buffer = UpdateMessageBuffer::try_from(bytes.clone()).unwrap();
        assert_eq!(buffer.bytes, bytes);
        assert_eq!(
            UpdateMessageBuffer::try_from(vec![0_u8; 0]).unwrap_err(),
            PetError::InvalidMessage,
        );

        // length
        assert_eq!(buffer.len(), 353 + 112 * sum_dict_len + 3 * LEN_BYTES);

        // signature
        assert_eq!(buffer.signature(), &bytes[MB::SIGNATURE_RANGE]);
        assert_eq!(buffer.signature_mut(), &mut bytes[MB::SIGNATURE_RANGE]);

        // message
        assert_eq!(buffer.message(), &bytes[MB::MESSAGE_RANGE]);

        // tag
        assert_eq!(buffer.tag(), &bytes[MB::TAG_RANGE]);
        assert_eq!(buffer.tag_mut(), &mut bytes[MB::TAG_RANGE]);

        // coordinator pk
        assert_eq!(buffer.coord_pk(), &bytes[MB::COORD_PK_RANGE]);
        assert_eq!(buffer.coord_pk_mut(), &mut bytes[MB::COORD_PK_RANGE]);

        // participant pk
        assert_eq!(buffer.part_pk(), &bytes[MB::PART_PK_RANGE]);
        assert_eq!(buffer.part_pk_mut(), &mut bytes[MB::PART_PK_RANGE]);

        // sum signature
        assert_eq!(buffer.sum_signature(), &bytes[MB::SUM_SIGNATURE_RANGE]);
        assert_eq!(
            buffer.sum_signature_mut(),
            &mut bytes[MB::SUM_SIGNATURE_RANGE],
        );

        // update signature
        assert_eq!(
            buffer.update_signature(),
            &bytes[MB::UPDATE_SIGNATURE_RANGE],
        );
        assert_eq!(
            buffer.update_signature_mut(),
            &mut bytes[MB::UPDATE_SIGNATURE_RANGE],
        );

        // certificate
        assert_eq!(buffer.certificate_len(), &bytes[MB::CERTIFICATE_LEN_RANGE]);
        let range = buffer.certificate_range.clone();
        assert_eq!(buffer.certificate(), &bytes[range.clone()]);
        assert_eq!(buffer.certificate_mut(), &mut bytes[range]);

        // masked model
        assert_eq!(
            buffer.masked_model_len(),
            &bytes[MB::MASKED_MODEL_LEN_RANGE],
        );
        let range = buffer.masked_model_range.clone();
        assert_eq!(buffer.masked_model(), &bytes[range.clone()]);
        assert_eq!(buffer.masked_model_mut(), &mut bytes[range]);

        // local seed dictionary
        assert_eq!(
            buffer.local_seed_dict_len(),
            &bytes[MB::LOCAL_SEED_DICT_LEN_RANGE],
        );
        let range = buffer.local_seed_dict_range.clone();
        assert_eq!(buffer.local_seed_dict(), &bytes[range.clone()]);
        assert_eq!(buffer.local_seed_dict_mut(), &mut bytes[range]);
    }

    #[test]
    fn test_updatemessage_serialize() {
        // from parts
        let sum_dict_len = 1 + randombytes_uniform(10) as usize;
        let pk = &UpdateParticipantPublicKey::from_slice_unchecked(randombytes(32).as_slice());
        let sum_signature = &Signature::from_slice_unchecked(randombytes(64).as_slice());
        let update_signature = &Signature::from_slice_unchecked(randombytes(64).as_slice());
        let certificate = &Certificate::zeroed();
        let masked_model = &auxiliary_masked_model();
        let local_seed_dict = &iter::repeat_with(|| {
            (
                SumParticipantPublicKey::from_slice_unchecked(randombytes(32).as_slice()),
                EncryptedMaskSeed::from_slice_unchecked(randombytes(80).as_slice()),
            )
        })
        .take(sum_dict_len)
        .collect();
        let msg = UpdateMessage::from_parts(
            pk,
            sum_signature,
            update_signature,
            certificate,
            masked_model,
            local_seed_dict,
        );
        assert_eq!(
            msg.pk as *const UpdateParticipantPublicKey,
            pk as *const UpdateParticipantPublicKey,
        );
        assert_eq!(
            msg.sum_signature as *const Signature,
            sum_signature as *const Signature,
        );
        assert_eq!(
            msg.update_signature as *const Signature,
            update_signature as *const Signature,
        );
        assert_eq!(
            msg.certificate as *const Certificate,
            certificate as *const Certificate,
        );
        assert_eq!(
            msg.masked_model as *const MaskedModel,
            masked_model as *const MaskedModel
        );
        assert_eq!(
            msg.local_seed_dict as *const LocalSeedDict,
            local_seed_dict as *const LocalSeedDict,
        );

        // serialize seed dictionary
        let local_seed_vec = msg.serialize_local_seed_dict();
        assert_eq!(
            local_seed_vec.len(),
            (PK_BYTES + EncryptedMaskSeed::BYTES) * sum_dict_len
        );
        assert!(local_seed_vec
            .chunks_exact(PK_BYTES + EncryptedMaskSeed::BYTES)
            .all(|chunk| {
                local_seed_dict
                    .get(&SumParticipantPublicKey::from_slice_unchecked(
                        &chunk[..PK_BYTES],
                    ))
                    .unwrap()
                    .as_slice()
                    == &chunk[PK_BYTES..]
            }));

        // serialize
        let mut buffer = UpdateMessageBuffer::new(32, masked_model.len(), 112 * sum_dict_len);
        let coord_pk = CoordinatorPublicKey::from_slice_unchecked(randombytes(32).as_slice());
        msg.serialize(&mut buffer, &coord_pk);
        assert_eq!(buffer.tag(), [Tag::Update as u8].as_ref());
        assert_eq!(buffer.coord_pk(), coord_pk.as_slice());
        assert_eq!(buffer.part_pk(), pk.as_slice());
        assert_eq!(buffer.sum_signature(), sum_signature.as_slice());
        assert_eq!(buffer.update_signature(), update_signature.as_slice());
        assert_eq!(
            buffer.certificate_len(),
            certificate.len().to_le_bytes().as_ref(),
        );
        assert_eq!(buffer.certificate(), certificate.as_slice());
        assert_eq!(
            buffer.masked_model_len(),
            masked_model.len().to_le_bytes().as_ref(),
        );
        assert_eq!(buffer.masked_model(), masked_model.serialize().as_slice());
        assert_eq!(
            buffer.local_seed_dict_len(),
            (112 * sum_dict_len as usize).to_le_bytes().as_ref(),
        );
        assert_eq!(buffer.local_seed_dict(), local_seed_vec.as_slice());
    }

    #[test]
    fn test_updatemessage_deserialize() {
        // deserialize seed dictionary
        let sum_dict_len = 1 + randombytes_uniform(10) as usize;
        let local_seed_vec = randombytes((PK_BYTES + EncryptedMaskSeed::BYTES) * sum_dict_len);
        let local_seed_dict = UpdateMessage::deserialize_local_seed_dict(&local_seed_vec).unwrap();
        for chunk in local_seed_vec.chunks_exact(PK_BYTES + EncryptedMaskSeed::BYTES) {
            assert_eq!(
                local_seed_dict
                    .get(&SumParticipantPublicKey::from_slice_unchecked(
                        &chunk[..PK_BYTES]
                    ))
                    .unwrap(),
                &EncryptedMaskSeed::from_slice_unchecked(&chunk[PK_BYTES..]),
            );
        }

        // deserialize
        let bytes = auxiliary_bytes(sum_dict_len);
        let buffer = UpdateMessageBuffer::try_from(bytes.clone()).unwrap();
        let msg = UpdateMessage::deserialize(buffer.clone()).unwrap();
        assert_eq!(
            msg.pk(),
            &UpdateParticipantPublicKey::from_slice_unchecked(&bytes[MB::PART_PK_RANGE]),
        );
        assert_eq!(
            msg.sum_signature(),
            &Signature::from_slice_unchecked(&bytes[MB::SUM_SIGNATURE_RANGE]),
        );
        assert_eq!(
            msg.update_signature(),
            &Signature::from_slice_unchecked(&bytes[MB::UPDATE_SIGNATURE_RANGE]),
        );
        assert_eq!(
            msg.certificate(),
            &Certificate::deserialize(&bytes[buffer.certificate_range.clone()]).unwrap()
        );
        assert_eq!(
            msg.masked_model(),
            &MaskedModel::deserialize(&bytes[buffer.masked_model_range.clone()]).unwrap(),
        );
        assert_eq!(
            msg.local_seed_dict(),
            &UpdateMessage::deserialize_local_seed_dict(
                &bytes[buffer.local_seed_dict_range.clone()]
            )
            .unwrap(),
        );
    }

    #[test]
    fn test_updatemessage() {
        // seal
        let sum_dict_len = 1 + randombytes_uniform(10) as usize;
        let (pk, sk) = generate_signing_key_pair();
        let sum_signature = Signature::from_slice_unchecked(randombytes(64).as_slice());
        let update_signature = Signature::from_slice_unchecked(randombytes(64).as_slice());
        let certificate = Certificate::zeroed();
        let masked_model = auxiliary_masked_model();
        let local_seed_dict = iter::repeat_with(|| {
            (
                SumParticipantPublicKey::from_slice_unchecked(randombytes(32).as_slice()),
                EncryptedMaskSeed::from_slice_unchecked(randombytes(80).as_slice()),
            )
        })
        .take(sum_dict_len)
        .collect();
        let (coord_pk, coord_sk) = generate_encrypt_key_pair();
        let bytes = UpdateMessage::from_parts(
            &pk,
            &sum_signature,
            &update_signature,
            &certificate,
            &masked_model,
            &local_seed_dict,
        )
        .seal(&sk, &coord_pk);

        // open
        let msg = UpdateMessage::open(&bytes, &coord_pk, &coord_sk).unwrap();
        assert_eq!(msg.pk(), &pk);
        assert_eq!(msg.sum_signature(), &sum_signature);
        assert_eq!(msg.update_signature(), &update_signature);
        assert_eq!(msg.certificate(), &certificate);
        assert_eq!(msg.masked_model(), &masked_model);
        assert_eq!(msg.local_seed_dict(), &local_seed_dict);

        // wrong signature
        let bytes = auxiliary_bytes(sum_dict_len);
        let mut buffer = UpdateMessageBuffer::try_from(bytes).unwrap();
        let msg = UpdateMessage::from_parts(
            &pk,
            &sum_signature,
            &update_signature,
            &certificate,
            &masked_model,
            &local_seed_dict,
        );
        msg.serialize(&mut buffer, &coord_pk);
        let bytes = coord_pk.encrypt(buffer.bytes());
        assert_eq!(
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong receiver
        msg.serialize(
            &mut buffer,
            &CoordinatorPublicKey::from_slice_unchecked(randombytes(32).as_slice()),
        );
        let bytes = coord_pk.encrypt(buffer.bytes());
        assert_eq!(
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong tag
        buffer.tag_mut().copy_from_slice([Tag::None as u8].as_ref());
        let bytes = coord_pk.encrypt(buffer.bytes());
        assert_eq!(
            UpdateMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong length
        assert_eq!(
            UpdateMessage::open([0_u8; 0].as_ref(), &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );
    }
}
