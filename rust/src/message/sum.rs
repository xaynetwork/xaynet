use std::{borrow::Borrow, convert::TryFrom, ops::Range};

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{Certificate, MessageBuffer, LEN_BYTES, PK_BYTES, SIGNATURE_BYTES, SUM_TAG, TAG_BYTES};
use crate::{
    CoordinatorPublicKey, CoordinatorSecretKey, ParticipantTaskSignature, PetError,
    SumParticipantEphemeralPublicKey, SumParticipantPublicKey, SumParticipantSecretKey,
};

#[derive(Clone, Debug)]
/// Access to sum message buffer fields.
struct SumMessageBuffer<B> {
    bytes: B,
}

impl SumMessageBuffer<Vec<u8>> {
    /// Create an empty sum message buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }
}

impl TryFrom<Vec<u8>> for SumMessageBuffer<Vec<u8>> {
    type Error = PetError;

    /// Create a sum message buffer from `bytes`. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let buffer = Self { bytes };
        if buffer.len() >= buffer.certificate_len_range().end
            && buffer.len() == buffer.ephm_pk_range().end
        {
            Ok(buffer)
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> MessageBuffer for SumMessageBuffer<B> {
    /// Get a reference to the sum message buffer.
    fn bytes(&'_ self) -> &'_ [u8] {
        self.bytes.as_ref()
    }

    /// Get a mutable reference to the sum message buffer.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> SumMessageBuffer<B> {
    /// Get the range of the public ephemeral key field.
    fn ephm_pk_range(&self) -> Range<usize> {
        self.sum_signature_range().end..self.sum_signature_range().end + PK_BYTES
    }

    /// Get a reference to the public ephemeral key field.
    fn ephm_pk(&'_ self) -> &'_ [u8] {
        let range = self.ephm_pk_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the public ephemeral key field.
    fn ephm_pk_mut(&mut self) -> &mut [u8] {
        let range = self.ephm_pk_range();
        &mut self.bytes_mut()[range]
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Encryption and decryption of sum messages.
pub struct SumMessage<K, C, S, E>
where
    K: Borrow<SumParticipantPublicKey>,
    C: Borrow<Certificate>,
    S: Borrow<ParticipantTaskSignature>,
    E: Borrow<SumParticipantEphemeralPublicKey>,
{
    pk: K,
    certificate: C,
    sum_signature: S,
    ephm_pk: E,
}

impl<K, C, S, E> SumMessage<K, C, S, E>
where
    K: Borrow<SumParticipantPublicKey>,
    C: Borrow<Certificate>,
    S: Borrow<ParticipantTaskSignature>,
    E: Borrow<SumParticipantEphemeralPublicKey>,
{
    /// Create a sum message from its parts.
    pub fn from_parts(pk: K, certificate: C, sum_signature: S, ephm_pk: E) -> Self {
        Self {
            pk,
            certificate,
            sum_signature,
            ephm_pk,
        }
    }

    /// Get the length of a serialized sum message.
    fn len(&self) -> usize {
        SIGNATURE_BYTES
            + TAG_BYTES
            + PK_BYTES
            + PK_BYTES
            + LEN_BYTES
            + self.certificate.borrow().len()
            + SIGNATURE_BYTES
            + PK_BYTES
    }

    /// Serialize the sum message into a buffer.
    fn serialize(&self, buffer: &mut SumMessageBuffer<Vec<u8>>, pk: &CoordinatorPublicKey) {
        buffer.tag_mut().copy_from_slice(&[SUM_TAG]);
        buffer.coord_pk_mut().copy_from_slice(pk.as_ref());
        buffer
            .part_pk_mut()
            .copy_from_slice(self.pk.borrow().as_ref());
        buffer
            .certificate_len_mut()
            .copy_from_slice(&self.certificate.borrow().len().to_le_bytes());
        buffer
            .certificate_mut()
            .copy_from_slice(self.certificate.borrow().as_ref());
        buffer
            .sum_signature_mut()
            .copy_from_slice(self.sum_signature.borrow().as_ref());
        buffer
            .ephm_pk_mut()
            .copy_from_slice(self.ephm_pk.borrow().as_ref());
    }

    /// Sign and encrypt the sum message.
    pub fn seal(&self, sk: &SumParticipantSecretKey, pk: &CoordinatorPublicKey) -> Vec<u8> {
        let mut buffer = SumMessageBuffer::new(self.len());
        self.serialize(&mut buffer, pk);
        let signature = sign::sign_detached(buffer.message(), sk);
        buffer.signature_mut().copy_from_slice(signature.as_ref());
        sealedbox::seal(buffer.bytes(), pk)
    }
}

impl
    SumMessage<
        SumParticipantPublicKey,
        Certificate,
        ParticipantTaskSignature,
        SumParticipantEphemeralPublicKey,
    >
{
    /// Deserialize a sum message from a buffer.
    fn deserialize(buffer: SumMessageBuffer<Vec<u8>>) -> Self {
        // safe unwraps: lengths of slices are guaranteed by constants
        let pk = sign::PublicKey::from_slice(buffer.part_pk()).unwrap();
        let certificate = buffer.certificate().into();
        let sum_signature = sign::Signature::from_slice(buffer.sum_signature()).unwrap();
        let ephm_pk = box_::PublicKey::from_slice(buffer.ephm_pk()).unwrap();
        Self {
            pk,
            certificate,
            sum_signature,
            ephm_pk,
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
        if buffer.tag() == [SUM_TAG]
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

    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }

    /// Get a reference to the sum signature.
    pub fn sum_signature(&self) -> &ParticipantTaskSignature {
        &self.sum_signature
    }

    /// Get a reference to the ephemeral public encryption key.
    pub fn ephm_pk(&self) -> &SumParticipantEphemeralPublicKey {
        &self.ephm_pk
    }
}

#[cfg(test)]
mod tests {
    use sodiumoxide::randombytes::randombytes;

    use super::*;

    fn auxiliary_bytes() -> Vec<u8> {
        [
            randombytes(129).as_slice(),
            &(0 as usize).to_le_bytes(),
            randombytes(96).as_slice(),
        ]
        .concat()
    }

    #[test]
    fn test_summessagebuffer_ranges() {
        let bytes = auxiliary_bytes();
        let buffer = SumMessageBuffer { bytes };
        assert_eq!(buffer.ephm_pk_range(), 193 + LEN_BYTES..225 + LEN_BYTES);
    }

    #[test]
    fn test_summessagebuffer_fields() {
        // new
        assert_eq!(SumMessageBuffer::new(10).bytes, vec![0_u8; 10]);

        // try from
        assert_eq!(
            SumMessageBuffer::try_from(vec![0_u8; 10]).unwrap_err(),
            PetError::InvalidMessage,
        );
        let mut bytes = auxiliary_bytes();
        let mut buffer = SumMessageBuffer::try_from(bytes.clone()).unwrap();
        assert_eq!(buffer.bytes, bytes);

        // ephm pk
        let range = buffer.ephm_pk_range();
        assert_eq!(buffer.ephm_pk(), &bytes[range.clone()]);
        assert_eq!(buffer.ephm_pk_mut(), &mut bytes[range]);
    }

    #[test]
    fn test_summessage_serialize() {
        // from parts
        let pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let certificate = &Vec::<u8>::new().into();
        let sum_signature = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let ephm_pk = &box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let msg = SumMessage::from_parts(pk, certificate, sum_signature, ephm_pk);
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
            msg.ephm_pk as *const box_::PublicKey,
            ephm_pk as *const box_::PublicKey,
        );
        assert_eq!(msg.len(), 225 + LEN_BYTES);

        // serialize
        let mut buffer = SumMessageBuffer::new(225 + LEN_BYTES);
        let coord_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        msg.serialize(&mut buffer, &coord_pk);
        assert_eq!(buffer.tag(), &[SUM_TAG]);
        assert_eq!(buffer.coord_pk(), coord_pk.as_ref());
        assert_eq!(buffer.part_pk(), pk.as_ref());
        assert_eq!(buffer.certificate_len(), &(0 as usize).to_le_bytes());
        assert_eq!(buffer.certificate(), certificate.as_ref());
        assert_eq!(buffer.sum_signature(), sum_signature.as_ref());
        assert_eq!(buffer.ephm_pk(), ephm_pk.as_ref());
    }

    #[test]
    fn test_summessage_deserialize() {
        // deserialize
        let bytes = auxiliary_bytes();
        let buffer = SumMessageBuffer::try_from(bytes.clone()).unwrap();
        let msg = SumMessage::deserialize(buffer.clone());
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
            msg.ephm_pk(),
            &box_::PublicKey::from_slice(&bytes[buffer.ephm_pk_range()]).unwrap(),
        );
    }

    #[test]
    fn test_summessage() {
        // seal
        let (pk, sk) = sign::gen_keypair();
        let certificate = Vec::<u8>::new().into();
        let sum_signature = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let ephm_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let (coord_pk, coord_sk) = box_::gen_keypair();
        let bytes = SumMessage::from_parts(&pk, &certificate, &sum_signature, &ephm_pk)
            .seal(&sk, &coord_pk);

        // open
        let msg = SumMessage::open(&bytes, &coord_pk, &coord_sk).unwrap();
        assert_eq!(msg.pk(), &pk);
        assert_eq!(msg.certificate(), &certificate);
        assert_eq!(msg.sum_signature(), &sum_signature);
        assert_eq!(msg.ephm_pk(), &ephm_pk);

        // wrong signature
        let bytes = auxiliary_bytes();
        let mut buffer = SumMessageBuffer::try_from(bytes).unwrap();
        let msg = SumMessage::from_parts(&pk, &certificate, &sum_signature, &ephm_pk);
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
        buffer.tag_mut().copy_from_slice(&[0_u8]);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            SumMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong length
        let buffer = SumMessageBuffer::new(10);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            SumMessage::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );
    }
}
