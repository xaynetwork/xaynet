use std::ops::Range;

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{MessageBuffer, CERTIFICATE_BYTES, SUM_TAG, TAG_BYTES};
use crate::{
    CoordinatorPublicKey, CoordinatorSecretKey, ParticipantTaskSignature, PetError,
    SumParticipantEphemeralPublicKey, SumParticipantPublicKey, SumParticipantSecretKey,
};

// sum message buffer field ranges
const EPHM_PK_RANGE: Range<usize> = 193..225; // 32 bytes

#[derive(Debug)]
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

    /// Create a sum message buffer from `bytes`. Fails if the `bytes` don't conform to the expected
    /// sum message length `exp_len`.
    fn from(bytes: Vec<u8>, exp_len: usize) -> Result<Self, PetError> {
        if bytes.len() != exp_len {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self { bytes })
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
    /// Get a reference to the public ephemeral key field of the sum message buffer.
    fn ephm_pk(&'_ self) -> &'_ [u8] {
        &self.bytes()[EPHM_PK_RANGE]
    }

    /// Get a mutable reference to the public ephemeral key field of the sum message buffer.
    fn ephm_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[EPHM_PK_RANGE]
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Encryption and decryption of sum messages.
pub struct SumMessage<K, C, S, E> {
    pk: K,
    certificate: C,
    sum_signature: S,
    ephm_pk: E,
}

impl<K, C, S, E> SumMessage<K, C, S, E> {
    /// Create a sum message from its parts.
    pub fn from(pk: K, certificate: C, sum_signature: S, ephm_pk: E) -> Self {
        Self {
            pk,
            certificate,
            sum_signature,
            ephm_pk,
        }
    }

    /// Get the expected length of a serialized sum message.
    const fn exp_len() -> usize {
        sign::SIGNATUREBYTES
            + TAG_BYTES
            + box_::PUBLICKEYBYTES
            + sign::PUBLICKEYBYTES
            + CERTIFICATE_BYTES
            + sign::SIGNATUREBYTES
            + box_::PUBLICKEYBYTES
    }
}

impl
    SumMessage<
        &SumParticipantPublicKey,
        &Vec<u8>,
        &ParticipantTaskSignature,
        &SumParticipantEphemeralPublicKey,
    >
{
    /// Serialize the sum message into a buffer.
    fn serialize(&self, buffer: &mut SumMessageBuffer<Vec<u8>>, pk: &CoordinatorPublicKey) {
        buffer.tag_mut().copy_from_slice(&[SUM_TAG]);
        buffer.coord_pk_mut().copy_from_slice(pk.as_ref());
        buffer.part_pk_mut().copy_from_slice(self.pk.as_ref());
        buffer.certificate_mut().copy_from_slice(self.certificate);
        buffer
            .sum_signature_mut()
            .copy_from_slice(self.sum_signature.as_ref());
        buffer.ephm_pk_mut().copy_from_slice(self.ephm_pk.as_ref());
    }

    /// Sign and encrypt the sum message.
    pub fn seal(&self, sk: &SumParticipantSecretKey, pk: &CoordinatorPublicKey) -> Vec<u8> {
        let mut buffer = SumMessageBuffer::new(Self::exp_len());
        self.serialize(&mut buffer, pk);
        let signature = sign::sign_detached(buffer.message(), sk);
        buffer.signature_mut().copy_from_slice(signature.as_ref());
        sealedbox::seal(buffer.bytes(), pk)
    }
}

impl
    SumMessage<
        SumParticipantPublicKey,
        Vec<u8>,
        ParticipantTaskSignature,
        SumParticipantEphemeralPublicKey,
    >
{
    /// Deserialize a sum message from a buffer. Fails if the `buffer` doesn't conform to the
    /// expected sum message length `exp_len`.
    fn deserialize(buffer: SumMessageBuffer<Vec<u8>>) -> Self {
        // safe unwraps: lengths of `buffer` slices are guaranteed by constants
        let pk = sign::PublicKey::from_slice(buffer.part_pk()).unwrap();
        let certificate = buffer.certificate().to_vec();
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
        let buffer = SumMessageBuffer::from(
            sealedbox::open(bytes, pk, sk).or(Err(PetError::InvalidMessage))?,
            Self::exp_len(),
        )?;
        if buffer.tag() != [SUM_TAG]
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
    pub fn pk(&self) -> &SumParticipantPublicKey {
        &self.pk
    }

    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &Vec<u8> {
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

    use super::{
        super::{CERTIFICATE_RANGE, PART_PK_RANGE, SUM_SIGNATURE_RANGE},
        *,
    };

    #[test]
    fn test_ranges() {
        assert_eq!(
            EPHM_PK_RANGE.end - EPHM_PK_RANGE.start,
            box_::PUBLICKEYBYTES,
        );
    }

    #[test]
    fn test_summessagebuffer() {
        // new
        assert_eq!(SumMessageBuffer::new(10).bytes, vec![0_u8; 10]);

        // from
        let mut bytes = randombytes(225);
        let mut buffer = SumMessageBuffer::from(bytes.clone(), 225).unwrap();
        assert_eq!(buffer.bytes, bytes);
        assert_eq!(
            SumMessageBuffer::from(bytes.clone(), 10).unwrap_err(),
            PetError::InvalidMessage,
        );

        // ephm pk
        assert_eq!(buffer.ephm_pk(), &bytes[EPHM_PK_RANGE]);
        assert_eq!(buffer.ephm_pk_mut(), &mut bytes[EPHM_PK_RANGE]);
    }

    #[test]
    fn test_summessage_serialize() {
        // from
        let pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let certificate = &Vec::<u8>::new();
        let sum_signature = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let ephm_pk = &box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let msg = SumMessage::from(pk, certificate, sum_signature, ephm_pk);
        assert_eq!(
            msg.pk as *const sign::PublicKey,
            pk as *const sign::PublicKey,
        );
        assert_eq!(
            msg.certificate as *const Vec<u8>,
            certificate as *const Vec<u8>,
        );
        assert_eq!(
            msg.sum_signature as *const sign::Signature,
            sum_signature as *const sign::Signature,
        );
        assert_eq!(
            msg.ephm_pk as *const box_::PublicKey,
            ephm_pk as *const box_::PublicKey,
        );

        // serialize
        let mut buffer = SumMessageBuffer::new(225);
        let coord_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        msg.serialize(&mut buffer, &coord_pk);
        assert_eq!(buffer.tag(), &[SUM_TAG]);
        assert_eq!(buffer.coord_pk(), coord_pk.as_ref());
        assert_eq!(buffer.part_pk(), pk.as_ref());
        assert_eq!(buffer.certificate(), certificate.as_slice());
        assert_eq!(buffer.sum_signature(), sum_signature.as_ref());
        assert_eq!(buffer.ephm_pk(), ephm_pk.as_ref());
    }

    #[test]
    fn test_summessage_deserialize() {
        // deserialize
        let bytes = randombytes(225);
        let buffer = SumMessageBuffer::from(bytes.clone(), 225).unwrap();
        let msg = SumMessage::deserialize(buffer);
        assert_eq!(
            msg.pk(),
            &sign::PublicKey::from_slice(&bytes[PART_PK_RANGE]).unwrap(),
        );
        assert_eq!(msg.certificate(), &bytes[CERTIFICATE_RANGE].to_vec());
        assert_eq!(
            msg.sum_signature(),
            &sign::Signature::from_slice(&bytes[SUM_SIGNATURE_RANGE]).unwrap(),
        );
        assert_eq!(
            msg.ephm_pk(),
            &box_::PublicKey::from_slice(&bytes[EPHM_PK_RANGE]).unwrap(),
        );
    }

    #[test]
    fn test_summessage() {
        // seal
        let (pk, sk) = sign::gen_keypair();
        let certificate = Vec::<u8>::new();
        let sum_signature = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let ephm_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let (coord_pk, coord_sk) = box_::gen_keypair();
        let bytes =
            SumMessage::from(&pk, &certificate, &sum_signature, &ephm_pk).seal(&sk, &coord_pk);

        // open
        let msg = SumMessage::open(&bytes, &coord_pk, &coord_sk).unwrap();
        assert_eq!(msg.pk(), &pk);
        assert_eq!(msg.certificate(), &certificate);
        assert_eq!(msg.sum_signature(), &sum_signature);
        assert_eq!(msg.ephm_pk(), &ephm_pk);

        // wrong signature
        let mut buffer = SumMessageBuffer::new(225);
        let msg = SumMessage::from(&pk, &certificate, &sum_signature, &ephm_pk);
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
