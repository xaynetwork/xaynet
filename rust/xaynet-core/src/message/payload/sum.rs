//! Sum message payloads.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: ../index.html

use std::ops::Range;

use anyhow::{anyhow, Context};

use crate::{
    crypto::ByteObject,
    message::{
        traits::{FromBytes, ToBytes},
        utils::range,
        DecodeError,
    },
    ParticipantTaskSignature,
    SumParticipantEphemeralPublicKey,
};

const SUM_SIGNATURE_RANGE: Range<usize> = range(0, ParticipantTaskSignature::LENGTH);
const EPHM_PK_RANGE: Range<usize> = range(
    SUM_SIGNATURE_RANGE.end,
    SumParticipantEphemeralPublicKey::LENGTH,
);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// A wrapper around a buffer that contains a [`Sum`] message.
///
/// It provides getters and setters to access the different fields of the message safely.
///
/// # Examples
/// ## Decoding a sum message
///
/// ```rust
/// # use xaynet_core::message::SumBuffer;
/// let sum_signature = vec![0x11; 64];
/// let ephm_pk = vec![0x22; 32];
/// let bytes = [sum_signature.as_slice(), ephm_pk.as_slice()].concat();
/// let buffer = SumBuffer::new(&bytes).unwrap();
/// assert_eq!(buffer.sum_signature(), sum_signature.as_slice());
/// assert_eq!(buffer.ephm_pk(), ephm_pk.as_slice());
/// ```
///
/// ## Encoding a sum message
///
/// ```rust
/// # use xaynet_core::message::SumBuffer;
/// let sum_signature = vec![0x11; 64];
/// let ephm_pk = vec![0x22; 32];
/// let mut storage = vec![0xff; 96];
/// let mut buffer = SumBuffer::new_unchecked(&mut storage);
/// buffer
///     .sum_signature_mut()
///     .copy_from_slice(&sum_signature[..]);
/// buffer.ephm_pk_mut().copy_from_slice(&ephm_pk[..]);
/// assert_eq!(&storage[..64], sum_signature.as_slice());
/// assert_eq!(&storage[64..], ephm_pk.as_slice());
/// ```
pub struct SumBuffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> SumBuffer<T> {
    /// Performs bound checks for the various message fields on `bytes` and returns a new
    /// [`SumBuffer`].
    ///
    /// # Errors
    /// Fails if the `bytes` are smaller than a minimal-sized sum message buffer.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid SumBuffer")?;
        Ok(buffer)
    }

    /// Returns a [`SumBuffer`] without performing any bound checks.
    ///
    /// This means accessing the various fields may panic if the data is invalid.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Performs bound checks to ensure the fields can be accessed without panicking.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        if len < EPHM_PK_RANGE.end {
            Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                EPHM_PK_RANGE.end
            ))
        } else {
            Ok(())
        }
    }
}

impl<T: AsMut<[u8]>> SumBuffer<T> {
    /// Gets a mutable reference to the sum participant ephemeral public key field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn ephm_pk_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[EPHM_PK_RANGE]
    }

    /// Gets a mutable reference to the sum signature field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn sum_signature_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[SUM_SIGNATURE_RANGE]
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> SumBuffer<&'a T> {
    /// Gets a reference to the sum participant ephemeral public key field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn ephm_pk(&self) -> &'a [u8] {
        &self.inner.as_ref()[EPHM_PK_RANGE]
    }

    /// Gets a reference to the sum signature field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn sum_signature(&self) -> &'a [u8] {
        &self.inner.as_ref()[SUM_SIGNATURE_RANGE]
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
/// A high level representation of a sum message.
///
/// These messages are sent by sum participants during the sum phase.
///
/// # Examples
/// ## Decoding a message
///
/// ```rust
/// # use xaynet_core::{crypto::ByteObject, message::{FromBytes, Sum}, ParticipantTaskSignature, SumParticipantEphemeralPublicKey};
/// let signature = vec![0x11; 64];
/// let ephm_pk = vec![0x22; 32];
/// let bytes = [signature.as_slice(), ephm_pk.as_slice()].concat();
/// let parsed = Sum::from_byte_slice(&bytes).unwrap();
/// let expected = Sum{
///     sum_signature: ParticipantTaskSignature::from_slice(&signature[..]).unwrap(),
///     ephm_pk: SumParticipantEphemeralPublicKey::from_slice(&ephm_pk[..]).unwrap(),
/// };
/// assert_eq!(parsed, expected);
/// ```
///
/// ## Encoding a message
///
/// ```rust
/// # use xaynet_core::{crypto::ByteObject, message::{ToBytes, Sum}, ParticipantTaskSignature, SumParticipantEphemeralPublicKey};
/// let sum_signature = ParticipantTaskSignature::from_slice(vec![0x11; 64].as_slice()).unwrap();
/// let ephm_pk = SumParticipantEphemeralPublicKey::from_slice(vec![0x22; 32].as_slice()).unwrap();
/// let msg = Sum {
///     sum_signature,
///     ephm_pk,
/// };
/// // we need a 96 bytes long buffer to serialize that message
/// assert_eq!(msg.buffer_length(), 96);
/// // create a buffer with enough space and encode the message
/// let mut buf = vec![0xff; 96];
/// msg.to_bytes(&mut buf);
///
/// assert_eq!(buf, [vec![0x11; 64].as_slice(), vec![0x22; 32].as_slice()].concat());
/// ```
pub struct Sum {
    /// The signature of the round seed and the word "sum".
    ///
    /// This is used to determine whether a participant is selected for the sum task.
    pub sum_signature: ParticipantTaskSignature,
    /// An ephemeral public key generated by a sum participant for the current round.
    pub ephm_pk: SumParticipantEphemeralPublicKey,
}

