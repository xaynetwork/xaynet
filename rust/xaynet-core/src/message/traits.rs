//! Message traits.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: ../index.html

use std::{
    convert::TryInto,
    io::{Cursor, Write},
    ops::Range,
};

use anyhow::{anyhow, Context};

use crate::{
    crypto::ByteObject,
    mask::seed::EncryptedMaskSeed,
    message::DecodeError,
    LocalSeedDict,
    SumParticipantPublicKey,
};

/// An interface for serializable message types.
///
/// See also [`FromBytes`] for deserialization.
pub trait ToBytes {
    /// The length of the buffer for encoding the type.
    fn buffer_length(&self) -> usize;

    /// Serialize the type in the given buffer.
    ///
    /// # Panics
    /// This method may panic if the given buffer is too small. Thus, [`buffer_length()`] must be
    /// called prior to calling this, and a large enough buffer must be provided.
    ///
    /// [`buffer_length()`]: #method.buffer_length
    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T);
}

/// An interface for deserializable message types.
///
/// See also [`ToBytes`] for serialization.
pub trait FromBytes: Sized {
    /// Deserialize the type from the given buffer.
    ///
    /// # Errors
    /// May fail if certain parts of the deserialized buffer don't pass message validity checks.
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError>;
}

impl<T> FromBytes for T
where
    T: ByteObject,
{
    fn from_bytes<U: AsRef<[u8]>>(buffer: &U) -> Result<Self, DecodeError> {
        Self::from_slice(buffer.as_ref())
            .ok_or_else(|| anyhow!("failed to deserialize byte object"))
    }
}

impl<T> ToBytes for T
where
    T: ByteObject,
{
    fn buffer_length(&self) -> usize {
        self.as_slice().len()
    }

    fn to_bytes<U: AsMut<[u8]>>(&self, buffer: &mut U) {
        buffer.as_mut().copy_from_slice(self.as_slice())
    }
}

/// A helper for encoding and decoding Length-Value (LV) fields.
///
/// Note that the 4 bytes [`length()`] field gives the length of the *total* Length-Value field,
/// _i.e._ the length of the value, plus the 4 extra bytes of the length field itself.
///
/// # Examples
/// ## Decoding a LV field
///
/// ```rust
/// # use xaynet_core::message::LengthValueBuffer;
/// let bytes = vec![
///     0x00, 0x00, 0x00, 0x05, // Length = 5
///     0xff, // Value = 0xff
///     0x11, 0x22, // Extra bytes
/// ];
/// let buffer = LengthValueBuffer::new(&bytes).unwrap();
/// assert_eq!(buffer.length(), 5);
/// assert_eq!(buffer.value_length(), 1);
/// assert_eq!(buffer.value(), &[0xff][..]);
/// ```
///
/// ## Encoding a LV field
///
/// ```rust
/// # use xaynet_core::message::LengthValueBuffer;
/// let mut bytes = vec![0xff; 9];
/// let mut buffer = LengthValueBuffer::new_unchecked(&mut bytes);
/// // It is important to set the length field before setting the value, otherwise, `value_mut()` will panic.
/// buffer.set_length(8);
/// buffer.value_mut().copy_from_slice(&[0, 1, 2, 3][..]);
/// let expected = vec![
///     0x00, 0x00, 0x00, 0x08, // Length = 8
///     0x00, 0x01, 0x02, 0x03, // Value
///     0xff, // unchanged
/// ];
///
/// assert_eq!(bytes, expected);
/// ```
///
/// [`length()`]: #method.length
pub struct LengthValueBuffer<T> {
    inner: T,
}

/// The size of the length field for encoding a Length-Value item.
const LENGTH_FIELD: Range<usize> = 0..4;

