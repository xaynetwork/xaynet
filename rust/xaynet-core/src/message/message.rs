//! Message buffers.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: crate::mask

use std::convert::{TryFrom, TryInto};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

use crate::{
    crypto::{ByteObject, PublicEncryptKey, PublicSigningKey, SecretSigningKey, Signature},
    message::{Chunk, DecodeError, FromBytes, Payload, Sum, Sum2, ToBytes, Update},
};

/// The minimum number of accepted `sum`/`sum2` messages for the PET protocol to function correctly.
pub const SUM_COUNT_MIN: u64 = 1;

/// The minimum number of accepted `update` messages for the PET protocol to function correctly.
pub const UPDATE_COUNT_MIN: u64 = 3;

pub(crate) mod ranges {
    use std::ops::Range;

    use super::*;
    use crate::message::utils::range;

    /// Byte range corresponding to the signature in a message in a
    /// message header
    pub const SIGNATURE: Range<usize> = range(0, Signature::LENGTH);
    /// Byte range corresponding to the participant public key in a
    /// message header
    pub const PARTICIPANT_PK: Range<usize> = range(SIGNATURE.end, PublicSigningKey::LENGTH);
    /// Byte range corresponding to the coordinator public key in a
    /// message header
    pub const COORDINATOR_PK: Range<usize> = range(PARTICIPANT_PK.end, PublicEncryptKey::LENGTH);
    /// Byte range corresponding to the length field in a message header
    pub const LENGTH: Range<usize> = range(COORDINATOR_PK.end, 4);
    /// Byte range corresponding to the tag in a message header
    pub const TAG: usize = LENGTH.end;
    /// Byte range corresponding to the flags in a message header
    pub const FLAGS: usize = TAG + 1;
    /// Byte range reserved for future use
    pub const RESERVED: Range<usize> = range(FLAGS + 1, 2);
}

/// Length in bytes of a message header
pub const HEADER_LENGTH: usize = ranges::RESERVED.end;

