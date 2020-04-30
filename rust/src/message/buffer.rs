use anyhow::{anyhow, Context};
use std::ops::{Range, RangeFrom};

use crate::{
    message::{utils::range, DecodeError, Flags, LengthValueBuffer},
    CoordinatorPublicKey,
    ParticipantPublicKey,
};

pub(crate) fn header_length(certificate_length: usize) -> usize {
    certificate_length + PARTICIPANT_PK_RANGE.end
}

// We currently only use 2 bits for the tag, so that byte could also
// be used for something else in the future.
const TAG_RANGE: usize = 0;
// Currently we only have one flag to indicate the presence of a
// certificate.
const FLAGS_RANGE: usize = 1;
// Reserve the remaining 2 bytes for future use. That also allows us
// to have 4 bytes alignment.
const RESERVED: Range<usize> = range(2, 2);
const COORDINATOR_PK_RANGE: Range<usize> = range(RESERVED.end, CoordinatorPublicKey::LENGTH);
const PARTICIPANT_PK_RANGE: Range<usize> =
    range(COORDINATOR_PK_RANGE.end, ParticipantPublicKey::LENGTH);

/// A wrapper around a buffer that contains a message. It provides
/// getters and setters to access the different fields of the message
/// safely.
///
/// # Examples
///
/// Reading a sum message:
///
/// ```rust
/// use xain_fl::message::{Tag, Flags, MessageBuffer};
/// use std::convert::TryFrom;
/// let mut bytes = vec![
///     0x01, // tag = 1
///     0x00, // flags = 0
///     0x00, 0x00, // reserved bytes, which are ignored
/// ];
/// bytes.extend(vec![0xaa; 32]); // coordinator public key
/// bytes.extend(vec![0xbb; 32]); // participant public key
/// // Payload: a sum message contains a signature and an ephemeral public key
/// bytes.extend(vec![0x11; 32]); // signature
/// bytes.extend(vec![0x22; 32]); // public key
///
/// let buffer = MessageBuffer::new(&bytes).unwrap();
/// assert!(!buffer.has_certificate());
/// assert_eq!(Tag::try_from(buffer.tag()).unwrap(), Tag::Sum);
/// assert_eq!(buffer.flags(), Flags::empty());
/// assert!(buffer.certificate().is_none());
/// assert_eq!(buffer.coordinator_pk(), vec![0xaa; 32].as_slice());
/// assert_eq!(buffer.participant_pk(), vec![0xbb; 32].as_slice());
/// assert_eq!(buffer.payload(), [vec![0x11; 32], vec![0x22; 32]].concat().as_slice());
/// ```
///
/// Writing a sum message:
///
/// ```rust
/// use xain_fl::message::{Tag, Flags, MessageBuffer};
/// use std::convert::TryFrom;
/// let mut expected = vec![
///     0x01, // tag = 1
///     0x00, // flags = 0
///     0x00, 0x00, // reserved bytes, which are ignored
/// ];
/// expected.extend(vec![0xaa; 32]); // coordinator public key
/// expected.extend(vec![0xbb; 32]); // participant public key
/// // Payload: a sum message contains a signature and an ephemeral public key
/// expected.extend(vec![0x11; 32]); // signature
/// expected.extend(vec![0x22; 32]); // public key
///
/// let mut bytes = vec![0; expected.len()];
/// let mut buffer = MessageBuffer::new_unchecked(&mut bytes);
/// buffer.set_tag(Tag::Sum.into());
/// buffer.set_flags(Flags::empty());
/// buffer
///     .coordinator_pk_mut()
///     .copy_from_slice(vec![0xaa; 32].as_slice());
/// buffer
///     .participant_pk_mut()
///     .copy_from_slice(vec![0xbb; 32].as_slice());
/// buffer
///     .payload_mut()
///     .copy_from_slice([vec![0x11; 32], vec![0x22; 32]].concat().as_slice());
/// assert_eq!(expected, bytes);
/// ```
pub struct MessageBuffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> MessageBuffer<T> {
    /// Perform bound checks for the various message fields on `bytes`
    /// and return a new `MessageBuffer`.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid MessageBuffer")?;
        Ok(buffer)
    }

    /// Return a `MessageBuffer` without performing any bound
    /// check. This means accessing the various fields may panic if
    /// the data is invalid.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Perform bound checks to ensure the fields can be accessed
    /// without panicking.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        // First, check the fixed size portion of the
        // header. PARTICIPANT_PK_RANGE is the last field
        if len < PARTICIPANT_PK_RANGE.end {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                PARTICIPANT_PK_RANGE.end
            ));
        }

        // Check if the header contains a certificate, and if it does,
        // check the length of certificate field.
        if self.has_certificate() {
            let bytes = &self.inner.as_ref()[PARTICIPANT_PK_RANGE.end..];
            let _ =
                LengthValueBuffer::new(bytes).context("certificate field has an invalid lenth")?;
        }

        Ok(())
    }

    /// Return whether this header contains a certificate
    pub fn has_certificate(&self) -> bool {
        self.flags().contains(Flags::CERTIFICATE)
    }

    fn payload_range(&self) -> RangeFrom<usize> {
        let certificate_length = self
            .has_certificate()
            .then(|| {
                let bytes = &self.inner.as_ref()[PARTICIPANT_PK_RANGE.end..];
                LengthValueBuffer::new(bytes).unwrap().length() as usize
            })
            .unwrap_or(0);
        let payload_start = PARTICIPANT_PK_RANGE.end + certificate_length;
        payload_start..
    }

    /// Get the tag field
    pub fn tag(&self) -> u8 {
        self.inner.as_ref()[TAG_RANGE]
    }

    /// Get the flags field
    pub fn flags(&self) -> Flags {
        Flags::from_bits_truncate(self.inner.as_ref()[FLAGS_RANGE])
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> MessageBuffer<&'a T> {
    /// Get a slice to the certificate. If the header doesn't contain
    /// any certificate, `None` is returned.
    pub fn certificate(&self) -> Option<LengthValueBuffer<&'a [u8]>> {
        self.has_certificate().then(|| {
            let bytes = &self.inner.as_ref()[PARTICIPANT_PK_RANGE.end..];
            LengthValueBuffer::new_unchecked(bytes)
        })
    }

    /// Get the coordinator public key field
    pub fn coordinator_pk(&self) -> &'a [u8] {
        &self.inner.as_ref()[COORDINATOR_PK_RANGE]
    }

    /// Get the participant public key field
    pub fn participant_pk(&self) -> &'a [u8] {
        &self.inner.as_ref()[PARTICIPANT_PK_RANGE]
    }

    /// Get the rest of the message
    pub fn payload(&self) -> &'a [u8] {
        &self.inner.as_ref()[self.payload_range()]
    }
}

