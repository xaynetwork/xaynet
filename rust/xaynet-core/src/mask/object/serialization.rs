//! Serialization of masked objects.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: ../index.html

use std::{convert::TryInto, ops::Range};

use anyhow::{anyhow, Context};
use num::bigint::BigUint;

use crate::{
    mask::{
        config::{serialization::MASK_CONFIG_BUFFER_LEN, MaskConfig},
        object::{MaskMany, MaskObject, MaskOne},
    },
    message::{
        traits::{FromBytes, ToBytes},
        utils::{range, ChunkableIterator},
        DecodeError,
    },
};

const MASK_CONFIG_FIELD: Range<usize> = range(0, MASK_CONFIG_BUFFER_LEN);
const NUMBERS_FIELD: Range<usize> = range(MASK_CONFIG_FIELD.end, 4);

// target dependent maximum number of mask object elements
#[cfg(target_pointer_width = "16")]
const MAX_NB: u32 = u16::MAX as u32;

/// A buffer for serialized mask objects.
pub struct MaskManyBuffer<T> {
    inner: T,
}

pub struct MaskObjectBuffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> MaskObjectBuffer<T> {
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        // TODO complete implementation in later refactoring
        Ok(buffer)
    }
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }
    pub fn _check_buffer_length(&self) -> Result<(), DecodeError> {
        // TODO complete implementation in later refactoring
        Ok(())
    }
    pub fn vector(&self) -> &[u8] {
        let len = self.scalar_offset();
        &self.inner.as_ref()[0..len]
    }
    pub fn scalar_offset(&self) -> usize {
        let vector = MaskManyBuffer::new_unchecked(&self.inner.as_ref()[0..]);
        vector.len()
    }
    pub fn scalar(&self) -> &[u8] {
        let offset = self.scalar_offset();
        &self.inner.as_ref()[offset..]
    }
    pub fn len(&self) -> usize {
        let scalar_offset = self.scalar_offset();
        let scalar = MaskManyBuffer::new_unchecked(&self.inner.as_ref()[scalar_offset..]);
        scalar_offset + scalar.len()
    }
}

