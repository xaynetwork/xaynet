//! Serialization of masked vectors.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: crate::mask

use std::{convert::TryInto, ops::Range};

use anyhow::{anyhow, Context};
use num::bigint::BigUint;

use crate::{
    mask::{
        config::{serialization::MASK_CONFIG_BUFFER_LEN, MaskConfig},
        object::MaskVect,
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

/// A buffer for serialized mask vectors.
pub struct MaskVectBuffer<T> {
    inner: T,
}

#[allow(clippy::len_without_is_empty)]
impl<T: AsRef<[u8]>> MaskVectBuffer<T> {
    /// Creates a new buffer from `bytes`.
    ///
    /// # Errors
    /// Fails if the `bytes` don't conform to the required buffer length for mask vectors.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid mask vector")?;
        Ok(buffer)
    }

    /// Creates a new buffer from `bytes`.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Checks if this buffer conforms to the required buffer length for mask vectors.
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
    /// similar to [`len()`] but cannot panic.
    ///
    /// [`len()`]: MaskVectBuffer::len
    fn try_len(&self) -> Result<usize, DecodeError> {
        let config =
            MaskConfig::from_byte_slice(&self.config()).context("invalid mask vector buffer")?;
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

    /// Gets the serialized mask vector elements.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn data(&self) -> &[u8] {
        &self.inner.as_ref()[NUMBERS_FIELD.end..self.len()]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> MaskVectBuffer<T> {
    /// Sets the number of serialized mask vector elements.
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

    /// Gets the serialized mask vector elements.
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
        let mut writer = MaskVectBuffer::new_unchecked(buffer.as_mut());
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
        let reader = MaskVectBuffer::new(buffer.as_ref())?;

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
            .context("failed to parse the number of items in mask vector")?;
        let bytes_per_number = config.bytes_per_number();

        let data_len = numbers as usize * bytes_per_number;
        if iter.len() < data_len {
            return Err(anyhow!(
                "mask vector is {} bytes long but byte stream only has {} bytes",
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::mask::object::serialization::tests::mask_config;

    pub fn mask_vect() -> (MaskVect, Vec<u8>) {
        let (config, mut bytes) = mask_config();
        let data = vec![
            BigUint::from(1_u8),
            BigUint::from(2_u8),
            BigUint::from(3_u8),
            BigUint::from(4_u8),
        ];
        let mask_vect = MaskVect::new_unchecked(config, data);

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
}
