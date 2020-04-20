use std::{borrow::Borrow, ops::Range};

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{Certificate, MessageBuffer, CERTIFICATE_BYTES, SUM2_TAG, TAG_BYTES};
use crate::{
    CoordinatorPublicKey, CoordinatorSecretKey, ParticipantTaskSignature, PetError,
    SumParticipantPublicKey, SumParticipantSecretKey,
};

// sum2 message buffer field ranges
const MASK_BYTES: usize = 32;
const MASK_RANGE: Range<usize> = 193..225; // 32 bytes

#[derive(Debug)]
/// Access to sum2 message buffer fields.
struct Sum2MessageBuffer<B> {
    bytes: B,
}

impl Sum2MessageBuffer<Vec<u8>> {
    /// Create an empty sum2 message buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }

    /// Create a sum2 message buffer from `bytes`. Fails if the `bytes` don't conform to the
    /// expected sum2 message length `exp_len`.
    fn try_from(bytes: Vec<u8>, exp_len: usize) -> Result<Self, PetError> {
        if bytes.len() != exp_len {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self { bytes })
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> MessageBuffer for Sum2MessageBuffer<B> {
    /// Get a reference to the sum2 message buffer.
    fn bytes(&'_ self) -> &'_ [u8] {
        self.bytes.as_ref()
    }

    /// Get a mutable reference to the sum2 message buffer.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> Sum2MessageBuffer<B> {
    /// Get a reference to the mask field of the sum2 message buffer.
    fn mask(&'_ self) -> &'_ [u8] {
        &self.bytes()[MASK_RANGE]
    }

    /// Get a mutable reference to the mask field of the sum2 message buffer.
    fn mask_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[MASK_RANGE]
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Encryption and decryption of sum2 messages.
pub struct Sum2Message<K, C, S, M>
where
    K: Borrow<SumParticipantPublicKey>,
    C: Borrow<Certificate>,
    S: Borrow<ParticipantTaskSignature>,
    M: Borrow<Vec<u8>>,
{
    pk: K,
    certificate: C,
    sum_signature: S,
    mask: M,
}

impl<K, C, S, M> Sum2Message<K, C, S, M>
where
    K: Borrow<SumParticipantPublicKey>,
    C: Borrow<Certificate>,
    S: Borrow<ParticipantTaskSignature>,
    M: Borrow<Vec<u8>>,
{
    /// Create a sum2 message from its parts.
    pub fn from_parts(pk: K, certificate: C, sum_signature: S, mask: M) -> Self {
        Self {
            pk,
            certificate,
            sum_signature,
            mask,
        }
    }

    /// Get the expected length of a serialized sum2 message.
    const fn exp_len() -> usize {
        sign::SIGNATUREBYTES
            + TAG_BYTES
            + box_::PUBLICKEYBYTES
            + sign::PUBLICKEYBYTES
            + CERTIFICATE_BYTES
            + sign::SIGNATUREBYTES
            + MASK_BYTES
    }

    /// Serialize the sum2 message into a buffer.
    fn serialize(&self, buffer: &mut Sum2MessageBuffer<Vec<u8>>, pk: &CoordinatorPublicKey) {
        buffer.tag_mut().copy_from_slice(&[SUM2_TAG]);
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
            .mask_mut()
            .copy_from_slice(self.mask.borrow().as_ref());
    }

    /// Sign and encrypt the sum2message.
    pub fn seal(&self, sk: &SumParticipantSecretKey, pk: &CoordinatorPublicKey) -> Vec<u8> {
        let mut buffer = Sum2MessageBuffer::new(Self::exp_len());
        self.serialize(&mut buffer, pk);
        let signature = sign::sign_detached(buffer.message(), sk);
        buffer.signature_mut().copy_from_slice(signature.as_ref());
        sealedbox::seal(buffer.bytes(), pk)
    }
}

impl Sum2Message<SumParticipantPublicKey, Certificate, ParticipantTaskSignature, Vec<u8>> {
    /// Deserialize a sum2 message from a buffer. Fails if the
    /// `buffer` doesn't conform to the expected sum2 message length
    /// `exp_len`.
    fn deserialize(buffer: Sum2MessageBuffer<Vec<u8>>) -> Self {
        // safe unwraps: lengths of `buffer` slices are guaranteed by constants
        let pk = sign::PublicKey::from_slice(buffer.part_pk()).unwrap();
        let certificate = buffer.certificate().into();
        let sum_signature = sign::Signature::from_slice(buffer.sum_signature()).unwrap();
        let mask = buffer.mask().to_vec();
        Self {
            pk,
            certificate,
            sum_signature,
            mask,
        }
    }

    /// Decrypt and verify a sum2 message. Fails if decryption or validation fails.
    pub fn open(
        bytes: &[u8],
        pk: &CoordinatorPublicKey,
        sk: &CoordinatorSecretKey,
    ) -> Result<Self, PetError> {
        let buffer = Sum2MessageBuffer::try_from(
            sealedbox::open(bytes, pk, sk).or(Err(PetError::InvalidMessage))?,
            Self::exp_len(),
        )?;
        if buffer.tag() != [SUM2_TAG]
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
    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }

    /// Get a reference to the sum signature.
    pub fn sum_signature(&self) -> &ParticipantTaskSignature {
        &self.sum_signature
    }

    /// Get a reference to the mask.
    pub fn mask(&self) -> &Vec<u8> {
        &self.mask
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
        assert_eq!(MASK_RANGE.end - MASK_RANGE.start, MASK_BYTES);
    }

    #[test]
    fn test_sum2messagebuffer() {
        // new
        assert_eq!(Sum2MessageBuffer::new(10).bytes, vec![0_u8; 10]);

        // try from
        let mut bytes = randombytes(225);
        let mut buffer = Sum2MessageBuffer::try_from(bytes.clone(), 225).unwrap();
        assert_eq!(buffer.bytes, bytes);
        assert_eq!(
            Sum2MessageBuffer::try_from(bytes.clone(), 10).unwrap_err(),
            PetError::InvalidMessage,
        );

        // mask
        assert_eq!(buffer.mask(), &bytes[MASK_RANGE]);
        assert_eq!(buffer.mask_mut(), &mut bytes[MASK_RANGE]);
    }

    #[test]
    fn test_sum2message_serialize() {
        // from parts
        let pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let certificate = &Vec::<u8>::new().into();
        let sum_signature = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let mask = &randombytes(32);
        let msg = Sum2Message::from_parts(pk, certificate, sum_signature, mask);
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
        assert_eq!(msg.mask as *const Vec<u8>, mask as *const Vec<u8>,);

        // serialize
        let mut buffer = Sum2MessageBuffer::new(225);
        let coord_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        msg.serialize(&mut buffer, &coord_pk);
        assert_eq!(buffer.tag(), &[SUM2_TAG]);
        assert_eq!(buffer.coord_pk(), coord_pk.as_ref());
        assert_eq!(buffer.part_pk(), pk.as_ref());
        assert_eq!(buffer.certificate(), certificate.as_ref());
        assert_eq!(buffer.sum_signature(), sum_signature.as_ref());
        assert_eq!(buffer.mask(), mask.as_slice());
    }

    #[test]
    fn test_sum2message_deserialize() {
        // deserialize
        let bytes = randombytes(225);
        let buffer = Sum2MessageBuffer::try_from(bytes.clone(), 225).unwrap();
        let msg = Sum2Message::deserialize(buffer);
        assert_eq!(
            msg.pk(),
            &sign::PublicKey::from_slice(&bytes[PART_PK_RANGE]).unwrap(),
        );
        assert_eq!(msg.certificate(), &bytes[CERTIFICATE_RANGE].into());
        assert_eq!(
            msg.sum_signature(),
            &sign::Signature::from_slice(&bytes[SUM_SIGNATURE_RANGE]).unwrap(),
        );
        assert_eq!(msg.mask(), &bytes[MASK_RANGE].to_vec());
    }

    #[test]
    fn test_sum2message() {
        // seal
        let (pk, sk) = sign::gen_keypair();
        let certificate = Vec::<u8>::new().into();
        let sum_signature = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let mask = randombytes(32);
        let (coord_pk, coord_sk) = box_::gen_keypair();
        let bytes =
            Sum2Message::from_parts(&pk, &certificate, &sum_signature, &mask).seal(&sk, &coord_pk);

        // open
        let msg = Sum2Message::open(&bytes, &coord_pk, &coord_sk).unwrap();
        assert_eq!(msg.pk(), &pk);
        assert_eq!(msg.certificate(), &certificate);
        assert_eq!(msg.sum_signature(), &sum_signature);
        assert_eq!(msg.mask(), &mask);

        // wrong signature
        let mut buffer = Sum2MessageBuffer::new(225);
        let msg = Sum2Message::from_parts(&pk, &certificate, &sum_signature, &mask);
        msg.serialize(&mut buffer, &coord_pk);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            Sum2Message::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong receiver
        msg.serialize(
            &mut buffer,
            &box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
        );
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            Sum2Message::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong tag
        buffer.tag_mut().copy_from_slice(&[0_u8]);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            Sum2Message::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong length
        let buffer = Sum2MessageBuffer::new(10);
        let bytes = sealedbox::seal(buffer.bytes(), &coord_pk);
        assert_eq!(
            Sum2Message::open(&bytes, &coord_pk, &coord_sk).unwrap_err(),
            PetError::InvalidMessage,
        );
    }
}