impl<T: AsRef<[u8]>> LengthValueBuffer<T> {
    /// Returns a new [`LengthValueBuffer`].
    ///
    /// # Errors
    /// This method performs bound checks and returns an error if the given buffer is not a valid
    /// Length-Value item.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use xaynet_core::message::LengthValueBuffer;
    /// // truncated length:
    /// assert!(LengthValueBuffer::new(&vec![0x00, 0x00, 0x00]).is_err());
    ///
    /// // truncated value:
    /// let bytes = vec![
    ///     0x00, 0x00, 0x00, 0x08, // length: 8
    ///     0x11, 0x22, 0x33, // value
    /// ];
    /// assert!(LengthValueBuffer::new(&bytes).is_err());
    ///
    /// // valid Length-Value item
    /// let bytes = vec![
    ///     0x00, 0x00, 0x00, 0x08, // length: 8
    ///     0x11, 0x22, 0x33, 0x44, // value
    ///     0xaa, 0xbb, // extra bytes are ignored
    /// ];
    /// let buf = LengthValueBuffer::new(&bytes).unwrap();
    /// assert_eq!(buf.length(), 8);
    /// assert_eq!(buf.value(), &[0x11, 0x22, 0x33, 0x44][..]);
    /// ```
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid LengthValueBuffer")?;
        Ok(buffer)
    }

    /// Create a new [`LengthValueBuffer`] without any bound checks.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Check that the buffer is a valid Length-Value item.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        if len < LENGTH_FIELD.end {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                LENGTH_FIELD.end
            ));
        }

        if (self.length() as usize) < LENGTH_FIELD.end {
            return Err(anyhow!(
                "invalid length value: {} (should be >= {})",
                len,
                LENGTH_FIELD.end
            ));
        }

        if len < self.length() as usize {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                self.length(),
            ));
        }
        Ok(())
    }

    /// Returns the length field. Note that the value of the length
    /// field includes the length of the field itself (4 bytes).
    ///
    /// # Panics
    /// This method may panic if buffer is not a valid Length-Value item.
    pub fn length(&self) -> u32 {
        // unwrap safe: the slice is exactly 4 bytes long
        u32::from_be_bytes(self.inner.as_ref()[LENGTH_FIELD].try_into().unwrap())
    }

    /// Returns the length of the value.
    pub fn value_length(&self) -> usize {
        self.length() as usize - LENGTH_FIELD.end
    }

    /// Returns the range corresponding to the value.
    fn value_range(&self) -> Range<usize> {
        let offset = LENGTH_FIELD.end;
        let value_length = self.value_length();
        offset..offset + value_length
    }
}

impl<T: AsMut<[u8]>> LengthValueBuffer<T> {
    /// Sets the length field to the given value.
    ///
    /// # Panics
    /// This method may panic if buffer is not a valid Length-Value item.
    pub fn set_length(&mut self, value: u32) {
        self.inner.as_mut()[LENGTH_FIELD].copy_from_slice(&value.to_be_bytes());
    }
}

impl<'a, T: AsRef<[u8]> + AsMut<[u8]> + ?Sized> LengthValueBuffer<&'a mut T> {
    /// Gets a mutable reference to the value field.
    ///
    /// # Panics
    /// This method may panic if buffer is not a valid Length-Value item.
    pub fn value_mut(&mut self) -> &mut [u8] {
        let range = self.value_range();
        &mut self.inner.as_mut()[range]
    }

    /// Gets a mutable reference to the underlying buffer.
    ///
    /// # Panics
    /// This method may panic if buffer is not a valid Length-Value item.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[..]
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> LengthValueBuffer<&'a T> {
    /// Gets a reference to the value field.
    ///
    /// # Panics
    /// This method may panic if buffer is not a valid Length-Value item.
    pub fn value(&self) -> &'a [u8] {
        &self.inner.as_ref()[self.value_range()]
    }

    /// Gets a reference to the underlying buffer.
    ///
    /// # Panics
    /// This method may panic if buffer is not a valid Length-Value item.
    pub fn bytes(self) -> &'a [u8] {
        let range = self.value_range();
        &self.inner.as_ref()[..range.end]
    }
}

