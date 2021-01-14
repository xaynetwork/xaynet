//! Update message payloads.
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
        traits::{FromBytes, LengthValueBuffer, ToBytes},
        utils::range,
        DecodeError,
    },
    LocalSeedDict,
    ParticipantTaskSignature,
};

const SUM_SIGNATURE_RANGE: Range<usize> = range(0, ParticipantTaskSignature::LENGTH);
const UPDATE_SIGNATURE_RANGE: Range<usize> =
    range(SUM_SIGNATURE_RANGE.end, ParticipantTaskSignature::LENGTH);

#[derive(Clone, Debug)]
/// A wrapper around a buffer that contains an [`Update`] message.
///
/// It provides getters and setters to access the different fields of the message safely.
pub struct UpdateBuffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> UpdateBuffer<T> {
    /// Performs bound checks for the various message fields on `bytes` and returns a new
    /// [`UpdateBuffer`].
    ///
    /// # Errors
    /// Fails if the `bytes` are smaller than a minimal-sized update message buffer.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("invalid UpdateBuffer")?;
        Ok(buffer)
    }

    /// Returns an [`UpdateBuffer`] without performing any bound checks.
    ///
    /// This means accessing the various fields may panic if the data is invalid.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Performs bound checks to ensure the fields can be accessed without panicking.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        // First, check the fixed size portion of the
        // header. UPDATE_SIGNATURE_RANGE is the last field
        if len < UPDATE_SIGNATURE_RANGE.end {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                UPDATE_SIGNATURE_RANGE.end
            ));
        }

        // Check length of the masked object field
        MaskObjectBuffer::new(&self.inner.as_ref()[self.masked_model_offset()..])
            .context("invalid masked object field")?;

        // Check the length of the local seed dictionary field
        let _ = LengthValueBuffer::new(&self.inner.as_ref()[self.local_seed_dict_offset()..])
            .context("invalid local seed dictionary length")?;

        Ok(())
    }

    /// Gets the offset of the masked model field.
    fn masked_model_offset(&self) -> usize {
        UPDATE_SIGNATURE_RANGE.end
    }

    /// Gets the offset of the local seed dictionary field.
    ///
    /// # Panics
    /// Computing the offset may panic if the buffer has not been checked before.
    fn local_seed_dict_offset(&self) -> usize {
        let masked_model =
            MaskObjectBuffer::new_unchecked(&self.inner.as_ref()[self.masked_model_offset()..]);
        self.masked_model_offset() + masked_model.len()
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> UpdateBuffer<&'a T> {
    /// Gets the sum signature field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn sum_signature(&self) -> &'a [u8] {
        &self.inner.as_ref()[SUM_SIGNATURE_RANGE]
    }

    /// Gets the update signature field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn update_signature(&self) -> &'a [u8] {
        &self.inner.as_ref()[UPDATE_SIGNATURE_RANGE]
    }

    /// Gets a slice that starts at the beginning of the masked model field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn masked_model(&self) -> &'a [u8] {
        let offset = self.masked_model_offset();
        &self.inner.as_ref()[offset..]
    }

    /// Gets a slice that starts at the beginning og the local seed dictionary field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn local_seed_dict(&self) -> &'a [u8] {
        let offset = self.local_seed_dict_offset();
        &self.inner.as_ref()[offset..]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> UpdateBuffer<T> {
    /// Gets a mutable reference to the sum signature field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn sum_signature_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[SUM_SIGNATURE_RANGE]
    }

    /// Gets a mutable reference to the update signature field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn update_signature_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[UPDATE_SIGNATURE_RANGE]
    }

    /// Gets a mutable slice that starts at the beginning of the masked model field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn masked_model_mut(&mut self) -> &mut [u8] {
        let offset = self.masked_model_offset();
        &mut self.inner.as_mut()[offset..]
    }

    /// Gets a mutable slice that starts at the beginning of the local seed dictionary field.
    ///
    /// # Panics
    /// Accessing the field may panic if the buffer has not been checked before.
    pub fn local_seed_dict_mut(&mut self) -> &mut [u8] {
        let offset = self.local_seed_dict_offset();
        &mut self.inner.as_mut()[offset..]
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
/// A high level representation of an update message.
///
/// These messages are sent by update participants during the update phase.
pub struct Update {
    /// The signature of the round seed and the word "sum".
    ///
    /// This is used to determine whether a participant is selected for the sum task.
    pub sum_signature: ParticipantTaskSignature,
    /// Signature of the round seed and the word "update".
    ///
    /// This is used to determine whether a participant is selected for the update task.
    pub update_signature: ParticipantTaskSignature,
    /// A model trained by an update participant.
    ///
    /// The model is masked with randomness derived from the participant seed.
    pub masked_model: MaskObject,
    /// A dictionary that contains the seed used to mask `masked_model`.
    ///
    /// The seed is encrypted with the ephemeral public key of each sum participant.
    pub local_seed_dict: LocalSeedDict,
}

impl ToBytes for Update {
    fn buffer_length(&self) -> usize {
        UPDATE_SIGNATURE_RANGE.end
            + self.masked_model.buffer_length()
            + self.local_seed_dict.buffer_length()
    }

    fn to_bytes<T: AsMut<[u8]> + AsRef<[u8]>>(&self, buffer: &mut T) {
        let mut writer = UpdateBuffer::new_unchecked(buffer.as_mut());
        self.sum_signature.to_bytes(&mut writer.sum_signature_mut());
        self.update_signature
            .to_bytes(&mut writer.update_signature_mut());
        self.masked_model.to_bytes(&mut writer.masked_model_mut());
        self.local_seed_dict
            .to_bytes(&mut writer.local_seed_dict_mut());
    }
}

impl FromBytes for Update {
    fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = UpdateBuffer::new(buffer.as_ref())?;
        Ok(Self {
            sum_signature: ParticipantTaskSignature::from_byte_slice(&reader.sum_signature())
                .context("invalid sum signature")?,
            update_signature: ParticipantTaskSignature::from_byte_slice(&reader.update_signature())
                .context("invalid update signature")?,
            masked_model: MaskObject::from_byte_slice(&reader.masked_model())
                .context("invalid masked model")?,
            local_seed_dict: LocalSeedDict::from_byte_slice(&reader.local_seed_dict())
                .context("invalid local seed dictionary")?,
        })
    }

    fn from_byte_stream<I: Iterator<Item = u8> + ExactSizeIterator>(
        iter: &mut I,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            sum_signature: ParticipantTaskSignature::from_byte_stream(iter)
                .context("invalid sum signature")?,
            update_signature: ParticipantTaskSignature::from_byte_stream(iter)
                .context("invalid update signature")?,
            masked_model: MaskObject::from_byte_stream(iter).context("invalid masked model")?,
            local_seed_dict: LocalSeedDict::from_byte_stream(iter)
                .context("invalid local seed dictionary")?,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::testutils::messages::update as helpers;

    #[test]
    fn buffer_read() {
        let bytes = helpers::payload().1;
        let buffer = UpdateBuffer::new(&bytes).unwrap();
        assert_eq!(
            buffer.sum_signature(),
            helpers::sum_task_signature().1.as_slice()
        );
        assert_eq!(
            buffer.update_signature(),
            helpers::update_task_signature().1.as_slice()
        );
        let expected = helpers::mask_object().1;
        assert_eq!(&buffer.masked_model()[..expected.len()], &expected[..]);
        assert_eq!(buffer.local_seed_dict(), &helpers::local_seed_dict().1[..]);
    }

    #[test]
    fn decode_invalid_seed_dict() {
        let mut invalid = helpers::local_seed_dict().1;
        // This truncates the last entry of the seed dictionary
        invalid[3] = 0xe3;
        let mut bytes = vec![];
        bytes.extend(helpers::sum_task_signature().1);
        bytes.extend(helpers::update_task_signature().1);
        bytes.extend(helpers::mask_object().1);
        bytes.extend(invalid);

        let e = Update::from_byte_slice(&bytes).unwrap_err();
        let cause = e.source().unwrap().to_string();
        assert_eq!(
            cause,
            "invalid local seed dictionary: trailing bytes".to_string()
        );
    }

    #[test]
    fn decode() {
        let (update, bytes) = helpers::payload();
        let parsed = Update::from_byte_slice(&bytes).unwrap();
        assert_eq!(parsed, update);
    }

    #[test]
    fn stream_parse() {
        let (update, bytes) = helpers::payload();
        let parsed = Update::from_byte_stream(&mut bytes.into_iter()).unwrap();
        assert_eq!(parsed, update);
    }

    #[test]
    fn encode() {
        let (update, bytes) = helpers::payload();
        assert_eq!(update.buffer_length(), bytes.len());
        let mut buf = vec![0xff; update.buffer_length()];
        update.to_bytes(&mut buf);
        // The order in which the hashmap is serialized is not
        // guaranteed, but we chose our key/values such that they are
        // sorted.
        //
        // First compute the offset at which the local seed dict value
        // starts: two signature (64 bytes), the masked model (32
        // bytes), the length field (4 bytes), the masked scalar (10 bytes)
        let offset = 64 * 2 + 32 + 4 + 10;
        // Sort the end of the buffer
        (&mut buf[offset..]).sort_unstable();
        assert_eq!(buf, bytes);
    }
}
