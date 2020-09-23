use std::convert::TryInto;

use anyhow::{anyhow, Context};

use crate::message::{
    traits::{FromBytes, ToBytes},
    DecodeError,
};

pub(crate) mod ranges {
    use crate::message::utils::range;
    use std::ops::Range;

    /// Byte range corresponding to the chunk ID in a chunk message
    pub const ID: Range<usize> = range(0, 2);
    /// Byte range corresponding to the message ID in a chunk message
    pub const MESSAGE_ID: Range<usize> = range(ID.end, 2);
    /// Byte range corresponding to the flags in a chunk message
    pub const FLAGS: usize = MESSAGE_ID.end;
    /// Byte range reserved for future use
    pub const RESERVED: Range<usize> = range(FLAGS + 1, 3);
}

/// Length in bytes of a chunk message header
const HEADER_LENGTH: usize = ranges::RESERVED.end;

/// A message chunk.
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Chunk {
    /// Chunk ID
    pub id: u16,
    /// ID of the message this chunk belongs to
    pub message_id: u16,
    /// `true` if this is the last chunk of the message, `false` otherwise
    pub last: bool,
    /// Data contained in this chunk.
    pub data: Vec<u8>,
}

bitflags::bitflags! {
    /// A bitmask that defines flags for a [`Chunk`].
    pub struct Flags: u8 {
        /// Indicates whether this message is the last chunk of a
        /// multipart message
        const LAST_CHUNK = 1 << 0;
    }
}

/// ```no_rust
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                id             |           message_id          |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |     flags     |                    reserved                   |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// +                       data (variable length)                  +
/// |                                                               |
/// ```
///
/// - `id`: ID of the chunk
/// - `message_id`: ID of the message this chunk belong to
/// - `flags`: currently the only supported flag indicates whether
///   this is the last chunk or not
pub struct ChunkBuffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> ChunkBuffer<T> {
    /// Performs bound checks for the various message fields on `bytes` and returns a new
    /// [`ChunkBuffer`].
    ///
    /// # Errors
    /// Fails if the `bytes` are smaller than a minimal-sized message buffer.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid ChunkBuffer")?;
        Ok(buffer)
    }

    /// Returns a [`ChunkBuffer`] without performing any bound checks.
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
        Ok(())
    }

    /// Gets the flags field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn flags(&self) -> Flags {
        Flags::from_bits_truncate(self.inner.as_ref()[ranges::FLAGS])
    }

    /// Gets the chunk ID field
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn id(&self) -> u16 {
        // Unwrapping is OK, as the slice is guaranteed to be 4 bytes
        // long
        u16::from_be_bytes(self.inner.as_ref()[ranges::ID].try_into().unwrap())
    }

    /// Gets the message ID field
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn message_id(&self) -> u16 {
        // Unwrapping is OK, as the slice is guaranteed to be 4 bytes
        // long
        u16::from_be_bytes(self.inner.as_ref()[ranges::MESSAGE_ID].try_into().unwrap())
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> ChunkBuffer<&'a T> {
    /// Gets the rest of the message.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn payload(&self) -> &'a [u8] {
        &self.inner.as_ref()[HEADER_LENGTH..]
    }
}

impl<T: AsMut<[u8]> + AsRef<[u8]>> ChunkBuffer<T> {
    /// Sets the flags field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn set_flags(&mut self, value: Flags) {
        self.inner.as_mut()[ranges::FLAGS] = value.bits();
    }

    /// Sets the chunk ID field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn set_id(&mut self, value: u16) {
        let bytes = value.to_be_bytes();
        self.inner.as_mut()[ranges::ID].copy_from_slice(&bytes[..]);
    }

    /// Sets the message ID field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn set_message_id(&mut self, value: u16) {
        let bytes = value.to_be_bytes();
        self.inner.as_mut()[ranges::MESSAGE_ID].copy_from_slice(&bytes[..]);
    }

    /// Gets a mutable reference to the rest of the message.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[HEADER_LENGTH..]
    }
}

impl FromBytes for Chunk {
    fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = ChunkBuffer::new(buffer.as_ref()).context("Invalid chunk buffer")?;
        Ok(Self {
            last: reader.flags().contains(Flags::LAST_CHUNK),
            id: reader.id(),
            message_id: reader.message_id(),
            data: reader.payload().to_vec(),
        })
    }

    fn from_byte_stream<I: Iterator<Item = u8> + ExactSizeIterator>(
        iter: &mut I,
    ) -> Result<Self, DecodeError> {
        if iter.len() < HEADER_LENGTH {
            return Err(anyhow!("byte stream exhausted"));
        }
        let id = u16::from_byte_stream(iter).context("cannot parse id")?;
        let message_id = u16::from_byte_stream(iter).context("cannot parse message id")?;
        let flags = Flags::from_bits_truncate(iter.next().unwrap());
        let data: Vec<u8> = iter.skip(3).collect();
        Ok(Self {
            id,
            message_id,
            data,
            last: flags.contains(Flags::LAST_CHUNK),
        })
    }
}

impl ToBytes for Chunk {
    fn buffer_length(&self) -> usize {
        HEADER_LENGTH + self.data.len()
    }

    fn to_bytes<T: AsMut<[u8]> + AsRef<[u8]>>(&self, buffer: &mut T) {
        let mut writer = ChunkBuffer::new(buffer.as_mut()).unwrap();
        let flags = if self.last {
            Flags::LAST_CHUNK
        } else {
            Flags::empty()
        };
        writer.set_flags(flags);
        writer.set_id(self.id);
        writer.set_message_id(self.message_id);
        writer.payload_mut()[..self.data.len()].copy_from_slice(self.data.as_slice());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags() -> (u8, Flags) {
        let flags = Flags::LAST_CHUNK;
        (flags.bits(), flags)
    }

    fn id() -> (Vec<u8>, u16) {
        let value = 0xdddd_u16;
        (value.to_be_bytes().to_vec(), value)
    }

    fn message_id() -> (Vec<u8>, u16) {
        let value = 0xeeee_u16;
        (value.to_be_bytes().to_vec(), value)
    }

    fn data() -> Vec<u8> {
        vec![0xff; 10]
    }

    fn chunk() -> (Vec<u8>, Chunk) {
        let mut bytes = vec![];
        bytes.extend(id().0);
        bytes.extend(message_id().0);
        bytes.push(flags().0);
        bytes.extend(vec![0x00, 0x00, 0x00]);
        bytes.extend(data());

        let message = Chunk {
            id: id().1,
            message_id: message_id().1,
            last: flags().1.contains(Flags::LAST_CHUNK),
            data: data(),
        };
        (bytes, message)
    }

    #[test]
    fn buffer_read() {
        let bytes = chunk().0;
        let buffer = ChunkBuffer::new(&bytes).unwrap();
        assert_eq!(buffer.id(), id().1);
        assert_eq!(buffer.message_id(), message_id().1);
        assert_eq!(buffer.flags(), flags().1);
        assert_eq!(buffer.payload(), &data()[..]);
    }

    #[test]
    fn stream_parse() {
        let (bytes, expected) = chunk();
        let actual = Chunk::from_byte_stream(&mut bytes.into_iter()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn buffer_write() {
        let expected = chunk().0;
        let mut bytes = vec![0; expected.len()];
        let mut buffer = ChunkBuffer::new_unchecked(&mut bytes);

        buffer.set_id(id().1);
        buffer.set_message_id(message_id().1);
        buffer.set_flags(flags().1);
        buffer.payload_mut().copy_from_slice(data().as_slice());
        assert_eq!(bytes, expected);
    }
}
