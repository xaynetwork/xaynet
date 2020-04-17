use std::ops::Range;

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{MessageBuffer, CERTIFICATE_BYTES, SUM2_TAG, TAG_BYTES};
use crate::PetError;

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
    fn from(bytes: Vec<u8>, exp_len: usize) -> Result<Self, PetError> {
        if bytes.len() != exp_len {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self { bytes })
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> MessageBuffer for Sum2MessageBuffer<B> {
    /// Get a reference to the sum2 message buffer.
    fn bytes<'b>(&'b self) -> &'b [u8] {
        self.bytes.as_ref()
    }

    /// Get a mutable reference to the sum2 message buffer.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }
}

impl<B: AsRef<[u8]> + AsMut<[u8]>> Sum2MessageBuffer<B> {
    /// Get a reference to the mask field of the sum2 message buffer.
    fn mask<'b>(&'b self) -> &'b [u8] {
        &self.bytes()[MASK_RANGE]
    }

    /// Get a mutable reference to the mask field of the sum2 message buffer.
    fn mask_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[MASK_RANGE]
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Encryption and decryption of sum2 messages.
pub struct Sum2Message<K, C, S, M> {
    sign_pk: K,
    certificate: C,
    sum_signature: S,
    mask: M,
}

impl<K, C, S, M> Sum2Message<K, C, S, M> {
    /// Create a sum2 message from its parts.
    pub fn from(sign_pk: K, certificate: C, sum_signature: S, mask: M) -> Self {
        Self {
            sign_pk,
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
}

impl Sum2Message<&sign::PublicKey, &Vec<u8>, &sign::Signature, &Vec<u8>> {
    /// Serialize the sum2 message into a buffer.
    fn serialize(&self, buffer: &mut Sum2MessageBuffer<Vec<u8>>, encr_pk: &box_::PublicKey) {
        buffer.tag_mut().copy_from_slice(&[SUM2_TAG]);
        buffer.encr_pk_mut().copy_from_slice(encr_pk.as_ref());
        buffer.sign_pk_mut().copy_from_slice(self.sign_pk.as_ref());
        buffer.certificate_mut().copy_from_slice(self.certificate);
        buffer
            .sum_signature_mut()
            .copy_from_slice(self.sum_signature.as_ref());
        buffer.mask_mut().copy_from_slice(self.mask);
    }

    /// Sign and encrypt the sum2message.
    pub fn seal(&self, sign_sk: &sign::SecretKey, encr_pk: &box_::PublicKey) -> Vec<u8> {
        let mut buffer = Sum2MessageBuffer::new(Self::exp_len());
        self.serialize(&mut buffer, encr_pk);
        let signature = sign::sign_detached(buffer.message(), sign_sk);
        buffer.signature_mut().copy_from_slice(signature.as_ref());
        sealedbox::seal(buffer.bytes(), encr_pk)
    }
}

impl Sum2Message<sign::PublicKey, Vec<u8>, sign::Signature, Vec<u8>> {
    /// Deserialize a sum2 message from a buffer. Fails if the `buffer` doesn't conform to the
    /// expected sum2 message length `exp_len`.
    fn deserialize(buffer: Sum2MessageBuffer<Vec<u8>>) -> Self {
        // safe unwraps: lengths of `buffer` slices are guaranteed by constants
        let sign_pk = sign::PublicKey::from_slice(buffer.sign_pk()).unwrap();
        let certificate = buffer.certificate().to_vec();
        let sum_signature = sign::Signature::from_slice(buffer.sum_signature()).unwrap();
        let mask = buffer.mask().to_vec();
        Self {
            sign_pk,
            certificate,
            sum_signature,
            mask,
        }
    }

    /// Decrypt and verify a sum2 message. Fails if decryption or validation fails.
    pub fn open(
        bytes: &[u8],
        encr_pk: &box_::PublicKey,
        encr_sk: &box_::SecretKey,
    ) -> Result<Self, PetError> {
        let buffer = Sum2MessageBuffer::from(
            sealedbox::open(bytes, encr_pk, encr_sk).or(Err(PetError::InvalidMessage))?,
            Self::exp_len(),
        )?;
        if buffer.tag() != [SUM2_TAG]
            || buffer.encr_pk() != encr_pk.as_ref()
            || !sign::verify_detached(
                // safe unwraps: lengths of `buffer` slices are guaranteed by constants
                &sign::Signature::from_slice(buffer.signature()).unwrap(),
                buffer.message(),
                &sign::PublicKey::from_slice(buffer.sign_pk()).unwrap(),
            )
        {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self::deserialize(buffer))
    }

    /// Get a reference to the public signature key.
    pub fn sign_pk(&self) -> &sign::PublicKey {
        &self.sign_pk
    }

    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &Vec<u8> {
        &self.certificate
    }

    /// Get a reference to the sum signature.
    pub fn sum_signature(&self) -> &sign::Signature {
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
        super::{CERTIFICATE_RANGE, SIGN_PK_RANGE, SUM_SIGNATURE_RANGE},
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

        // from
        let mut bytes = randombytes(225);
        let mut buffer = Sum2MessageBuffer::from(bytes.clone(), 225).unwrap();
        assert_eq!(buffer.bytes, bytes);
        assert_eq!(
            Sum2MessageBuffer::from(bytes.clone(), 10).unwrap_err(),
            PetError::InvalidMessage,
        );

        // mask
        assert_eq!(buffer.mask(), &bytes[MASK_RANGE]);
        assert_eq!(buffer.mask_mut(), &mut bytes[MASK_RANGE]);
    }

    #[test]
    fn test_sum2message_serialize() {
        // from
        let sign_pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let certificate = &Vec::<u8>::new();
        let sum_signature = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let mask = &randombytes(32);
        let msg = Sum2Message::from(sign_pk, certificate, sum_signature, mask);
        assert_eq!(
            msg.sign_pk as *const sign::PublicKey,
            sign_pk as *const sign::PublicKey,
        );
        assert_eq!(
            msg.certificate as *const Vec<u8>,
            certificate as *const Vec<u8>,
        );
        assert_eq!(
            msg.sum_signature as *const sign::Signature,
            sum_signature as *const sign::Signature,
        );
        assert_eq!(msg.mask as *const Vec<u8>, mask as *const Vec<u8>,);

        // serialize
        let mut buffer = Sum2MessageBuffer::new(225);
        let encr_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        msg.serialize(&mut buffer, &encr_pk);
        assert_eq!(buffer.tag(), &[SUM2_TAG]);
        assert_eq!(buffer.sign_pk(), sign_pk.as_ref());
        assert_eq!(buffer.encr_pk(), encr_pk.as_ref());
        assert_eq!(buffer.certificate(), certificate.as_slice());
        assert_eq!(buffer.sum_signature(), sum_signature.as_ref());
        assert_eq!(buffer.mask(), mask.as_slice());
    }

    #[test]
    fn test_sum2message_deserialize() {
        // deserialize
        let bytes = randombytes(225);
        let buffer = Sum2MessageBuffer::from(bytes.clone(), 225).unwrap();
        let msg = Sum2Message::deserialize(buffer);
        assert_eq!(
            msg.sign_pk(),
            &sign::PublicKey::from_slice(&bytes[SIGN_PK_RANGE]).unwrap(),
        );
        assert_eq!(msg.certificate(), &bytes[CERTIFICATE_RANGE].to_vec());
        assert_eq!(
            msg.sum_signature(),
            &sign::Signature::from_slice(&bytes[SUM_SIGNATURE_RANGE]).unwrap(),
        );
        assert_eq!(msg.mask(), &bytes[MASK_RANGE].to_vec());
    }

    #[test]
    fn test_sum2message() {
        // seal
        let (sign_pk, sign_sk) = sign::gen_keypair();
        let certificate = Vec::<u8>::new();
        let sum_signature = sign::Signature::from_slice(&randombytes(64)).unwrap();
        let mask = randombytes(32);
        let (encr_pk, encr_sk) = box_::gen_keypair();
        let bytes = Sum2Message::from(&sign_pk, &certificate, &sum_signature, &mask)
            .seal(&sign_sk, &encr_pk);

        // open
        let msg = Sum2Message::open(&bytes, &encr_pk, &encr_sk).unwrap();
        assert_eq!(msg.sign_pk(), &sign_pk);
        assert_eq!(msg.certificate(), &certificate);
        assert_eq!(msg.sum_signature(), &sum_signature);
        assert_eq!(msg.mask(), &mask);

        // wrong signature
        let mut buffer = Sum2MessageBuffer::new(225);
        let msg = Sum2Message::from(&sign_pk, &certificate, &sum_signature, &mask);
        msg.serialize(&mut buffer, &encr_pk);
        let bytes = sealedbox::seal(buffer.bytes(), &encr_pk);
        assert_eq!(
            Sum2Message::open(&bytes, &encr_pk, &encr_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong receiver
        msg.serialize(
            &mut buffer,
            &box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
        );
        let bytes = sealedbox::seal(buffer.bytes(), &encr_pk);
        assert_eq!(
            Sum2Message::open(&bytes, &encr_pk, &encr_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong tag
        buffer.tag_mut().copy_from_slice(&[0_u8]);
        let bytes = sealedbox::seal(buffer.bytes(), &encr_pk);
        assert_eq!(
            Sum2Message::open(&bytes, &encr_pk, &encr_sk).unwrap_err(),
            PetError::InvalidMessage,
        );

        // wrong length
        let buffer = Sum2MessageBuffer::new(10);
        let bytes = sealedbox::seal(buffer.bytes(), &encr_pk);
        assert_eq!(
            Sum2Message::open(&bytes, &encr_pk, &encr_sk).unwrap_err(),
            PetError::InvalidMessage,
        );
    }
}