/// A wrapper around a buffer that contains a [`Message`].
///
/// It provides getters and setters to access the different fields of
/// the message safely. A message is made of a header and a payload:
///
/// ```no_rust
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                           signature                           +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                         participant_pk                        +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                         coordinator_pk                        +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +                                                               +
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                             length                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |      tag      |     flags     |          reserved             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// +                    payload (variable length)                  +
/// |                                                               |
/// ```
///
/// - `signature` contains the signature of the entire message
/// - `participant_pk` contains the public key for verifying the
///   signature
/// - `coordinator_pk` is the coordinator public encryption key. It is
///    embedded in the message for security reasons. See [_Donald
///    T. Davis, "Defective Sign & Encrypt in S/MIME, PKCS#7, MOSS,
///    PEM, PGP, and XML.", Proc. Usenix Tech. Conf. 2001 (Boston,
///    Mass., June 25-30,
///    2001)_](http://world.std.com/~dtd/sign_encrypt/sign_encrypt7.html)
/// - `length` is the length in bytes of the _full_ message, _i.e._
///   including the header. This is a 32 bits field so in theory,
///   messages can be as big as 2^32 = 4,294,967,296 bytes.
/// - `tag` indicates the type of message (sum, update, sum2 or
///   multipart message)
/// - the `flags` field currently supports a single flag, that
///   indicates whether this is a multipart message
///
/// # Examples
/// ## Reading a sum message
///
/// ```rust
/// use std::convert::TryFrom;
/// use xaynet_core::message::{Flags, MessageBuffer, Tag};
///
/// let mut bytes = vec![0x11; 64]; // message signature
/// bytes.extend(vec![0x22; 32]); // participant public signing key
/// bytes.extend(vec![0x33; 32]); // coordinator public encrypt key
/// bytes.extend(&200_u32.to_be_bytes()); // Length field
/// bytes.push(0x01); // tag (sum message)
/// bytes.push(0x00); // flags (not a multipart message)
/// bytes.extend(vec![0x00, 0x00]); // reserved
///
/// // Payload: a sum message contains a signature and an ephemeral public key
/// bytes.extend(vec![0xaa; 32]); // signature
/// bytes.extend(vec![0xbb; 32]); // public key
///
/// let buffer = MessageBuffer::new(&bytes).unwrap();
/// assert_eq!(buffer.signature(), vec![0x11; 64].as_slice());
/// assert_eq!(buffer.participant_pk(), vec![0x22; 32].as_slice());
/// assert_eq!(buffer.coordinator_pk(), vec![0x33; 32].as_slice());
/// assert_eq!(Tag::try_from(buffer.tag()).unwrap(), Tag::Sum);
/// assert_eq!(Flags::try_from(buffer.flags()).unwrap(), Flags::empty());
/// assert_eq!(
///     buffer.payload(),
///     [vec![0xaa; 32], vec![0xbb; 32]].concat().as_slice()
/// );
/// ```
///
/// ## Writing a sum message
///
/// ```rust
/// use std::convert::TryFrom;
/// use xaynet_core::message::{Flags, MessageBuffer, Tag};
///
/// let mut expected = vec![0x11; 64]; // message signature
/// expected.extend(vec![0x22; 32]); // participant public signing key
/// expected.extend(vec![0x33; 32]); // coordinator public signing key
/// expected.extend(&200_u32.to_be_bytes()); // length field
/// expected.push(0x01); // tag (sum message)
/// expected.push(0x00); // flags (not a multipart message)
/// expected.extend(vec![0x00, 0x00]); // reserved
///
/// // Payload: a sum message contains a signature and an ephemeral public key
/// expected.extend(vec![0xaa; 32]); // signature
/// expected.extend(vec![0xbb; 32]); // public key
///
/// let mut bytes = vec![0; expected.len()];
/// let mut buffer = MessageBuffer::new_unchecked(&mut bytes);
/// buffer
///     .signature_mut()
///     .copy_from_slice(vec![0x11; 64].as_slice());
/// buffer
///     .participant_pk_mut()
///     .copy_from_slice(vec![0x22; 32].as_slice());
/// buffer
///     .coordinator_pk_mut()
///     .copy_from_slice(vec![0x33; 32].as_slice());
/// buffer.set_length(200 as u32);
/// buffer.set_tag(Tag::Sum.into());
/// buffer.set_flags(Flags::empty());
/// buffer
///     .payload_mut()
///     .copy_from_slice([vec![0xaa; 32], vec![0xbb; 32]].concat().as_slice());
/// assert_eq!(expected, bytes);
/// ```
pub struct MessageBuffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> MessageBuffer<T> {
    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn as_ref(&self) -> MessageBuffer<&T> {
        MessageBuffer::new_unchecked(self.inner())
    }
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
    /// This means accessing the various fields may panic if the data
    /// is invalid.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Performs bound checks to ensure the fields can be accessed
    /// without panicking.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        if len < HEADER_LENGTH {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                HEADER_LENGTH
            ));
        }
        let expected_len = self.length() as usize;
        let actual_len = self.inner.as_ref().len();
        if actual_len < expected_len {
            return Err(anyhow!(
                "invalid message length: length field says {}, but buffer is {} bytes long",
                expected_len,
                actual_len
            ));
        }
        Ok(())
    }

    /// Gets the tag field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn tag(&self) -> u8 {
        self.inner.as_ref()[ranges::TAG]
    }

    /// Gets the flags field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn flags(&self) -> Flags {
        Flags::from_bits_truncate(self.inner.as_ref()[ranges::FLAGS])
    }

    /// Gets the length field
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn length(&self) -> u32 {
        // Unwrapping is OK, as the slice is guaranteed to be 4 bytes
        // long
        u32::from_be_bytes(self.inner.as_ref()[ranges::LENGTH].try_into().unwrap())
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> MessageBuffer<&'a T> {
    /// Gets the message signature field
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn signature(&self) -> &'a [u8] {
        &self.inner.as_ref()[ranges::SIGNATURE]
    }

    /// Gets the participant public key field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn participant_pk(&self) -> &'a [u8] {
        &self.inner.as_ref()[ranges::PARTICIPANT_PK]
    }

    /// Gets the coordinator public key field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn coordinator_pk(&self) -> &'a [u8] {
        &self.inner.as_ref()[ranges::COORDINATOR_PK]
    }

    /// Gets the rest of the message.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn payload(&self) -> &'a [u8] {
        &self.inner.as_ref()[HEADER_LENGTH..]
    }

    /// Parse the signature and public signing key, and check the
    /// message signature.
    pub fn check_signature(&self) -> Result<(), DecodeError> {
        let signature = Signature::from_byte_slice(&self.signature())
            .context("cannot parse the signature field")?;
        let participant_pk = PublicSigningKey::from_byte_slice(&self.participant_pk())
            .context("cannot part the public key field")?;

        if participant_pk.verify_detached(&signature, self.signed_data()) {
            Ok(())
        } else {
            Err(anyhow!("invalid message signature"))
        }
    }

    /// Return the portion of the message used to compute the
    /// signature, ie the entire message except the signature field
    /// itself.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn signed_data(&self) -> &'a [u8] {
        let signed_data_range = ranges::SIGNATURE.end..self.length() as usize;
        &self.inner.as_ref()[signed_data_range]
    }
}

