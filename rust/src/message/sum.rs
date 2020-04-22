use std::{
    borrow::Borrow,
    convert::{TryFrom, TryInto},
    ops::Range,
};

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{MessageBuffer, Tag, LEN_BYTES, PK_BYTES};
use crate::{
    certificate::Certificate,
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    ParticipantTaskSignature,
    PetError,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    SumParticipantSecretKey,
};

#[derive(Clone, Debug)]
/// Access to sum message buffer fields.
struct SumMessageBuffer<B> {
    bytes: B,
    certificate_range: Range<usize>,
}

impl SumMessageBuffer<Vec<u8>> {
    /// Create an empty sum message buffer.
    fn new(certificate_len: usize) -> Self {
        let bytes = [
            vec![0_u8; Self::EPHM_PK_RANGE.end],
            certificate_len.to_le_bytes().to_vec(),
            vec![0_u8; certificate_len],
        ]
        .concat();
        let certificate_range =
            Self::CERTIFICATE_LEN_RANGE.end..Self::CERTIFICATE_LEN_RANGE.end + certificate_len;
        Self {
            bytes,
            certificate_range,
        }
    }
}

impl TryFrom<Vec<u8>> for SumMessageBuffer<Vec<u8>> {
    type Error = PetError;

    /// Create a sum message buffer from `bytes`. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let mut buffer = Self {
            bytes,
            certificate_range: 0..0,
        };
        if buffer.len() >= Self::CERTIFICATE_LEN_RANGE.end {
            // safe unwrap: length of slice is guaranteed by constants
            buffer.certificate_range = Self::CERTIFICATE_LEN_RANGE.end
                ..Self::CERTIFICATE_LEN_RANGE.end
                    + usize::from_le_bytes(buffer.certificate_len().try_into().unwrap());
        } else {
            return Err(PetError::InvalidMessage);
        }
        if buffer.len() == buffer.certificate_range.end {
            Ok(buffer)
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> MessageBuffer for SumMessageBuffer<B> {
    /// Get a reference to the message buffer.
    fn bytes(&'_ self) -> &'_ [u8] {
        self.bytes.as_ref()
    }

    /// Get a mutable reference to the message buffer.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> SumMessageBuffer<B> {
    /// Get the range of the public ephemeral key field.
    const EPHM_PK_RANGE: Range<usize> =
        Self::SUM_SIGNATURE_RANGE.end..Self::SUM_SIGNATURE_RANGE.end + PK_BYTES;

    /// Get the range of the certificate length field.
    const CERTIFICATE_LEN_RANGE: Range<usize> =
        Self::EPHM_PK_RANGE.end..Self::EPHM_PK_RANGE.end + LEN_BYTES;

    /// Get a reference to the public ephemeral key field.
    fn ephm_pk(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::EPHM_PK_RANGE]
    }

    /// Get a mutable reference to the public ephemeral key field.
    fn ephm_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[Self::EPHM_PK_RANGE]
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
}

#[derive(Clone, Debug, PartialEq)]
/// Encryption and decryption of sum messages.
pub struct SumMessage<K, S, E, C>
where
    K: Borrow<SumParticipantPublicKey>,
    S: Borrow<ParticipantTaskSignature>,
    E: Borrow<SumParticipantEphemeralPublicKey>,
    C: Borrow<Certificate>,
{
    pk: K,
    sum_signature: S,
    ephm_pk: E,
    certificate: C,
}

impl<K, S, E, C> SumMessage<K, S, E, C>
where
    K: Borrow<SumParticipantPublicKey>,
    S: Borrow<ParticipantTaskSignature>,
    E: Borrow<SumParticipantEphemeralPublicKey>,
    C: Borrow<Certificate>,
{
    /// Create a sum message from its parts.
    pub fn from_parts(pk: K, sum_signature: S, ephm_pk: E, certificate: C) -> Self {
        Self {
            pk,
            sum_signature,
            ephm_pk,
            certificate,
        }
    }

    /// Serialize the sum message into a buffer.
    fn serialize<B: AsRef<[u8]> + AsMut<[u8]>>(
        &self,
        buffer: &mut SumMessageBuffer<B>,
        pk: &CoordinatorPublicKey,
    ) {
        buffer.tag_mut().copy_from_slice([Tag::Sum as u8].as_ref());
        buffer.coord_pk_mut().copy_from_slice(pk.as_ref());
        buffer
            .part_pk_mut()
            .copy_from_slice(self.pk.borrow().as_ref());
        buffer
            .sum_signature_mut()
            .copy_from_slice(self.sum_signature.borrow().as_ref());
        buffer
            .ephm_pk_mut()
            .copy_from_slice(self.ephm_pk.borrow().as_ref());
        buffer
            .certificate_mut()
            .copy_from_slice(self.certificate.borrow().as_ref());
    }

    /// Sign and encrypt the sum message.
    pub fn seal(&self, sk: &SumParticipantSecretKey, pk: &CoordinatorPublicKey) -> Vec<u8> {
        let mut buffer = SumMessageBuffer::new(self.certificate.borrow().len());
        self.serialize(&mut buffer, pk);
        let signature = sign::sign_detached(buffer.message(), sk);
        buffer.signature_mut().copy_from_slice(signature.as_ref());
        sealedbox::seal(buffer.bytes(), pk)
    }
}

impl
    SumMessage<
        SumParticipantPublicKey,
        ParticipantTaskSignature,
        SumParticipantEphemeralPublicKey,
        Certificate,
    >
{
    /// Deserialize a sum message from a buffer.
    fn deserialize(buffer: SumMessageBuffer<Vec<u8>>) -> Self {
        // safe unwraps: lengths of slices are guaranteed by constants
        let pk = sign::PublicKey::from_slice(buffer.part_pk()).unwrap();
        let sum_signature = sign::Signature::from_slice(buffer.sum_signature()).unwrap();
        let ephm_pk = box_::PublicKey::from_slice(buffer.ephm_pk()).unwrap();
        let certificate = buffer.certificate().into();
        Self {
            pk,
            sum_signature,
            ephm_pk,
            certificate,
        }
    }

    /// Decrypt and verify a sum message. Fails if decryption or validation fails.
    pub fn open(
        bytes: &[u8],
        pk: &CoordinatorPublicKey,
        sk: &CoordinatorSecretKey,
    ) -> Result<Self, PetError> {
        let buffer = SumMessageBuffer::try_from(
            sealedbox::open(bytes, pk, sk).or(Err(PetError::InvalidMessage))?,
        )?;
        if buffer.tag() == [Tag::Sum as u8]
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
    pub fn pk(&self) -> &SumParticipantPublicKey {
        &self.pk
    }

    /// Get a reference to the sum signature.
    pub fn sum_signature(&self) -> &ParticipantTaskSignature {
        &self.sum_signature
    }

    /// Get a reference to the ephemeral public encryption key.
    pub fn ephm_pk(&self) -> &SumParticipantEphemeralPublicKey {
        &self.ephm_pk
    }

    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }
}

#[cfg(test)]
mod tests {
    use sodiumoxide::randombytes::randombytes;

    use super::*;
    use crate::message::{SIGNATURE_BYTES, TAG_BYTES};

    fn auxiliary_bytes() -> Vec<u8> {
        [
            randombytes(225),
            (32 as usize).to_le_bytes().to_vec(),
            vec![0_u8; 32],
        ]
        .concat()
    }

    type MB = SumMessageBuffer<Vec<u8>>;

    #[test]
    fn test_summessagebuffer_ranges() {
        assert_eq!(MB::SIGNATURE_RANGE, ..SIGNATURE_BYTES);
        assert_eq!(MB::MESSAGE_RANGE, SIGNATURE_BYTES..);
        assert_eq!(MB::TAG_RANGE, 64..64 + TAG_BYTES);
        assert_eq!(MB::COORD_PK_RANGE, 65..65 + PK_BYTES);
        assert_eq!(MB::PART_PK_RANGE, 97..97 + PK_BYTES);
        assert_eq!(MB::SUM_SIGNATURE_RANGE, 129..129 + SIGNATURE_BYTES);
        assert_eq!(MB::EPHM_PK_RANGE, 193..193 + PK_BYTES);
        assert_eq!(MB::CERTIFICATE_LEN_RANGE, 225..225 + LEN_BYTES);
        assert_eq!(
            SumMessageBuffer::new(32).certificate_range,
            225 + LEN_BYTES..225 + LEN_BYTES + 32,
        );
    }

    #[test]
    fn test_summessagebuffer_fields() {
        // new
        assert_eq!(
            SumMessageBuffer::new(32).bytes,
            [
                vec![0_u8; 225],
                (32 as usize).to_le_bytes().to_vec(),
                vec![0_u8; 32],
            ]
            .concat(),
        );

        // try from
        let mut bytes = auxiliary_bytes();
        let mut buffer = SumMessageBuffer::try_from(bytes.clone()).unwrap();
        assert_eq!(buffer.bytes, bytes);
        assert_eq!(
            SumMessageBuffer::try_from(vec![0_u8; 0]).unwrap_err(),
            PetError::InvalidMessage,
        );

        // length
        assert_eq!(buffer.len(), 257 + LEN_BYTES);

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

        // ephm pk
        assert_eq!(buffer.ephm_pk(), &bytes[MB::EPHM_PK_RANGE]);
        assert_eq!(buffer.ephm_pk_mut(), &mut bytes[MB::EPHM_PK_RANGE]);

        // certificate
        assert_eq!(buffer.certificate_len(), &bytes[MB::CERTIFICATE_LEN_RANGE]);
        let range = buffer.certificate_range.clone();
        assert_eq!(buffer.certificate(), &bytes[range.clone()]);
        assert_eq!(buffer.certificate_mut(), &mut bytes[range]);
    }

    #[test]
    fn test_summessage_serialize() {
        // from parts
        let pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let sum_signature = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let ephm_pk = &box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let certificate = &Certificate::new();
        let msg = SumMessage::from_parts(pk, sum_signature, ephm_pk, certificate);
        assert_eq!(
            msg.pk as *const sign::PublicKey,
            pk as *const sign::PublicKey,
        );
        assert_eq!(
            msg.sum_signature as *const sign::Signature,
            sum_signature as *const sign::Signature,
        );
        assert_eq!(
            msg.ephm_pk as *const box_::PublicKey,
            ephm_pk as *const box_::PublicKey,
        );
        assert_eq!(
            msg.certificate as *const Certificate,
            certificate as *const Certificate,
        );

        // serialize
        let mut buffer = SumMessageBuffer::new(32);
        let coord_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        msg.serialize(&mut buffer, &coord_pk);
        assert_eq!(buffer.tag(), [Tag::Sum as u8].as_ref());
        assert_eq!(buffer.coord_pk(), coord_pk.as_ref());
        assert_eq!(buffer.part_pk(), pk.as_ref());
        assert_eq!(buffer.sum_signature(), sum_signature.as_ref());
        assert_eq!(buffer.ephm_pk(), ephm_pk.as_ref());
        assert_eq!(
            buffer.certificate_len(),
            certificate.len().to_le_bytes().as_ref(),
        );
        assert_eq!(buffer.certificate(), certificate.as_ref());
    }

    #[test]
    fn test_summessage_deserialize() {
        // deserialize
        let bytes = auxiliary_bytes();
        let buffer = SumMessageBuffer::try_from(bytes.clone()).unwrap();
        let msg = SumMessage::deserialize(buffer.clone());
        assert_eq!(
            msg.pk(),
            &sign::PublicKey::from_slice(&bytes[MB::PART_PK_RANGE]).unwrap(),
        );
        assert_eq!(
            msg.sum_signature(),
            &sign::Signature::from_slice(&bytes[MB::SUM_SIGNATURE_RANGE]).unwrap(),
        );
        assert_eq!(
            msg.ephm_pk(),
            &box_::PublicKey::from_slice(&bytes[MB::EPHM_PK_RANGE]).unwrap(),
        );
        assert_eq!(
            msg.certificate(),
            &bytes[buffer.certificate_range.clone()].into(),
        );
    }

    #[test]
    fn test_summessage() {
        // seal
        let (pk, sk) = sign::gen_keypair();
        let sum_signature = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let ephm_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let certificate = Certificate::new();
        let (coord_pk, coord_sk) = box_::gen_keypair();
        let bytes = SumMessage::from_parts(&pk, &sum_signature, &ephm_pk, &certificate)
            .seal(&sk, &coord_pk);

        // open
        let msg = SumMessage::open(&bytes, &coord_pk, &coord_sk).unwrap();
        assert_eq!(msg.pk(), &pk);
        assert_eq!(msg.sum_signature(), &sum_signature);
        assert_eq!(msg.ephm_pk(), &ephm_pk);
        assert_eq!(msg.certificate(), &certificate);

        // wrong signature
        let bytes = auxiliary_bytes();
        let mut buffer = SumMessageBuffer::try_from(bytes).unwrap();
        let msg = SumMessage::from_parts(&pk, &sum_signature, &ephm_pk, &certificate);
        msg.serialize(&mut buffer, &coord_pk);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            SumMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong receiver
        msg.serialize(
            &mut buffer,
            &box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
        );
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            SumMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong tag
        buffer.tag_mut().copy_from_slice([Tag::None as u8].as_ref());
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            SumMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong length
        assert_eq!(
            SumMessage::open([0_u8; 0].as_ref(), &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );
    }
}
