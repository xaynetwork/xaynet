pub mod sum;
pub mod sum2;
pub mod update;

use std::ops::Range;

// message tags
const TAG_BYTES: usize = 1;
const SUM_TAG: u8 = 100;
const UPDATE_TAG: u8 = 101;
const SUM2_TAG: u8 = 102;

// common message buffer field ranges
const CERTIFICATE_BYTES: usize = 0;
const SIGNATURE_RANGE: Range<usize> = 0..64; // 64 bytes
const MESSAGE_START: usize = 64;
const TAG_RANGE: Range<usize> = 64..65; // 1 byte
const COORD_PK_RANGE: Range<usize> = 65..97; // 32 bytes
const PART_PK_RANGE: Range<usize> = 97..129; // 32 bytes
const CERTIFICATE_RANGE: Range<usize> = 129..129; // 0 bytes (dummy)
const SUM_SIGNATURE_RANGE: Range<usize> = 129..193; // 64 bytes

#[derive(Debug, PartialEq)]
/// A dummy type that represents a certificate.
pub struct Certificate(Vec<u8>);

impl AsRef<[u8]> for Certificate {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for Certificate {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for Certificate {
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

    /// Get a reference to the signature field of the message buffer.
    fn signature(&'_ self) -> &'_ [u8] {
        &self.bytes()[SIGNATURE_RANGE]
    }

    /// Get a mutable reference to the signature field of the message buffer.
    fn signature_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[SIGNATURE_RANGE]
    }

    /// Get a reference to the message field of the message buffer.
    fn message(&'_ self) -> &'_ [u8] {
        &self.bytes()[MESSAGE_START..]
    }

    /// Get a mutable reference to the message field of the message buffer.
    fn message_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[MESSAGE_START..]
    }

    /// Get a reference to the tag field of the message buffer.
    fn tag(&'_ self) -> &'_ [u8] {
        &self.bytes()[TAG_RANGE]
    }

    /// Get a mutable reference to the tag field of the message buffer.
    fn tag_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[TAG_RANGE]
    }

    /// Get a reference to the coordinator public key field of the message buffer.
    fn coord_pk(&'_ self) -> &'_ [u8] {
        &self.bytes()[COORD_PK_RANGE]
    }

    /// Get a mutable reference to the coordinator public key field of the message buffer.
    fn coord_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[COORD_PK_RANGE]
    }

    /// Get a reference to the participant public key field of the message buffer.
    fn part_pk(&'_ self) -> &'_ [u8] {
        &self.bytes()[PART_PK_RANGE]
    }

    /// Get a mutable reference to the participant public key field of the message buffer.
    fn part_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[PART_PK_RANGE]
    }

    /// Get a reference to the certificate field of the message buffer.
    fn certificate(&'_ self) -> &'_ [u8] {
        &self.bytes()[CERTIFICATE_RANGE]
    }

    /// Get a mutable reference to the certificate field of the message buffer.
    fn certificate_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[CERTIFICATE_RANGE]
    }

    /// Get a reference to the sum signature field of the message buffer.
    fn sum_signature(&'_ self) -> &'_ [u8] {
        &self.bytes()[SUM_SIGNATURE_RANGE]
    }

    /// Get a mutable reference to the sum signature field of the message buffer.
    fn sum_signature_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[SUM_SIGNATURE_RANGE]
    }
}

#[cfg(test)]
mod tests {
    use sodiumoxide::{
        crypto::{box_, sign},
        randombytes::randombytes,
    };

    use super::*;

    #[test]
    fn test_ranges() {
        assert_eq!(
            SIGNATURE_RANGE.end - SIGNATURE_RANGE.start,
            sign::SIGNATUREBYTES,
        );
        assert_eq!(TAG_RANGE.end - TAG_RANGE.start, TAG_BYTES);
        assert_eq!(
            COORD_PK_RANGE.end - COORD_PK_RANGE.start,
            box_::PUBLICKEYBYTES,
        );
        assert_eq!(
            PART_PK_RANGE.end - PART_PK_RANGE.start,
            sign::PUBLICKEYBYTES,
        );
        assert_eq!(
            CERTIFICATE_RANGE.end - CERTIFICATE_RANGE.start,
            CERTIFICATE_BYTES,
        );
        assert_eq!(
            SUM_SIGNATURE_RANGE.end - SUM_SIGNATURE_RANGE.start,
            sign::SIGNATUREBYTES,
        );
    }

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

    #[test]
    fn test_messagebuffer() {
        let mut bytes = randombytes(193);
        let mut buffer = TestMessageBuffer {
            bytes: bytes.clone(),
        };

        // bytes
        assert_eq!(buffer.bytes(), &bytes[..]);
        assert_eq!(buffer.bytes_mut(), &mut bytes[..]);

        // signature
        assert_eq!(buffer.signature(), &bytes[0..64]);
        assert_eq!(buffer.signature_mut(), &mut bytes[0..64]);

        // message
        assert_eq!(buffer.message(), &bytes[64..]);
        assert_eq!(buffer.message_mut(), &mut bytes[64..]);

        // tag
        assert_eq!(buffer.tag(), &bytes[64..65]);
        assert_eq!(buffer.tag_mut(), &mut bytes[64..65]);

        // coordinator pk
        assert_eq!(buffer.coord_pk(), &bytes[65..97]);
        assert_eq!(buffer.coord_pk_mut(), &mut bytes[65..97]);

        // participant pk
        assert_eq!(buffer.part_pk(), &bytes[97..129]);
        assert_eq!(buffer.part_pk_mut(), &mut bytes[97..129]);

        // certificate
        assert_eq!(buffer.certificate(), &bytes[129..129]);
        assert_eq!(buffer.certificate_mut(), &mut bytes[129..129]);

        // sum signature
        assert_eq!(buffer.sum_signature(), &bytes[129..193]);
        assert_eq!(buffer.sum_signature_mut(), &mut bytes[129..193]);
    }
}
