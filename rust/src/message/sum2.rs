use std::{
    borrow::Borrow,
    convert::{TryFrom, TryInto},
    ops::Range,
};

use sodiumoxide::crypto::{sealedbox, sign};

use super::{
    Certificate,
    MessageBuffer,
    LEN_BYTES,
    PK_BYTES,
    SIGNATURE_BYTES,
    SUM2_TAG,
    TAG_BYTES,
};
use crate::{
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    ParticipantTaskSignature,
    PetError,
    SumParticipantPublicKey,
    SumParticipantSecretKey,
};

#[derive(Clone, Debug, PartialEq)]
/// A mask. (TODO: move this to the masking module later on.)
pub struct Mask(Vec<u8>);

impl Mask {
    /// Get the length of the mask.
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl AsRef<[u8]> for Mask {
    /// Get a reference to the mask.
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for Mask {
    /// Create a mask from bytes.
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for Mask {
    /// Create a mask from a slice of bytes.
    fn from(slice: &[u8]) -> Self {
        Self(slice.to_vec())
    }
}

#[derive(Clone, Debug)]
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
}

impl TryFrom<Vec<u8>> for Sum2MessageBuffer<Vec<u8>> {
    type Error = PetError;

    /// Create a sum2 message buffer from `bytes`. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let buffer = Self { bytes };
        if buffer.len() >= buffer.certificate_len_range().end
            && buffer.len() >= buffer.mask_len_range().end
            && buffer.len() == buffer.mask_range().end
        {
            Ok(buffer)
        } else {
            Err(PetError::InvalidMessage)
        }
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
    /// Get the range of the mask length field.
    fn mask_len_range(&self) -> Range<usize> {
        self.sum_signature_range().end..self.sum_signature_range().end + LEN_BYTES
    }

    /// Get a reference to the mask length field.
    fn mask_len(&'_ self) -> &'_ [u8] {
        let range = self.mask_len_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the mask length field.
    fn mask_len_mut(&mut self) -> &mut [u8] {
        let range = self.mask_len_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the number of bytes of the mask field.
    fn mask_bytes(&self) -> usize {
        // safe unwrap: length of slice is guaranteed by constants
        usize::from_le_bytes(self.mask_len().try_into().unwrap())
    }

    /// Get the range of the mask field.
    fn mask_range(&self) -> Range<usize> {
        self.mask_len_range().end..self.mask_len_range().end + self.mask_bytes()
    }

    /// Get a reference to the mask field.
    fn mask(&'_ self) -> &'_ [u8] {
        let range = self.mask_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the mask field.
    fn mask_mut(&mut self) -> &mut [u8] {
        let range = self.mask_range();
        &mut self.bytes_mut()[range]
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Encryption and decryption of sum2 messages.
pub struct Sum2Message<K, C, S, M>
where
    K: Borrow<SumParticipantPublicKey>,
    C: Borrow<Certificate>,
    S: Borrow<ParticipantTaskSignature>,
    M: Borrow<Mask>,
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
    M: Borrow<Mask>,
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

    /// Get the length of a serialized sum2 message.
    fn len(&self) -> usize {
        SIGNATURE_BYTES
            + TAG_BYTES
            + PK_BYTES
            + PK_BYTES
            + LEN_BYTES
            + self.certificate.borrow().len()
            + SIGNATURE_BYTES
            + LEN_BYTES
            + self.mask.borrow().len()
    }

    /// Serialize the sum2 message into a buffer.
    fn serialize(&self, buffer: &mut Sum2MessageBuffer<Vec<u8>>, pk: &CoordinatorPublicKey) {
        buffer.tag_mut().copy_from_slice(&[SUM2_TAG]);
        buffer.coord_pk_mut().copy_from_slice(pk.borrow().as_ref());
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
            .mask_len_mut()
            .copy_from_slice(&self.mask.borrow().len().to_le_bytes());
        buffer
            .mask_mut()
            .copy_from_slice(self.mask.borrow().as_ref());
    }

    /// Sign and encrypt the sum2message.
    pub fn seal(&self, sk: &SumParticipantSecretKey, pk: &CoordinatorPublicKey) -> Vec<u8> {
        let mut buffer = Sum2MessageBuffer::new(self.len());
        self.serialize(&mut buffer, pk);
        let signature = sign::sign_detached(buffer.message(), sk);
        buffer.signature_mut().copy_from_slice(signature.as_ref());
        sealedbox::seal(buffer.bytes(), pk)
    }
}

impl Sum2Message<SumParticipantPublicKey, Certificate, ParticipantTaskSignature, Mask> {
    /// Deserialize a sum2 message from a buffer.
    fn deserialize(buffer: Sum2MessageBuffer<Vec<u8>>) -> Self {
        // safe unwraps: lengths of slices are guaranteed by constants
        let pk = sign::PublicKey::from_slice(buffer.part_pk()).unwrap();
        let certificate = buffer.certificate().into();
        let sum_signature = sign::Signature::from_slice(buffer.sum_signature()).unwrap();
        let mask = buffer.mask().into();
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
        )?;
        if buffer.tag() == [SUM2_TAG]
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

    /// Get a reference to the mask.
    pub fn mask(&self) -> &Mask {
        &self.mask
    }
}

#[cfg(test)]
mod tests {
    use sodiumoxide::{crypto::box_, randombytes::randombytes};

    use super::*;

    fn auxiliary_bytes() -> Vec<u8> {
        [
            randombytes(129).as_slice(),
            &(0 as usize).to_le_bytes(),
            randombytes(64).as_slice(),
            &(32 as usize).to_le_bytes(),
            randombytes(32).as_slice(),
        ]
        .concat()
    }

    #[test]
    fn test_sum2messagebuffer_ranges() {
        let bytes = auxiliary_bytes();
        let buffer = Sum2MessageBuffer { bytes };
        assert_eq!(
            buffer.mask_range(),
            193 + 2 * LEN_BYTES..225 + 2 * LEN_BYTES,
        );
    }

    #[test]
    fn test_sum2messagebuffer_fields() {
        // new
        assert_eq!(Sum2MessageBuffer::new(10).bytes, vec![0_u8; 10]);

        // try from
        assert_eq!(
            Sum2MessageBuffer::try_from(vec![0_u8; 10]).unwrap_err(),
            PetError::InvalidMessage,
        );
        let mut bytes = auxiliary_bytes();
        let mut buffer = Sum2MessageBuffer::try_from(bytes.clone()).unwrap();
        assert_eq!(buffer.bytes, bytes);

        // mask length
        let range = buffer.mask_len_range();
        assert_eq!(buffer.mask_len(), &bytes[range.clone()]);
        assert_eq!(buffer.mask_len_mut(), &mut bytes[range]);
        assert_eq!(buffer.mask_bytes(), 32);

        // mask
        let range = buffer.mask_range();
        assert_eq!(buffer.mask(), &bytes[range.clone()]);
        assert_eq!(buffer.mask_mut(), &mut bytes[range]);
    }

    #[test]
    fn test_sum2message_serialize() {
        // from parts
        let pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let certificate = &Vec::<u8>::new().into();
        let sum_signature = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let mask = &randombytes(32).into();
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
        assert_eq!(msg.mask as *const Mask, mask as *const Mask);
        assert_eq!(msg.len(), 225 + 2 * LEN_BYTES);

        // serialize
        let mut buffer = Sum2MessageBuffer::new(225 + 2 * LEN_BYTES);
        let coord_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        msg.serialize(&mut buffer, &coord_pk);
        assert_eq!(buffer.tag(), &[SUM2_TAG]);
        assert_eq!(buffer.coord_pk(), coord_pk.as_ref());
        assert_eq!(buffer.part_pk(), pk.as_ref());
        assert_eq!(buffer.certificate_len(), &(0 as usize).to_le_bytes());
        assert_eq!(buffer.certificate(), certificate.as_ref());
        assert_eq!(buffer.sum_signature(), sum_signature.as_ref());
        assert_eq!(buffer.mask_len(), &(32 as usize).to_le_bytes());
        assert_eq!(buffer.mask(), mask.as_ref());
    }

    #[test]
    fn test_sum2message_deserialize() {
        // deserialize
        let bytes = auxiliary_bytes();
        let buffer = Sum2MessageBuffer::try_from(bytes.clone()).unwrap();
        let msg = Sum2Message::deserialize(buffer.clone());
        assert_eq!(
            msg.pk(),
            &sign::PublicKey::from_slice(&bytes[buffer.part_pk_range()]).unwrap(),
        );
        assert_eq!(msg.certificate(), &bytes[buffer.certificate_range()].into());
        assert_eq!(
            msg.sum_signature(),
            &sign::Signature::from_slice(&bytes[buffer.sum_signature_range()]).unwrap(),
        );
        assert_eq!(msg.mask(), &bytes[buffer.mask_range()].into());
    }

    #[test]
    fn test_sum2message() {
        // seal
        let (pk, sk) = sign::gen_keypair();
        let certificate = Vec::<u8>::new().into();
        let sum_signature = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let mask = randombytes(32).into();
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
        let bytes = auxiliary_bytes();
        let mut buffer = Sum2MessageBuffer::try_from(bytes).unwrap();
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