impl<T: AsMut<[u8]> + AsRef<[u8]>> MessageBuffer<T> {
    /// Sets the tag field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn set_tag(&mut self, value: u8) {
        self.inner.as_mut()[ranges::TAG] = value;
    }

    /// Sets the flags field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn set_flags(&mut self, value: Flags) {
        self.inner.as_mut()[ranges::FLAGS] = value.bits();
    }

    /// Sets the length field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn set_length(&mut self, value: u32) {
        let bytes = value.to_be_bytes();
        self.inner.as_mut()[ranges::LENGTH].copy_from_slice(&bytes[..]);
    }

    /// Gets a mutable reference to the message signature field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn signature_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[ranges::SIGNATURE]
    }

    /// Gets a mutable reference to the participant public key field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn participant_pk_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[ranges::PARTICIPANT_PK]
    }

    /// Gets a mutable reference to the coordinator public key field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn coordinator_pk_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[ranges::COORDINATOR_PK]
    }

    /// Gets a mutable reference to the rest of the message.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[HEADER_LENGTH..]
    }

    /// Gets a mutable reference to the portion of the message used to
    /// compute the signature, ie the entire message except the
    /// signature field itself.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn signed_data_mut(&mut self) -> &mut [u8] {
        let signed_data_range = ranges::SIGNATURE.end..self.length() as usize;
        &mut self.inner.as_mut()[signed_data_range]
    }
}

bitflags::bitflags! {
    /// A bitmask that defines flags for a [`Message`].
    pub struct Flags: u8 {
        /// Indicates whether this message is a multipart message
        const MULTIPART = 1 << 0;
    }
}

#[derive(Copy, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
/// A tag that indicates the type of the [`Message`].
pub enum Tag {
    /// A tag for [`Sum`] messages
    Sum,
    /// A tag for [`Update`] messages
    Update,
    /// A tag for [`Sum2`] messages
    Sum2,
}

impl TryFrom<u8> for Tag {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Tag::Sum,
            2 => Tag::Update,
            3 => Tag::Sum2,
            _ => return Err(anyhow!("invalid tag {}", value)),
        })
    }
}