const ENTRY_LENGTH: usize = SumParticipantPublicKey::LENGTH + EncryptedMaskSeed::LENGTH;

impl ToBytes for LocalSeedDict {
    fn buffer_length(&self) -> usize {
        LENGTH_FIELD.end + self.len() * ENTRY_LENGTH
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        let mut writer = Cursor::new(buffer.as_mut());
        let length = self.buffer_length() as u32;
        let _ = writer.write(&length.to_be_bytes()).unwrap();
        for (key, value) in self {
            let _ = writer.write(key.as_slice()).unwrap();
            let _ = writer.write(value.as_ref()).unwrap();
        }
    }
}

impl FromBytes for LocalSeedDict {
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = LengthValueBuffer::new(buffer.as_ref())?;
        let mut dict = LocalSeedDict::new();

        let key_length = SumParticipantPublicKey::LENGTH;
        let mut entries = reader.value().chunks_exact(ENTRY_LENGTH);
        for chunk in &mut entries {
            // safe unwraps: lengths of slices are guaranteed
            // by constants.
            let key = SumParticipantPublicKey::from_slice(&chunk[..key_length]).unwrap();
            let value = EncryptedMaskSeed::from_slice(&chunk[key_length..]).unwrap();
            if dict.insert(key, value).is_some() {
                return Err(anyhow!("invalid local seed dictionary: duplicated key"));
            }
        }
        if !entries.remainder().is_empty() {
            return Err(anyhow!("invalid local seed dictionary: trailing bytes"));
        }
        Ok(dict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_length_value_buffer() {
        let bytes = vec![
            0x00, 0x00, 0x00, 0x05, // Length = 1
            0xff, // Value = 0xff
            0x11, 0x22, // Extra bytes
        ];
        let buffer = LengthValueBuffer::new(&bytes).unwrap();
        assert_eq!(buffer.length(), 5);
        assert_eq!(buffer.value_length(), 1);
        assert_eq!(buffer.value(), &[0xff][..]);
    }

    #[test]
    fn decode_empty_value() {
        let bytes = vec![0x00, 0x00, 0x00, 0x04];
        let buffer = LengthValueBuffer::new(&bytes).unwrap();
        assert_eq!(buffer.length(), 4);
        assert_eq!(buffer.value_length(), 0);
    }

    #[test]
    fn decode_length_value_buffer_buffer_exhausted() {
        let bytes = vec![
            0x00, 0x00, 0x00, 0x08, // Length = 6
            0x11, 0x22, // Only 2 bytes
        ];
        assert!(LengthValueBuffer::new(bytes).is_err());
    }

    #[test]
    fn decode_length_value_buffer_invalid_length() {
        // Missing bytes
        let bytes = vec![0x00, 0x00, 0x00];
        assert!(LengthValueBuffer::new(bytes).is_err());
        // Length field invalid
        let bytes = vec![0x00, 0x00, 0x00, 0x03];
        assert!(LengthValueBuffer::new(bytes).is_err());
    }

    #[test]
    fn encode_length_value_buffer() {
        let mut bytes = vec![0xff; 7];
        let mut buffer = LengthValueBuffer::new_unchecked(&mut bytes);
        buffer.set_length(6);
        buffer.value_mut().copy_from_slice(&[0x11, 0x22][..]);
        let expected = vec![
            0x00, 0x00, 0x00, 0x06, // Length = 6
            0x11, 0x22, // Value
            0xff, // unchanged
        ];

        assert_eq!(bytes, expected);
    }

    #[test]
    fn encode_length_value_buffer_emty() {
        let mut bytes = vec![0xff; 5];
        let mut buffer = LengthValueBuffer::new_unchecked(&mut bytes);
        buffer.set_length(4);
        buffer.value_mut().copy_from_slice(&[][..]);
        let expected = vec![
            0x00, 0x00, 0x00, 0x04, // Length = 0
            0xff, // unchanged
        ];

        assert_eq!(bytes, expected);
    }
}
