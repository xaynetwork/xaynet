//! Sum2 message payloads.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: crate::message

use std::ops::Range;

use anyhow::{anyhow, Context};

use crate::{
    crypto::ByteObject,
    mask::object::{serialization::MaskObjectBuffer, MaskObject},
    message::{
        traits::{FromBytes, ToBytes},
        utils::range,
        DecodeError,
    },
    ParticipantTaskSignature,
};

const SUM_SIGNATURE_RANGE: Range<usize> = range(0, ParticipantTaskSignature::LENGTH);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// A wrapper around a buffer that contains a [`Sum2`] message.
///
/// It provides getters and setters to access the different fields of the message safely.
pub struct Sum2Buffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> Sum2Buffer<T> {
    /// Performs bound checks for the various message fields on `bytes` and returns a new
    /// [`Sum2Buffer`].
    ///
    /// # Errors
    /// Fails if the `bytes` are smaller than a minimal-sized sum2 message buffer.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid Sum2Buffer")?;
        Ok(buffer)
    }

    /// Returns a `Sum2Buffer` with the given `bytes` without performing bound checks.
    ///
    /// This means that accessing the message fields may panic.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Performs bound checks for the various message fields on this buffer.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        if len < SUM_SIGNATURE_RANGE.end {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                SUM_SIGNATURE_RANGE.end
            ));
        }

        // check the length of the mask field
        MaskObjectBuffer::new(&self.inner.as_ref()[self.model_mask_offset()..])
            .context("invalid mask field")?;

        Ok(())
    }

    /// Gets the offset of the model mask field.
    fn model_mask_offset(&self) -> usize {
        SUM_SIGNATURE_RANGE.end
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Sum2Buffer<T> {
    /// Gets a mutable reference to the sum signature field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn sum_signature_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[SUM_SIGNATURE_RANGE]
    }

    /// Gets a mutable reference to the model mask field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn model_mask_mut(&mut self) -> &mut [u8] {
        let offset = self.model_mask_offset();
        &mut self.inner.as_mut()[offset..]
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Sum2Buffer<&'a T> {
    /// Gets a reference to the sum signature field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn sum_signature(&self) -> &'a [u8] {
        &self.inner.as_ref()[SUM_SIGNATURE_RANGE]
    }

    /// Gets a reference to the model mask field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn model_mask(&self) -> &'a [u8] {
        let offset = self.model_mask_offset();
        &self.inner.as_ref()[offset..]
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
/// A high level representation of a sum2 message.
///
/// These messages are sent by sum participants during the sum2 phase.
pub struct Sum2 {
    /// The signature of the round seed and the word "sum".
    ///
    /// This is used to determine whether a participant is selected for the sum task.
    pub sum_signature: ParticipantTaskSignature,

    /// A model mask computed by the participant.
    pub model_mask: MaskObject,
}

impl ToBytes for Sum2 {
    fn buffer_length(&self) -> usize {
        SUM_SIGNATURE_RANGE.end + self.model_mask.buffer_length()
    }

    fn to_bytes<T: AsMut<[u8]> + AsRef<[u8]>>(&self, buffer: &mut T) {
        let mut writer = Sum2Buffer::new_unchecked(buffer.as_mut());
        self.sum_signature.to_bytes(&mut writer.sum_signature_mut());
        self.model_mask.to_bytes(&mut writer.model_mask_mut());
    }
}

impl FromBytes for Sum2 {
    fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = Sum2Buffer::new(buffer.as_ref())?;
        Ok(Self {
            sum_signature: ParticipantTaskSignature::from_byte_slice(&reader.sum_signature())
                .context("invalid sum signature")?,
            model_mask: MaskObject::from_byte_slice(&reader.model_mask())
                .context("invalid mask")?,
        })
    }

    fn from_byte_stream<I: Iterator<Item = u8> + ExactSizeIterator>(
        iter: &mut I,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            sum_signature: ParticipantTaskSignature::from_byte_stream(iter)
                .context("invalid sum signature")?,
            model_mask: MaskObject::from_byte_stream(iter).context("invalid mask object")?,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use crate::testutils::messages::sum2 as helpers;

    use super::*;

    #[test]
    fn buffer_read() {
        let bytes = helpers::payload().1;
        let buffer = Sum2Buffer::new(&bytes).unwrap();
        assert_eq!(buffer.sum_signature(), &helpers::sum_task_signature().1[..]);

        let expected_mask = helpers::mask_object().1;
        let expected_length = expected_mask.len();
        let actual_mask = &buffer.model_mask()[..expected_length];
        assert_eq!(actual_mask, expected_mask);
    }

    #[test]
    fn buffer_write() {
        // length = 64 (signature) + 42 (mask) = 106
        let mut bytes = vec![0xff; 106];
        {
            let mut buffer = Sum2Buffer::new_unchecked(&mut bytes);
            buffer
                .sum_signature_mut()
                .copy_from_slice(&helpers::sum_task_signature().1[..]);
            let mask = helpers::mask_object().1;
            buffer.model_mask_mut()[..mask.len()].copy_from_slice(&mask[..]);
        }
        assert_eq!(&bytes[..], &helpers::payload().1[..]);
    }

    #[test]
    fn encode() {
        let (sum2, bytes) = helpers::payload();
        assert_eq!(sum2.buffer_length(), bytes.len());

        let mut buf = vec![0xff; sum2.buffer_length()];
        sum2.to_bytes(&mut buf);
        assert_eq!(buf, bytes);
    }

    #[test]
    fn decode() {
        let (sum2, bytes) = helpers::payload();
        let parsed = Sum2::from_byte_slice(&bytes).unwrap();
        assert_eq!(parsed, sum2);
    }

    #[test]
    fn stream_parse() {
        let (sum2, bytes) = helpers::payload();
        let parsed = Sum2::from_byte_stream(&mut bytes.into_iter()).unwrap();
        assert_eq!(parsed, sum2);
    }
}