impl From<Tag> for u8 {
    fn from(tag: Tag) -> Self {
        match tag {
            Tag::Sum => 1,
            Tag::Update => 2,
            Tag::Sum2 => 3,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
/// A header common to all messages.
pub struct Message {
    /// Message signature. This can be `None` if it hasn't been
    /// computed yet.
    pub signature: Option<Signature>,
    /// The participant public key, used to verify the message
    /// signature.
    pub participant_pk: PublicSigningKey,
    /// The coordinator public key
    pub coordinator_pk: PublicEncryptKey,
    /// Wether this is a multipart message
    pub is_multipart: bool,
    /// The type of message. This information is partially redundant
    /// with the `payload` field. So when serializing the message,
    /// this field is ignored if the payload is a [`Payload::Sum`],
    /// [`Payload::Update`], or [`Payload::Sum2`]. However, it is
    /// taken as is for [`Payload::Chunk`].
    pub tag: Tag,
    /// Message payload
    pub payload: Payload,
}

impl Message {
    /// Create a new sum message with the given participant and
    /// coordinator public keys.
    pub fn new_sum(
        participant_pk: PublicSigningKey,
        coordinator_pk: PublicEncryptKey,
        message: Sum,
    ) -> Self {
        Self {
            signature: None,
            participant_pk,
            coordinator_pk,
            is_multipart: false,
            tag: Tag::Sum,
            payload: message.into(),
        }
    }

    /// Create a new sum2 message with the given participant and
    /// coordinator public keys.
    pub fn new_sum2(
        participant_pk: PublicSigningKey,
        coordinator_pk: PublicEncryptKey,
        message: Sum2,
    ) -> Self {
        Self {
            signature: None,
            participant_pk,
            coordinator_pk,
            is_multipart: false,
            tag: Tag::Sum2,
            payload: message.into(),
        }
    }

    /// Create a new update message with the given participant and
    /// coordinator public keys.
    pub fn new_update(
        participant_pk: PublicSigningKey,
        coordinator_pk: PublicEncryptKey,
        message: Update,
    ) -> Self {
        Self {
            signature: None,
            participant_pk,
            coordinator_pk,
            is_multipart: false,
            tag: Tag::Update,
            payload: message.into(),
        }
    }

    /// Create a new multipart message with the given participant and
    /// coordinator public keys.
    pub fn new_multipart(
        participant_pk: PublicSigningKey,
        coordinator_pk: PublicEncryptKey,
        message: Chunk,
        tag: Tag,
    ) -> Self {
        Self {
            signature: None,
            participant_pk,
            coordinator_pk,
            is_multipart: true,
            tag,
            payload: message.into(),
        }
    }

    /// Parse the given message **without** verifying the
    /// signature. If you need to check the signature, call
    /// [`MessageBuffer.verify_signature`] before parsing the message.
    pub fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MessageBuffer::new(buffer.as_ref())?;
        let signature =
            Signature::from_byte_slice(&reader.signature()).context("failed to parse signature")?;
        let participant_pk = PublicSigningKey::from_byte_slice(&reader.participant_pk())
            .context("failed to parse public key")?;
        let coordinator_pk = PublicEncryptKey::from_byte_slice(&reader.coordinator_pk())
            .context("failed to parse public key")?;

        let tag = reader.tag().try_into()?;
        let is_multipart = reader.flags().contains(Flags::MULTIPART);

        let payload = if is_multipart {
            Chunk::from_byte_slice(&reader.payload()).map(Into::into)
        } else {
            match tag {
                Tag::Sum => Sum::from_byte_slice(&reader.payload()).map(Into::into),
                Tag::Update => Update::from_byte_slice(&reader.payload()).map(Into::into),
                Tag::Sum2 => Sum2::from_byte_slice(&reader.payload()).map(Into::into),
            }
        }
        .context("failed to parse message payload")?;

        Ok(Self {
            participant_pk,
            coordinator_pk,
            signature: Some(signature),
            payload,
            is_multipart,
            tag,
        })
    }

    /// Serialize this message. If the `signature` attribute is
    /// `Some`, the signature will be directly inserted in the message
    /// header. Otherwise it will be computed.
    ///
    /// # Panic
    ///
    /// This method panics if the given buffer is too small for the
    /// message to fit.
    pub fn to_bytes<T: AsMut<[u8]> + AsRef<[u8]> + ?Sized>(
        &self,
        buffer: &mut T,
        sk: &SecretSigningKey,
    ) {
        let mut writer = MessageBuffer::new(buffer.as_mut()).unwrap();

        self.participant_pk
            .to_bytes(&mut writer.participant_pk_mut());
        self.coordinator_pk
            .to_bytes(&mut writer.coordinator_pk_mut());
        let flags = if self.is_multipart {
            Flags::MULTIPART
        } else {
            Flags::empty()
        };
        writer.set_flags(flags);
        self.payload.to_bytes(&mut writer.payload_mut());
        // Determine the tag from the payload type if
        // possible. Otherwise, use the self.tag field.
        let tag = match self.payload {
            Payload::Sum(_) => Tag::Sum,
            Payload::Update(_) => Tag::Update,
            Payload::Sum2(_) => Tag::Sum2,
            Payload::Chunk(_) => self.tag,
        };
        writer.set_tag(tag.into());
        writer.set_length(self.buffer_length() as u32);
        // insert the signature last. If the message contains one, use
        // it. Otherwise compute it.
        let signature = match self.signature {
            Some(signature) => signature,
            None => sk.sign_detached(writer.signed_data_mut()),
        };
        signature.to_bytes(&mut writer.signature_mut());
    }

    pub fn buffer_length(&self) -> usize {
        self.payload.buffer_length() + HEADER_LENGTH
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use super::*;
    use crate::{
        message::{Message, Tag},
        testutils::messages as helpers,
    };

    fn sum_message() -> (Message, Vec<u8>) {
        helpers::message(helpers::sum::payload)
    }

    #[test]
    fn buffer_read() {
        let bytes = sum_message().1;
        let buffer = MessageBuffer::new(&bytes).unwrap();
        assert_eq!(Tag::try_from(buffer.tag()).unwrap(), Tag::Sum);
        assert_eq!(buffer.signature(), helpers::signature().1.as_slice());
        assert_eq!(
            buffer.participant_pk(),
            helpers::participant_pk().1.as_slice()
        );
        assert_eq!(
            buffer.coordinator_pk(),
            helpers::coordinator_pk().1.as_slice()
        );
        assert_eq!(buffer.length() as usize, bytes.len());
        assert_eq!(buffer.payload(), helpers::sum::payload().1.as_slice());
    }

    #[test]
    fn buffer_write() {
        let expected = sum_message().1;
        let mut bytes = vec![0; expected.len()];
        let mut buffer = MessageBuffer::new_unchecked(&mut bytes);

        buffer
            .signature_mut()
            .copy_from_slice(helpers::signature().1.as_slice());
        buffer
            .participant_pk_mut()
            .copy_from_slice(helpers::participant_pk().1.as_slice());
        buffer
            .coordinator_pk_mut()
            .copy_from_slice(helpers::coordinator_pk().1.as_slice());
        buffer.set_tag(Tag::Sum.into());
        buffer.set_length(expected.len() as u32);
        buffer
            .payload_mut()
            .copy_from_slice(helpers::sum::payload().1.as_slice());
        assert_eq!(bytes, expected);
    }
}