#[allow(clippy::len_without_is_empty)]
impl<T: AsRef<[u8]>> MaskManyBuffer<T> {
    /// Creates a new buffer from `bytes`.
    ///
    /// # Errors
    /// Fails if the `bytes` don't conform to the required buffer length for mask objects.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid MaskMany")?;
        Ok(buffer)
    }

    /// Creates a new buffer from `bytes`.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Checks if this buffer conforms to the required buffer length for mask objects.
    ///
    /// # Errors
    /// Fails if the buffer is too small.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        if len < NUMBERS_FIELD.end {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                NUMBERS_FIELD.end
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
    /// similar to [`len`] but cannot panic.
    fn try_len(&self) -> Result<usize, DecodeError> {
        let config =
            MaskConfig::from_byte_slice(&self.config()).context("invalid MaskObject buffer")?;
        let bytes_per_number = config.bytes_per_number();
        let (data_length, overflows) = self.numbers().overflowing_mul(bytes_per_number);
        if overflows {
            return Err(anyhow!(
                "invalid MaskObject buffer: invalid masking config or numbers field"
            ));
        }
        Ok(NUMBERS_FIELD.end + data_length)
    }

    /// Gets the expected number of bytes of this buffer wrt to the masking configuration.
    ///
    /// # Panics
    /// Panics if the serialized masking configuration is invalid.
    pub fn len(&self) -> usize {
        let config = MaskConfig::from_byte_slice(&self.config()).unwrap();
        let bytes_per_number = config.bytes_per_number();
        let data_length = self.numbers() * bytes_per_number;
        NUMBERS_FIELD.end + data_length
    }

    /// Gets the number of serialized mask object elements.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    ///
    /// Panics if the number can't be represented as usize on targets smaller than 32 bits.
    pub fn numbers(&self) -> usize {
        // UNWRAP SAFE: the slice is exactly 4 bytes long
        let nb = u32::from_be_bytes(self.inner.as_ref()[NUMBERS_FIELD].try_into().unwrap());

        // smaller targets than 32 bits are currently not of interest
        #[cfg(target_pointer_width = "16")]
        if nb > MAX_NB {
            panic!("16 bit targets or smaller are currently not fully supported")
        }

        nb as usize
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
    pub fn vector_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[0..]
    }
    pub fn scalar_mut(&mut self) -> &mut [u8] {
        let offset = self.scalar_offset();
        &mut self.inner.as_mut()[offset..]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> MaskManyBuffer<T> {
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

impl ToBytes for MaskMany {
    fn buffer_length(&self) -> usize {
        NUMBERS_FIELD.end + self.config.bytes_per_number() * self.data.len()
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        let mut writer = MaskManyBuffer::new_unchecked(buffer.as_mut());
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

impl FromBytes for MaskMany {
    fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MaskManyBuffer::new(buffer.as_ref())?;

        let config = MaskConfig::from_byte_slice(&reader.config())?;
        let mut data = Vec::with_capacity(reader.numbers());
        let bytes_per_number = config.bytes_per_number();
        for chunk in reader.data().chunks(bytes_per_number) {
            data.push(BigUint::from_bytes_le(chunk));
        }

        Ok(MaskMany { data, config })
    }

    fn from_byte_stream<I: Iterator<Item = u8> + ExactSizeIterator>(
        iter: &mut I,
    ) -> Result<Self, DecodeError> {
        let config = MaskConfig::from_byte_stream(iter)?;
        if iter.len() < 4 {
            return Err(anyhow!("byte stream exhausted"));
        }
        let numbers = u32::from_byte_stream(iter)
            .context("failed to parse the number of item in mask object")?;
        let bytes_per_number = config.bytes_per_number();

        let data_len = numbers as usize * bytes_per_number;
        if iter.len() < data_len {
            return Err(anyhow!(
                "mask object is {} bytes long but byte stream only has {} bytes",
                data_len,
                iter.len()
            ));
        }

        let mut data = Vec::with_capacity(numbers as usize);
        let mut buf = vec![0; bytes_per_number];
        for chunk in iter.take(data_len).chunks(bytes_per_number).into_iter() {
            for (i, b) in chunk.enumerate() {
                buf[i] = b;
            }
            data.push(BigUint::from_bytes_le(buf.as_slice()));
        }

        Ok(MaskObject { data, config })
    }
}

impl ToBytes for MaskOne {
    fn buffer_length(&self) -> usize {
        MaskMany::from(self).buffer_length()
    }

    fn to_bytes<T: AsMut<[u8]> + AsRef<[u8]>>(&self, buffer: &mut T) {
        MaskMany::from(self).to_bytes(buffer)
    }
}

impl FromBytes for MaskOne {
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        // TODO more direct implementation in later refactoring
        let mut mask_many = MaskMany::from_bytes(buffer)?;
        let vec_len = mask_many.data.len();
        if vec_len == 1 {
            Ok(MaskOne::new(mask_many.config, mask_many.data.remove(0)))
        } else {
            Err(anyhow!(
                "invalid data length: expected 1 but got {}",
                vec_len
            ))
        }
    }
}

impl ToBytes for MaskObject {
    fn buffer_length(&self) -> usize {
        self.vector.buffer_length() + self.scalar.buffer_length()
    }

    fn to_bytes<T: AsMut<[u8]> + AsRef<[u8]>>(&self, buffer: &mut T) {
        let mut writer = MaskObjectBuffer::new_unchecked(buffer.as_mut());
        self.vector.to_bytes(&mut writer.vector_mut());
        self.scalar.to_bytes(&mut writer.scalar_mut());
    }
}

impl FromBytes for MaskObject {
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MaskObjectBuffer::new(buffer.as_ref())?;
        let vector = MaskMany::from_bytes(&reader.vector()).context("invalid vector part")?;
        let scalar = MaskOne::from_bytes(&reader.scalar()).context("invalid scalar part")?;
        Ok(Self { vector, scalar })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mask::{
        config::{BoundType, DataType, GroupType, MaskConfig, ModelType},
        MaskObject,
    };

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
        let data_n = vec![
            BigUint::from(1_u8),
            BigUint::from(2_u8),
            BigUint::from(3_u8),
            BigUint::from(4_u8),
        ];
        let data_1 = BigUint::from(1_u8);
        let many = MaskMany::new(config, data_n);
        let one = MaskOne::new(config, data_1);
        MaskObject::new(many, one)
    }

    pub fn bytes() -> Vec<u8> {
        vec![
            // vector part
            0x00, 0x02, 0x00, 0x03, // config
            0x00, 0x00, 0x00, 0x04, // number of elements
            // data
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // 1
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, // 2
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, // 3
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, // 4
            // scalar part
            0x00, 0x02, 0x00, 0x03, // config
            0x00, 0x00, 0x00, 0x01, // number of elements
            // data
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // 1
        ]
    }

    pub fn object_1() -> MaskMany {
        // config.order() = 20_000_000_000_001 with this config, so the data
        // should be stored on 6 bytes.
        let config = MaskConfig {
            group_type: GroupType::Integer,
            data_type: DataType::I32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };
        // 1 weight => 6 bytes
        let data = vec![BigUint::from(1_u8)];
        MaskMany::new(config, data)
    }

    pub fn bytes_1() -> Vec<u8> {
        vec![
            0x00, 0x02, 0x00, 0x03, // config
            0x00, 0x00, 0x00, 0x01, // number of elements
            // data
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // 1
        ]
    }

    #[test]
    fn serialize() {
        let mut buf = vec![0xff; 46];
        object().to_bytes(&mut buf);
        assert_eq!(buf, bytes());
    }

    #[test]
    fn deserialize() {
        assert_eq!(MaskObject::from_byte_slice(&bytes()).unwrap(), object());
    }

    #[test]
    fn deserialize_stream() {
        assert_eq!(
            MaskObject::from_byte_stream(&mut bytes().into_iter()).unwrap(),
            object()
        );
    }

    #[test]
    fn serialize_1() {
        let mut buf = vec![0xff; 14];
        object_1().to_bytes(&mut buf);
        assert_eq!(buf, bytes_1());
    }

    #[test]
    fn deserialize_1() {
        assert_eq!(MaskMany::from_byte_slice(&bytes_1()).unwrap(), object_1());
    }

    #[test]
    fn deserialize_stream_1() {
        assert_eq!(
            MaskObject::from_byte_stream(&mut bytes_1().into_iter()).unwrap(),
            object_1()
        );
    }
}
