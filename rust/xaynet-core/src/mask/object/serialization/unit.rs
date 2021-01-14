//! Serialization of masked units.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: crate::mask

use std::ops::Range;

use anyhow::{anyhow, Context};
use num::bigint::BigUint;

use crate::{
    mask::{
        config::{serialization::MASK_CONFIG_BUFFER_LEN, MaskConfig},
        object::MaskUnit,
    },
    message::{
        traits::{FromBytes, ToBytes},
        utils::range,
        DecodeError,
    },
};

const MASK_CONFIG_FIELD: Range<usize> = range(0, MASK_CONFIG_BUFFER_LEN);

/// A buffer for serialized mask units.
pub struct MaskUnitBuffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> MaskUnitBuffer<T> {
    /// Creates a new buffer from `bytes`.
    ///
    /// # Errors
    /// Fails if the `bytes` don't conform to the required buffer length for mask units.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid mask unit")?;
        Ok(buffer)
    }

    /// Creates a new buffer from `bytes`.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Checks if this buffer conforms to the required buffer length for mask units.
    ///
    /// # Errors
    /// Fails if the buffer is too small.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        if len < MASK_CONFIG_FIELD.end {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                MASK_CONFIG_FIELD.end
            ));
        }

        let total_expected_length = self.try_len()?;
        if len < total_expected_length {
            return Err(anyhow!(
                "invalid buffer length: expected {} bytes but buffer has only {} bytes",
                total_expected_length,
                len
            ));
        }
        Ok(())
    }

    /// Return the expected length of the underlying byte buffer,
    /// based on the masking config field of numbers field. This is
    /// similar to [`len()`] but cannot panic.
    ///
    /// [`len()`]: MaskUnitBuffer::len
    pub fn try_len(&self) -> Result<usize, DecodeError> {
        let config =
            MaskConfig::from_byte_slice(&self.config()).context("invalid mask unit buffer")?;
        let data_length = config.bytes_per_number();
        Ok(MASK_CONFIG_FIELD.end + data_length)
    }

    /// Gets the expected number of bytes of this buffer wrt to the masking configuration.
    ///
    /// # Panics
    /// Panics if the serialized masking configuration is invalid.
    pub fn len(&self) -> usize {
        let config = MaskConfig::from_byte_slice(&self.config()).unwrap();
        let data_length = config.bytes_per_number();
        MASK_CONFIG_FIELD.end + data_length
    }

    /// Gets the serialized masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn config(&self) -> &[u8] {
        &self.inner.as_ref()[MASK_CONFIG_FIELD]
    }

    /// Gets the serialized mask unit element.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn data(&self) -> &[u8] {
        &self.inner.as_ref()[MASK_CONFIG_FIELD.end..self.len()]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> MaskUnitBuffer<T> {
    /// Gets the serialized masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn config_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[MASK_CONFIG_FIELD]
    }

    /// Gets the serialized mask unit element.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn data_mut(&mut self) -> &mut [u8] {
        let end = self.len();
        &mut self.inner.as_mut()[MASK_CONFIG_FIELD.end..end]
    }
}

impl ToBytes for MaskUnit {
    fn buffer_length(&self) -> usize {
        MASK_CONFIG_FIELD.end + self.config.bytes_per_number()
    }

    fn to_bytes<T: AsMut<[u8]> + AsRef<[u8]>>(&self, buffer: &mut T) {
        let mut writer = MaskUnitBuffer::new_unchecked(buffer.as_mut());
        self.config.to_bytes(&mut writer.config_mut());

        let data = writer.data_mut();
        // FIXME: this allocates a vec which is sub-optimal. See
        // https://github.com/rust-num/num-bigint/issues/152
        let bytes = self.data.to_bytes_le();
        // This may panic if the data is invalid and is an
        // integer that is bigger than what is expected by the
        // configuration.
        data[..bytes.len()].copy_from_slice(&bytes[..]);
        // padding
        for b in data
            .iter_mut()
            .take(self.config.bytes_per_number())
            .skip(bytes.len())
        {
            *b = 0;
        }
    }
}

impl FromBytes for MaskUnit {
    fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MaskUnitBuffer::new(buffer.as_ref())?;
        let config = MaskConfig::from_byte_slice(&reader.config())?;
        let data = BigUint::from_bytes_le(reader.data());

        Ok(MaskUnit { data, config })
    }

    fn from_byte_stream<I: Iterator<Item = u8> + ExactSizeIterator>(
        iter: &mut I,
    ) -> Result<Self, DecodeError> {
        let config = MaskConfig::from_byte_stream(iter)?;
        if iter.len() < 4 {
            return Err(anyhow!("byte stream exhausted"));
        }
        let data_len = config.bytes_per_number();
        if iter.len() < data_len {
            return Err(anyhow!(
                "mask unit is {} bytes long but byte stream only has {} bytes",
                data_len,
                iter.len()
            ));
        }

        let mut buf = vec![0; data_len];
        for (i, b) in iter.take(data_len).enumerate() {
            buf[i] = b;
        }
        let data = BigUint::from_bytes_le(buf.as_slice());

        Ok(MaskUnit { data, config })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mask::object::serialization::tests::mask_config;

    pub fn mask_unit() -> (MaskUnit, Vec<u8>) {
        let (config, mut bytes) = mask_config();
        let data = BigUint::from(1_u8);
        let mask_unit = MaskUnit::new_unchecked(config, data);

        bytes.extend(vec![
            // data (6 bytes with this config)
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // 1
        ]);
        (mask_unit, bytes)
    }

    #[test]
    fn serialize_mask_unit() {
        let (mask_unit, expected) = mask_unit();
        let mut buf = vec![0xff; expected.len()];
        mask_unit.to_bytes(&mut buf);
        assert_eq!(buf, expected);
    }

    #[test]
    fn deserialize_mask_unit() {
        let (expected, bytes) = mask_unit();
        assert_eq!(MaskUnit::from_byte_slice(&&bytes[..]).unwrap(), expected);
    }

    #[test]
    fn deserialize_mask_unit_from_stream() {
        let (expected, bytes) = mask_unit();
        assert_eq!(
            MaskUnit::from_byte_stream(&mut bytes.into_iter()).unwrap(),
            expected
        );
    }
}
