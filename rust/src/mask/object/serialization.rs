//! Serialization of masked objects.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: ../index.html

use std::{convert::TryInto, ops::Range};

use anyhow::{anyhow, Context};
use num::bigint::BigUint;

use crate::{
    mask::{config::serialization::MASK_CONFIG_BUFFER_LEN, object::MaskObject, MaskConfig},
    message::{utils::range, DecodeError, FromBytes, ToBytes},
};

const MASK_CONFIG_FIELD: Range<usize> = range(0, MASK_CONFIG_BUFFER_LEN);
const NUMBERS_FIELD: Range<usize> = range(MASK_CONFIG_FIELD.end, 4);

/// A buffer for serialized mask objects.
pub struct MaskObjectBuffer<T> {
    inner: T,
}

#[allow(clippy::len_without_is_empty)]
impl<T: AsRef<[u8]>> MaskObjectBuffer<T> {
    /// Creates a new buffer from `bytes`.
    ///
    /// # Errors
    /// Fails if the `bytes` don't conform to the required buffer length for mask objects.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid MaskObject")?;
        Ok(buffer)
    }

    /// Creates a new buffer from `bytes`.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Checks if this buffer conforms to the required buffer length for mask objects.
    ///
    /// # Errors
    /// Fails if the buffer is to small.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        if len < NUMBERS_FIELD.end {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                NUMBERS_FIELD.end
            ));
        }

        let config = MaskConfig::from_bytes(&self.config()).context("invalid MaskObject buffer")?;
        let bytes_per_number = config.bytes_per_number();
        let (data_length, overflows) = (self.numbers() as usize).overflowing_mul(bytes_per_number);
        if overflows {
            return Err(anyhow!(
                "invalid MaskObject buffer: invalid masking config or numbers field"
            ));
        }
        let total_expected_length = NUMBERS_FIELD.end + data_length;
        if len < total_expected_length {
            return Err(anyhow!(
                "invalid buffer length: expected {} bytes but buffer has only {} bytes",
                total_expected_length,
                len
            ));
        }
        Ok(())
    }

    /// Gets the expected number of bytes of this buffer wrt to the masking configuration.
    ///
    /// # Panics
    /// Panics if the serialized masking configuration is invalid.
    pub fn len(&self) -> usize {
        let config = MaskConfig::from_bytes(&self.config()).unwrap();
        let bytes_per_number = config.bytes_per_number();
        let data_length = self.numbers() as usize * bytes_per_number;
        NUMBERS_FIELD.end + data_length
    }

    /// Gets the number of serialized mask object elements.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn numbers(&self) -> u32 {
        // UNWRAP SAFE: the slice is exactly 4 bytes long
        u32::from_be_bytes(self.inner.as_ref()[NUMBERS_FIELD].try_into().unwrap())
    }

    /// Gets the serialized masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn config(&self) -> &[u8] {
        &self.inner.as_ref()[MASK_CONFIG_FIELD]
    }

    /// Gets the serialized mask object elements.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn data(&self) -> &[u8] {
        &self.inner.as_ref()[NUMBERS_FIELD.end..self.len()]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> MaskObjectBuffer<T> {
    /// Sets the number of serialized mask object elements.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn set_numbers(&mut self, value: u32) {
        self.inner.as_mut()[NUMBERS_FIELD].copy_from_slice(&value.to_be_bytes());
    }

    /// Gets the serialized masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn config_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[MASK_CONFIG_FIELD]
    }

    /// Gets the serialized mask object elements.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn data_mut(&mut self) -> &mut [u8] {
        let end = self.len();
        &mut self.inner.as_mut()[NUMBERS_FIELD.end..end]
    }
}

impl ToBytes for MaskObject {
    fn buffer_length(&self) -> usize {
        NUMBERS_FIELD.end + self.config.bytes_per_number() * self.data.len()
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        let mut writer = MaskObjectBuffer::new_unchecked(buffer.as_mut());
        self.config.to_bytes(&mut writer.config_mut());
        writer.set_numbers(self.data.len() as u32);

        let mut data = writer.data_mut();
        let bytes_per_number = self.config.bytes_per_number();

        for int in self.data.iter() {
            // FIXME: this allocates a vec which is sub-optimal. See
            // https://github.com/rust-num/num-bigint/issues/152
            let bytes = int.to_bytes_le();
            // This may panic if the data is invalid and contains
            // integers that are bigger than what is expected by the
            // configuration.
            data[..bytes.len()].copy_from_slice(&bytes[..]);
            // padding
            for b in data.iter_mut().take(bytes_per_number).skip(bytes.len()) {
                *b = 0;
            }
            data = &mut data[bytes_per_number..];
        }
    }
}

impl FromBytes for MaskObject {
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MaskObjectBuffer::new(buffer.as_ref())?;

        let config = MaskConfig::from_bytes(&reader.config())?;
        let mut data = Vec::with_capacity(reader.numbers() as usize);
        let bytes_per_number = config.bytes_per_number();
        for chunk in reader.data().chunks(bytes_per_number) {
            data.push(BigUint::from_bytes_le(chunk));
        }

        Ok(MaskObject { data, config })
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mask::{BoundType, DataType, GroupType, MaskConfig, ModelType};

    pub fn object() -> MaskObject {
        // config.order() = 20_000_000_000_001 with this config, so the data
        // should be stored on 6 bytes.
        let config = MaskConfig {
            group_type: GroupType::Integer,
            data_type: DataType::I32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };
        // 4 weights, each stored on 6 bytes => 24 bytes.
        let data = vec![
            BigUint::from(1_u8),
            BigUint::from(2_u8),
            BigUint::from(3_u8),
            BigUint::from(4_u8),
        ];
        MaskObject::new(config, data)
    }

    pub fn bytes() -> Vec<u8> {
        vec![
            0x00, 0x02, 0x00, 0x03, // config
            0x00, 0x00, 0x00, 0x04, // number of elements
            // data
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // 1
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, // 2
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, // 3
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, // 4
        ]
    }

    #[test]
    fn serialize() {
        let mut buf = vec![0xff; 32];
        object().to_bytes(&mut buf);
        assert_eq!(buf, bytes());
    }

    #[test]
    fn deserialize() {
        assert_eq!(MaskObject::from_bytes(&bytes()).unwrap(), object());
    }
}
