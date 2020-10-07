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
        object::{MaskObject, MaskUnit, MaskVect},
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

/// A buffer for serialized `MaskMany`s.
pub struct MaskManyBuffer<T> {
    inner: T,
}

/// A buffer for serialized mask objects.
pub struct MaskObjectBuffer<T> {
    inner: T,
}

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
    /// Fails if the buffer is too small.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let inner = self.inner.as_ref();
        // check length of vector field
        MaskManyBuffer::new(&inner[0..]).context("invalid vector field")?;
        // check length of scalar field
        // TODO possible change to MaskOneBuffer in the future once implemented
        MaskManyBuffer::new(&inner[self.scalar_offset()..]).context("invalid scalar field")?;
        Ok(())
    }

    /// Gets the vector part.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn vector(&self) -> &[u8] {
        let len = self.scalar_offset();
        &self.inner.as_ref()[0..len]
    }

    /// Gets the offset of the scalar field.
    pub fn scalar_offset(&self) -> usize {
        let vector = MaskManyBuffer::new_unchecked(&self.inner.as_ref()[0..]);
        vector.len()
    }

    /// Gets the scalar part.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn scalar(&self) -> &[u8] {
        let offset = self.scalar_offset();
        &self.inner.as_ref()[offset..]
    }

    /// Gets the expected number of bytes of this buffer.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
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

impl ToBytes for MaskVect {
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

impl FromBytes for MaskVect {
    fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MaskManyBuffer::new(buffer.as_ref())?;

        let config = MaskConfig::from_byte_slice(&reader.config())?;
        let mut data = Vec::with_capacity(reader.numbers());
        let bytes_per_number = config.bytes_per_number();
        for chunk in reader.data().chunks(bytes_per_number) {
            data.push(BigUint::from_bytes_le(chunk));
        }

        Ok(MaskVect { data, config })
    }

    fn from_byte_stream<I: Iterator<Item = u8> + ExactSizeIterator>(
        iter: &mut I,
    ) -> Result<Self, DecodeError> {
        let config = MaskConfig::from_byte_stream(iter)?;
        if iter.len() < 4 {
            return Err(anyhow!("byte stream exhausted"));
        }
        let numbers = u32::from_byte_stream(iter)
            .context("failed to parse the number of items in mask object")?;
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

        Ok(MaskVect { data, config })
    }
}

impl ToBytes for MaskUnit {
    fn buffer_length(&self) -> usize {
        MaskVect::from(self).buffer_length()
    }

    fn to_bytes<T: AsMut<[u8]> + AsRef<[u8]>>(&self, buffer: &mut T) {
        MaskVect::from(self).to_bytes(buffer)
    }
}

impl FromBytes for MaskUnit {
    fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        // TODO more direct implementation in later refactoring
        let mut mask_vect = MaskVect::from_byte_slice(buffer)?;
        let vec_len = mask_vect.data.len();
        if vec_len == 1 {
            Ok(MaskUnit::new(mask_vect.config, mask_vect.data.remove(0)))
        } else {
            Err(anyhow!(
                "invalid data length: expected 1 but got {}",
                vec_len
            ))
        }
    }

    fn from_byte_stream<I: Iterator<Item = u8> + ExactSizeIterator>(
        iter: &mut I,
    ) -> Result<Self, DecodeError> {
        let mut mask_vect = MaskVect::from_byte_stream(iter)?;
        let vec_len = mask_vect.data.len();
        if vec_len == 1 {
            Ok(MaskUnit::new(mask_vect.config, mask_vect.data.remove(0)))
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
    fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MaskObjectBuffer::new(buffer.as_ref())?;
        let vector = MaskVect::from_byte_slice(&reader.vector()).context("invalid vector part")?;
        let scalar = MaskUnit::from_byte_slice(&reader.scalar()).context("invalid scalar part")?;
        Ok(Self { vector, scalar })
    }

    fn from_byte_stream<I: Iterator<Item = u8> + ExactSizeIterator>(
        iter: &mut I,
    ) -> Result<Self, DecodeError> {
        let vector = MaskVect::from_byte_stream(iter).context("invalid vector part")?;
        let scalar = MaskUnit::from_byte_stream(iter).context("invalid scalar part")?;
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

    pub fn mask_config() -> (MaskConfig, Vec<u8>) {
        // config.order() = 20_000_000_000_001 with this config, so the data
        // should be stored on 6 bytes.
        let config = MaskConfig {
            group_type: GroupType::Integer,
            data_type: DataType::I32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };
        let bytes = vec![0x00, 0x02, 0x00, 0x03];
        (config, bytes)
    }

    pub fn mask_vect() -> (MaskVect, Vec<u8>) {
        let (config, mut bytes) = mask_config();
        let data = vec![
            BigUint::from(1_u8),
            BigUint::from(2_u8),
            BigUint::from(3_u8),
            BigUint::from(4_u8),
        ];
        let mask_vect = MaskVect::new(config, data);

        bytes.extend(vec![
            // number of elements
            0x00, 0x00, 0x00, 0x04, // data (1 weight => 6 bytes with this config)
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // 1
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, // 2
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, // 3
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, // 4
        ]);

        (mask_vect, bytes)
    }

    pub fn mask_unit() -> (MaskUnit, Vec<u8>) {
        let (config, mut bytes) = mask_config();
        let data = BigUint::from(1_u8);
        let mask_unit = MaskUnit::new(config, data);

        bytes.extend(vec![
            // number of elements
            0x00, 0x00, 0x00, 0x01, // data
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // 1
        ]);
        (mask_unit, bytes)
    }

    pub fn mask_object() -> (MaskObject, Vec<u8>) {
        let (mask_vect, mask_vect_bytes) = mask_vect();
        let (mask_unit, mask_unit_bytes) = mask_unit();
        let obj = MaskObject::new(mask_vect, mask_unit);
        let bytes = [mask_vect_bytes.as_slice(), mask_unit_bytes.as_slice()].concat();

        (obj, bytes)
    }

    #[test]
    fn serialize_mask_object() {
        let (mask_object, expected) = mask_object();
        let mut buf = vec![0xff; 46];
        mask_object.to_bytes(&mut buf);
        assert_eq!(buf, expected);
    }

    #[test]
    fn deserialize_mask_object() {
        let (expected, bytes) = mask_object();
        assert_eq!(MaskObject::from_byte_slice(&&bytes[..]).unwrap(), expected);
    }

    #[test]
    fn deserialize_mask_object_from_stream() {
        let (expected, bytes) = mask_object();
        assert_eq!(
            MaskObject::from_byte_stream(&mut bytes.into_iter()).unwrap(),
            expected
        );
    }

    #[test]
    fn serialize_mask_vect() {
        let (mask_vect, expected) = mask_vect();
        let mut buf = vec![0xff; expected.len()];
        mask_vect.to_bytes(&mut buf);
        assert_eq!(buf, expected);
    }

    #[test]
    fn deserialize_mask_vect() {
        let (expected, bytes) = mask_vect();
        assert_eq!(MaskVect::from_byte_slice(&&bytes[..]).unwrap(), expected);
    }

    #[test]
    fn deserialize_mask_vect_from_stream() {
        let (expected, bytes) = mask_vect();
        assert_eq!(
            MaskVect::from_byte_stream(&mut bytes.into_iter()).unwrap(),
            expected
        );
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
