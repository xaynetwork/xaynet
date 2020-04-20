pub mod sum;
pub mod sum2;
pub mod update;

use std::{
    convert::TryInto,
    mem,
    ops::{Range, RangeFrom, RangeTo},
};

use sodiumoxide::crypto::sign;

// message tags
const SUM_TAG: u8 = 100;
const UPDATE_TAG: u8 = 101;
const SUM2_TAG: u8 = 102;

// message buffer bytes
const SIGNATURE_BYTES: usize = sign::SIGNATUREBYTES;
const TAG_BYTES: usize = 1;
const PK_BYTES: usize = sign::PUBLICKEYBYTES;
const LEN_BYTES: usize = mem::size_of::<usize>();

#[derive(Debug, PartialEq)]
/// A dummy type that represents a certificate.
pub struct Certificate(Vec<u8>);

impl Certificate {
    /// Get the length of the certificate.
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl AsRef<[u8]> for Certificate {
    /// Get a reference to the certificate.
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for Certificate {
    /// Create a certificate from bytes.
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for Certificate {
    /// Create a certificate from a slice of bytes.
    fn from(slice: &[u8]) -> Self {
        Self(slice.to_vec())
    }
}

/// Access to common message buffer fields.
trait MessageBuffer: Sized {
    /// Get a reference to the message buffer.
    fn bytes(&'_ self) -> &'_ [u8];

    /// Get a mutable reference to the message buffer.
    fn bytes_mut(&mut self) -> &mut [u8];

    /// Get the length of the message buffer.
    fn len(&self) -> usize {
        self.bytes().len()
    }

    /// Get the range of the signature field.
    fn signature_range(&self) -> RangeTo<usize> {
        ..SIGNATURE_BYTES
    }

    /// Get a reference to the signature field.
    fn signature(&'_ self) -> &'_ [u8] {
        let range = self.signature_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the signature.
    fn signature_mut(&mut self) -> &mut [u8] {
        let range = self.signature_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the range of the message field.
    fn message_range(&self) -> RangeFrom<usize> {
        self.signature_range().end..
    }

    /// Get a reference to the message field.
    fn message(&'_ self) -> &'_ [u8] {
        let range = self.message_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the message field.
    fn message_mut(&mut self) -> &mut [u8] {
        let range = self.message_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the range of the tag field.
    fn tag_range(&self) -> Range<usize> {
        self.signature_range().end..self.signature_range().end + TAG_BYTES
    }

    /// Get a reference to the tag field.
    fn tag(&'_ self) -> &'_ [u8] {
        let range = self.tag_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the tag field.
    fn tag_mut(&mut self) -> &mut [u8] {
        let range = self.tag_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the range of the coordinator public key field.
    fn coord_pk_range(&self) -> Range<usize> {
        self.tag_range().end..self.tag_range().end + PK_BYTES
    }

    /// Get a reference to the coordinator public key field.
    fn coord_pk(&'_ self) -> &'_ [u8] {
        let range = self.coord_pk_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the coordinator public key field.
    fn coord_pk_mut(&mut self) -> &mut [u8] {
        let range = self.coord_pk_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the range of the participant public key field.
    fn part_pk_range(&self) -> Range<usize> {
        self.coord_pk_range().end..self.coord_pk_range().end + PK_BYTES
    }

    /// Get a reference to the participant public key field.
    fn part_pk(&'_ self) -> &'_ [u8] {
        let range = self.part_pk_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the participant public key field.
    fn part_pk_mut(&mut self) -> &mut [u8] {
        let range = self.part_pk_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the range of the certificate length field.
    fn certificate_len_range(&self) -> Range<usize> {
        self.part_pk_range().end..self.part_pk_range().end + LEN_BYTES
    }

    /// Get a reference to the certificate length field.
    fn certificate_len(&'_ self) -> &'_ [u8] {
        let range = self.certificate_len_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the certificate length field.
    fn certificate_len_mut(&mut self) -> &mut [u8] {
        let range = self.certificate_len_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the number of bytes of the certificate field.
    fn certificate_bytes(&self) -> usize {
        // safe unwrap: length of slice is guaranteed by constants
        usize::from_le_bytes(self.certificate_len().try_into().unwrap())
    }

    /// Get the range of the certificate field.
    fn certificate_range(&self) -> Range<usize> {
        self.certificate_len_range().end
            ..self.certificate_len_range().end + self.certificate_bytes()
    }

    /// Get a reference to the certificate field.
    fn certificate(&'_ self) -> &'_ [u8] {
        let range = self.certificate_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the certificate field of.
    fn certificate_mut(&mut self) -> &mut [u8] {
        let range = self.certificate_range();
        &mut self.bytes_mut()[range]
    }

    /// Get the range of the sum signature field.
    fn sum_signature_range(&self) -> Range<usize> {
        self.certificate_range().end..self.certificate_range().end + SIGNATURE_BYTES
    }

    /// Get a reference to the sum signature field.
    fn sum_signature(&'_ self) -> &'_ [u8] {
        let range = self.sum_signature_range();
        &self.bytes()[range]
    }

    /// Get a mutable reference to the sum signature field.
    fn sum_signature_mut(&mut self) -> &mut [u8] {
        let range = self.sum_signature_range();
        &mut self.bytes_mut()[range]
    }
}

#[cfg(test)]
mod tests {
    use sodiumoxide::{crypto::box_, randombytes::randombytes};

    use super::*;

    struct TestMessageBuffer<B> {
        bytes: B,
    }

    impl<B: AsRef<[u8]> + AsMut<[u8]>> MessageBuffer for TestMessageBuffer<B> {
        fn bytes<'b>(&'b self) -> &'b [u8] {
            self.bytes.as_ref()
        }

        fn bytes_mut(&mut self) -> &mut [u8] {
            self.bytes.as_mut()
        }
    }

    fn auxiliary_bytes() -> Vec<u8> {
        [
            randombytes(129).as_slice(),
            &(0 as usize).to_le_bytes(),
            randombytes(64).as_slice(),
        ]
        .concat()
    }

    #[test]
    fn test_messagebuffer_ranges() {
        // constants
        assert_eq!(SIGNATURE_BYTES, sign::SIGNATUREBYTES);
        assert_eq!(TAG_BYTES, 1);
        assert_eq!(PK_BYTES, box_::PUBLICKEYBYTES);
        assert_eq!(PK_BYTES, sign::PUBLICKEYBYTES);
        assert_eq!(LEN_BYTES, LEN_BYTES);

        // ranges
        let bytes = auxiliary_bytes();
        let buffer = TestMessageBuffer { bytes };
        assert_eq!(buffer.signature_range(), ..64);
        assert_eq!(buffer.message_range(), 64..);
        assert_eq!(buffer.tag_range(), 64..65);
        assert_eq!(buffer.coord_pk_range(), 65..97);
        assert_eq!(buffer.part_pk_range(), 97..129);
        assert_eq!(buffer.certificate_len_range(), 129..129 + LEN_BYTES);
        assert_eq!(buffer.certificate_range(), 129 + LEN_BYTES..129 + LEN_BYTES);
        assert_eq!(
            buffer.sum_signature_range(),
            129 + LEN_BYTES..193 + LEN_BYTES,
        );
    }

    #[test]
    fn test_messagebuffer_fields() {
        let mut bytes = auxiliary_bytes();
        let mut buffer = TestMessageBuffer {
            bytes: bytes.clone(),
        };

        // bytes
        assert_eq!(buffer.bytes(), bytes.as_slice());
        assert_eq!(buffer.bytes_mut(), bytes.as_mut_slice());

        // len
        assert_eq!(buffer.len(), 201);

        // signature
        let range = buffer.signature_range();
        assert_eq!(buffer.signature(), &bytes[range.clone()]);
        assert_eq!(buffer.signature_mut(), &mut bytes[range]);

        // message
        let range = buffer.message_range();
        assert_eq!(buffer.message(), &bytes[range.clone()]);
        assert_eq!(buffer.message_mut(), &mut bytes[range]);

        // tag
        let range = buffer.tag_range();
        assert_eq!(buffer.tag(), &bytes[range.clone()]);
        assert_eq!(buffer.tag_mut(), &mut bytes[range]);

        // coordinator pk
        let range = buffer.coord_pk_range();
        assert_eq!(buffer.coord_pk(), &bytes[range.clone()]);
        assert_eq!(buffer.coord_pk_mut(), &mut bytes[range]);

        // participant pk
        let range = buffer.part_pk_range();
        assert_eq!(buffer.part_pk(), &bytes[range.clone()]);
        assert_eq!(buffer.part_pk_mut(), &mut bytes[range]);

        // certificate length
        let range = buffer.certificate_len_range();
        assert_eq!(buffer.certificate_len(), &bytes[range.clone()]);
        assert_eq!(buffer.certificate_len_mut(), &mut bytes[range]);
        assert_eq!(buffer.certificate_bytes(), 0);

        // certificate
        let range = buffer.certificate_range();
        assert_eq!(buffer.certificate(), &bytes[range.clone()]);
        assert_eq!(buffer.certificate_mut(), &mut bytes[range]);

        // sum signature
        let range = buffer.sum_signature_range();
        assert_eq!(buffer.sum_signature(), &bytes[range.clone()]);
        assert_eq!(buffer.sum_signature_mut(), &mut bytes[range]);
    }
}