impl ToBytes for Sum {
    fn buffer_length(&self) -> usize {
        EPHM_PK_RANGE.end
    }

    fn to_bytes<T: AsMut<[u8]> + AsRef<[u8]>>(&self, buffer: &mut T) {
        let mut writer = SumBuffer::new(buffer.as_mut()).unwrap();
        self.sum_signature.to_bytes(&mut writer.sum_signature_mut());
        self.ephm_pk.to_bytes(&mut writer.ephm_pk_mut());
    }
}

impl FromBytes for Sum {
    fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = SumBuffer::new(buffer.as_ref())?;

        let sum_signature = ParticipantTaskSignature::from_byte_slice(&reader.sum_signature())
            .context("invalid sum signature")?;

        let ephm_pk = SumParticipantEphemeralPublicKey::from_byte_slice(&reader.ephm_pk())
            .context("invalid ephemeral public key")?;

        Ok(Self {
            sum_signature,
            ephm_pk,
        })
    }

    fn from_byte_stream<I: Iterator<Item = u8> + ExactSizeIterator>(
        iter: &mut I,
    ) -> Result<Self, DecodeError> {
        let sum_signature =
            ParticipantTaskSignature::from_byte_stream(iter).context("invalid sum signature")?;
        let ephm_pk = SumParticipantEphemeralPublicKey::from_byte_stream(iter)
            .context("invalid ephemeral public key")?;

        Ok(Self {
            sum_signature,
            ephm_pk,
        })
    }
}

#[cfg(test)]
pub(in crate::message) mod tests {
    use super::*;
    use crate::crypto::ByteObject;

    fn sum_signature_bytes() -> Vec<u8> {
        vec![0x11; ParticipantTaskSignature::LENGTH]
    }

    fn ephm_pk_bytes() -> Vec<u8> {
        vec![0x22; SumParticipantEphemeralPublicKey::LENGTH]
    }

    pub(crate) fn sum_bytes() -> Vec<u8> {
        [sum_signature_bytes().as_slice(), ephm_pk_bytes().as_slice()].concat()
    }

    pub(crate) fn sum() -> Sum {
        let sum_signature =
            ParticipantTaskSignature::from_slice(&sum_signature_bytes()[..]).unwrap();
        let ephm_pk = SumParticipantEphemeralPublicKey::from_slice(&ephm_pk_bytes()).unwrap();
        Sum {
            sum_signature,
            ephm_pk,
        }
    }

    #[test]
    fn buffer_read() {
        let bytes = sum_bytes();
        let buffer = SumBuffer::new(&bytes).unwrap();
        assert_eq!(buffer.sum_signature(), &sum_signature_bytes()[..]);
        assert_eq!(buffer.ephm_pk(), &ephm_pk_bytes()[..]);
    }

    #[test]
    fn buffer_read_invalid() {
        let bytes = sum_bytes();
        assert!(SumBuffer::new(&bytes[1..]).is_err());
    }

    #[test]
    fn buffer_write() {
        let mut buffer = vec![0xff; EPHM_PK_RANGE.end];
        let mut writer = SumBuffer::new_unchecked(&mut buffer);
        writer
            .sum_signature_mut()
            .copy_from_slice(sum_signature_bytes().as_slice());
        writer
            .ephm_pk_mut()
            .copy_from_slice(ephm_pk_bytes().as_slice());
    }

    #[test]
    fn encode() {
        let message = sum();
        assert_eq!(message.buffer_length(), sum_bytes().len());

        let mut buf = vec![0xff; message.buffer_length()];
        message.to_bytes(&mut buf);
        assert_eq!(buf, sum_bytes());
    }

    #[test]
    fn decode() {
        let parsed = Sum::from_byte_slice(&sum_bytes()).unwrap();
        let expected = sum();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn stream_parse() {
        let parsed = Sum::from_byte_stream(&mut sum_bytes().into_iter()).unwrap();
        let expected = sum();
        assert_eq!(parsed, expected);
    }
}
