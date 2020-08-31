//! Message buffers.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: ../index.html

use std::ops::{Range, RangeFrom};

use anyhow::{anyhow, Context};

use crate::{
    crypto::ByteObject,
    message::{utils::range, DecodeError, Flags},
    ParticipantPublicKey,
};

// We currently only use 2 bits for the tag, so that byte could also
// be used for something else in the future.
const TAG_RANGE: usize = 0;
// Currently we only don't have any flag
const FLAGS_RANGE: usize = 1;
// Reserve the remaining 2 bytes for future use. That also allows us
// to have 4 bytes alignment.
const RESERVED: Range<usize> = range(2, 2);
const PARTICIPANT_PK_RANGE: Range<usize> = range(RESERVED.end, ParticipantPublicKey::LENGTH);
pub(crate) const HEADER_LENGTH: usize = PARTICIPANT_PK_RANGE.end;

/// A wrapper around a buffer that contains a [`Message`].
///
/// It provides getters and setters to access the different fields of the message safely.
///
/// # Examples
/// ## Reading a sum message
///
/// ```rust
/// use xaynet_core::message::{Tag, MessageBuffer};
/// use std::convert::TryFrom;
/// let mut bytes = vec![
///     0x01, // tag = 1
///     0x00, // flags = 0
///     0x00, 0x00, // reserved bytes, which are ignored
/// ];
/// bytes.extend(vec![0xbb; 32]); // participant public key
/// // Payload: a sum message contains a signature and an ephemeral public key
/// bytes.extend(vec![0x11; 32]); // signature
/// bytes.extend(vec![0x22; 32]); // public key
///
/// let buffer = MessageBuffer::new(&bytes).unwrap();
/// assert_eq!(Tag::try_from(buffer.tag()).unwrap(), Tag::Sum);
/// assert_eq!(buffer.flags(), 0);
/// assert_eq!(buffer.participant_pk(), vec![0xbb; 32].as_slice());
/// assert_eq!(buffer.payload(), [vec![0x11; 32], vec![0x22; 32]].concat().as_slice());
/// ```
///
/// ## Writing a sum message
///
/// ```rust
/// use xaynet_core::message::{Tag, MessageBuffer};
/// use std::convert::TryFrom;
/// let mut expected = vec![
///     0x01, // tag = 1
///     0x00, // flags = 0
///     0x00, 0x00, // reserved bytes, which are ignored
/// ];
/// expected.extend(vec![0xbb; 32]); // participant public key
/// // Payload: a sum message contains a signature and an ephemeral public key
/// expected.extend(vec![0x11; 32]); // signature
/// expected.extend(vec![0x22; 32]); // public key
///
/// let mut bytes = vec![0; expected.len()];
/// let mut buffer = MessageBuffer::new_unchecked(&mut bytes);
/// buffer.set_tag(Tag::Sum.into());
/// buffer.set_flags(0);
/// buffer
///     .participant_pk_mut()
///     .copy_from_slice(vec![0xbb; 32].as_slice());
/// buffer
///     .payload_mut()
///     .copy_from_slice([vec![0x11; 32], vec![0x22; 32]].concat().as_slice());
/// assert_eq!(expected, bytes);
/// ```
///
/// [`Message`]: struct.Message.html
pub struct MessageBuffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> MessageBuffer<T> {
    /// Performs bound checks for the various message fields on `bytes` and returns a new
    /// [`MessageBuffer`].
    ///
    /// # Errors
    /// Fails if the `bytes` are smaller than a minimal-sized message buffer.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid MessageBuffer")?;
        Ok(buffer)
    }

    /// Returns a [`MessageBuffer`] without performing any bound checks.
    ///
    /// This means accessing the various fields may panic if the data is invalid.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Performs bound checks to ensure the fields can be accessed without panicking.
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

        Ok(())
    }

    /// Computes the payload range.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    fn payload_range(&self) -> RangeFrom<usize> {
        PARTICIPANT_PK_RANGE.end..
    }

    /// Gets the tag field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn tag(&self) -> u8 {
        self.inner.as_ref()[TAG_RANGE]
    }

    /// Gets the flags field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn flags(&self) -> Flags {
        self.inner.as_ref()[FLAGS_RANGE]
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> MessageBuffer<&'a T> {
    /// Gets the participant public key field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn participant_pk(&self) -> &'a [u8] {
        &self.inner.as_ref()[PARTICIPANT_PK_RANGE]
    }

    /// Gets the rest of the message.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn payload(&self) -> &'a [u8] {
        &self.inner.as_ref()[self.payload_range()]
    }
}