impl<T: AsMut<[u8]> + AsRef<[u8]>> MessageBuffer<T> {
    /// Set the tag field
    pub fn set_tag(&mut self, value: u8) {
        self.inner.as_mut()[TAG_RANGE] = value;
    }

    /// Set the flags field
    pub fn set_flags(&mut self, value: Flags) {
        self.inner.as_mut()[FLAGS_RANGE] = value.bits();
    }

    /// Get a mutable reference to the certificate field
    pub fn certificate_mut(&mut self) -> Option<LengthValueBuffer<&mut [u8]>> {
        if self.has_certificate() {
            let bytes = &mut self.inner.as_mut()[PARTICIPANT_PK_RANGE.end..];
            Some(LengthValueBuffer::new_unchecked(bytes))
        } else {
            None
        }
    }
    /// Get a mutable reference to the coordinator public key field
    pub fn coordinator_pk_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[COORDINATOR_PK_RANGE]
    }

    /// Get a mutable reference to the participant public key field
    pub fn participant_pk_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[PARTICIPANT_PK_RANGE]
    }

    /// Get a mutable reference to the rest of the message
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let range = self.payload_range();
        &mut self.inner.as_mut()[range]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        certificate::Certificate,
        crypto::ByteObject,
        message::{sum, HeaderOwned, MessageOwned, Tag},
    };
    use std::convert::TryFrom;

    fn coordinator_pk() -> (Vec<u8>, CoordinatorPublicKey) {
        let bytes = vec![0xaa; 32];
        let pk = CoordinatorPublicKey::from_slice(bytes.as_slice()).unwrap();
        (bytes, pk)
    }

    fn participant_pk() -> (Vec<u8>, ParticipantPublicKey) {
        let bytes = vec![0xbb; 32];
        let pk = ParticipantPublicKey::from_slice(&bytes).unwrap();
        (bytes, pk)
    }

    fn certificate() -> (Vec<u8>, Certificate) {
        let bytes = vec![0x01; 32];
        let cert = Certificate::try_from(bytes.as_slice()).unwrap();
        (bytes, cert)
    }

    pub(crate) fn header_bytes(tag: Tag, with_certificate: bool) -> Vec<u8> {
        let mut buf = vec![
            tag.into(),
            // flags
            if with_certificate { 1 } else { 0 },
            // reserved bytes, which can be anything
            0xff,
            0xff,
        ];
        buf.extend(coordinator_pk().0);
        buf.extend(participant_pk().0);
        if with_certificate {
            // certificate length
            buf.extend(vec![0x00, 0x00, 0x00, 32 + 4]);
            buf.extend(certificate().0);
        }
        buf
    }

    pub(crate) fn header(tag: Tag, with_certificate: bool) -> HeaderOwned {
        HeaderOwned {
            tag,
            coordinator_pk: coordinator_pk().1,
            participant_pk: participant_pk().1,
            certificate: if with_certificate {
                Some(certificate().1)
            } else {
                None
            },
        }
    }

    fn sum(with_certificate: bool) -> (Vec<u8>, MessageOwned) {
        let mut bytes = header_bytes(Tag::Sum, with_certificate);
        bytes.extend(sum::tests::sum_bytes());

        let header = header(Tag::Sum, with_certificate);
        let payload = sum::tests::sum().into();
        let message = MessageOwned { header, payload };
        (bytes, message)
    }

    #[test]
    fn buffer_read_no_cert() {
        let (bytes, _) = sum(false);
        let buffer = MessageBuffer::new(&bytes).unwrap();
        assert!(!buffer.has_certificate());
        assert_eq!(Tag::try_from(buffer.tag()).unwrap(), Tag::Sum);
        assert_eq!(buffer.flags(), Flags::empty());
        assert!(buffer.certificate().is_none());
        assert_eq!(buffer.coordinator_pk(), coordinator_pk().0.as_slice());
        assert_eq!(buffer.participant_pk(), participant_pk().0.as_slice());
    }

    #[test]
    fn buffer_read_with_cert() {
        let (bytes, _) = sum(true);
        let buffer = MessageBuffer::new(&bytes).unwrap();
        assert!(buffer.has_certificate());
        assert_eq!(Tag::try_from(buffer.tag()).unwrap(), Tag::Sum);
        assert_eq!(buffer.flags(), Flags::CERTIFICATE);
        assert_eq!(buffer.certificate().unwrap().value(), &certificate().0[..]);
        assert_eq!(buffer.coordinator_pk(), coordinator_pk().0.as_slice());
        assert_eq!(buffer.participant_pk(), participant_pk().0.as_slice());
    }

    #[test]
    fn buffer_write_no_cert() {
        let expected = sum(false).0;
        let mut bytes = vec![0xff; expected.len()];
        let mut buffer = MessageBuffer::new_unchecked(&mut bytes);

        buffer.set_tag(Tag::Sum.into());
        buffer.set_flags(Flags::empty());
        buffer
            .coordinator_pk_mut()
            .copy_from_slice(coordinator_pk().0.as_slice());
        buffer
            .participant_pk_mut()
            .copy_from_slice(participant_pk().0.as_slice());
        buffer
            .payload_mut()
            .copy_from_slice(sum::tests::sum_bytes().as_slice());
        assert_eq!(bytes, expected);
    }

    #[test]
    fn buffer_write_with_cert() {
        let expected = sum(true).0;
        let mut bytes = vec![0xff; expected.len()];
        let mut buffer = MessageBuffer::new_unchecked(&mut bytes);

        buffer.set_tag(Tag::Sum.into());
        buffer.set_flags(Flags::CERTIFICATE);
        buffer
            .coordinator_pk_mut()
            .copy_from_slice(coordinator_pk().0.as_slice());
        buffer
            .participant_pk_mut()
            .copy_from_slice(participant_pk().0.as_slice());
        buffer.certificate_mut().unwrap().set_length(32 + 4);
        buffer
            .certificate_mut()
            .unwrap()
            .value_mut()
            .copy_from_slice(certificate().0.as_slice());
        buffer
            .payload_mut()
            .copy_from_slice(sum::tests::sum_bytes().as_slice());
        assert_eq!(bytes, expected);
    }
}