impl<T: AsMut<[u8]> + AsRef<[u8]>> MessageBuffer<T> {
    /// Sets the tag field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn set_tag(&mut self, value: u8) {
        self.inner.as_mut()[TAG_RANGE] = value;
    }

    /// Sets the flags field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn set_flags(&mut self, value: Flags) {
        self.inner.as_mut()[FLAGS_RANGE] = value;
    }

    /// Gets a mutable reference to the participant public key field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn participant_pk_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[PARTICIPANT_PK_RANGE]
    }

    /// Gets a mutable reference to the rest of the message.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let range = self.payload_range();
        &mut self.inner.as_mut()[range]
    }
}

#[cfg(test)]
pub(in crate::message) mod tests {
    use std::convert::TryFrom;

    use super::*;
    use crate::{
        crypto::ByteObject,
        message::{
            header::{Header, Tag},
            message::Message,
            payload::sum,
        },
    };

    fn participant_pk() -> (Vec<u8>, ParticipantPublicKey) {
        let bytes = vec![0xbb; 32];
        let pk = ParticipantPublicKey::from_slice(&bytes).unwrap();
        (bytes, pk)
    }

    pub(crate) fn header_bytes(tag: Tag) -> Vec<u8> {
        let mut buf = vec![
            tag.into(),
            // flags
            0x00,
            // reserved bytes, which can be anything
            0xff,
            0xff,
        ];
        buf.extend(participant_pk().0);
        buf
    }

    pub(crate) fn header(tag: Tag) -> Header {
        Header {
            tag,
            participant_pk: participant_pk().1,
        }
    }

    fn sum() -> (Vec<u8>, Message) {
        let mut bytes = header_bytes(Tag::Sum);
        bytes.extend(sum::tests::sum_bytes());

        let header = header(Tag::Sum);
        let payload = sum::tests::sum().into();
        let message = Message { header, payload };
        (bytes, message)
    }

    #[test]
    fn buffer_read_no_cert() {
        let (bytes, _) = sum();
        let buffer = MessageBuffer::new(&bytes).unwrap();
        assert_eq!(Tag::try_from(buffer.tag()).unwrap(), Tag::Sum);
        assert_eq!(buffer.flags(), 0);
        assert_eq!(buffer.participant_pk(), participant_pk().0.as_slice());
    }

    #[test]
    fn buffer_read_with_cert() {
        let (bytes, _) = sum();
        let buffer = MessageBuffer::new(&bytes).unwrap();
        assert_eq!(Tag::try_from(buffer.tag()).unwrap(), Tag::Sum);
        assert_eq!(buffer.flags(), 0);
        assert_eq!(buffer.participant_pk(), participant_pk().0.as_slice());
    }

    #[test]
    fn buffer_write_no_cert() {
        let expected = sum().0;
        let mut bytes = vec![0xff; expected.len()];
        let mut buffer = MessageBuffer::new_unchecked(&mut bytes);

        buffer.set_tag(Tag::Sum.into());
        buffer.set_flags(0);
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
        let expected = sum().0;
        let mut bytes = vec![0xff; expected.len()];
        let mut buffer = MessageBuffer::new_unchecked(&mut bytes);

        buffer.set_tag(Tag::Sum.into());
        buffer.set_flags(0);
        buffer
            .participant_pk_mut()
            .copy_from_slice(participant_pk().0.as_slice());
        buffer
            .payload_mut()
            .copy_from_slice(sum::tests::sum_bytes().as_slice());
        assert_eq!(bytes, expected);
    }
}
