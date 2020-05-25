use anyhow::{anyhow, Context};
use std::{convert::TryInto, ops::Range};

use num::bigint::BigUint;

use super::MaskObject;
use crate::{
    mask::{config::serialization::MASK_CONFIG_BUFFER_LEN, MaskConfig},
    message::{utils::range, DecodeError, FromBytes, ToBytes},
};

const MASK_CONFIG_FIELD: Range<usize> = range(0, MASK_CONFIG_BUFFER_LEN);
const DIGITS_FIELD: Range<usize> = range(MASK_CONFIG_FIELD.end, 4);

pub struct MaskObjectBuffer<T> {
    inner: T,
}

#[allow(clippy::len_without_is_empty)]
impl<T: AsRef<[u8]>> MaskObjectBuffer<T> {
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid MaskObject")?;
        Ok(buffer)
    }

    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        if len < DIGITS_FIELD.end {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                DIGITS_FIELD.end
            ));
        }

        let config = MaskConfig::from_bytes(&self.config()).context("invalid MaskObject buffer")?;
        let bytes_per_digit = config.bytes_per_digit();
        let (data_length, overflows) = (self.digits() as usize).overflowing_mul(bytes_per_digit);
        if overflows {
            return Err(anyhow!(
                "invalid MaskObject buffer: invalid mask config or digits field"
            ));
        }
        let total_expected_length = DIGITS_FIELD.end + data_length;
        if len < total_expected_length {
            return Err(anyhow!(
                "invalid buffer length: expected {} bytes but buffer has only {} bytes",
                total_expected_length,
                len
            ));
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        let config = MaskConfig::from_bytes(&self.config()).unwrap();
        let bytes_per_digit = config.bytes_per_digit();
        let data_length = self.digits() as usize * bytes_per_digit;
        DIGITS_FIELD.end + data_length
    }

    pub fn digits(&self) -> u32 {
        // UNWRAP SAFE: the slice is exactly 4 bytes long
        u32::from_be_bytes(self.inner.as_ref()[DIGITS_FIELD].try_into().unwrap())
    }

    pub fn config(&self) -> &[u8] {
        &self.inner.as_ref()[MASK_CONFIG_FIELD]
    }

    pub fn data(&self) -> &[u8] {
        &self.inner.as_ref()[DIGITS_FIELD.end..self.len()]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> MaskObjectBuffer<T> {
    pub fn set_digits(&mut self, value: u32) {
        self.inner.as_mut()[DIGITS_FIELD].copy_from_slice(&value.to_be_bytes());
    }
    pub fn config_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[MASK_CONFIG_FIELD]
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        let end = self.len();
        &mut self.inner.as_mut()[DIGITS_FIELD.end..end]
    }
}

impl ToBytes for MaskObject {
    fn buffer_length(&self) -> usize {
        DIGITS_FIELD.end + self.config.bytes_per_digit() * self.data.len()
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        let mut writer = MaskObjectBuffer::new_unchecked(buffer.as_mut());
        self.config.to_bytes(&mut writer.config_mut());
        writer.set_digits(self.data.len() as u32);

        let mut data = writer.data_mut();
        let bytes_per_digit = self.config.bytes_per_digit();

        for int in self.data.iter() {
            // FIXME: this allocates a vec which is sub-optimal. See
            // https://github.com/rust-num/num-bigint/issues/152
            let bytes = int.to_bytes_le();
            // This may panic if the data is invalid and contains
            // integers that are bigger than what is expected by the
            // configuration.
            data[..bytes.len()].copy_from_slice(&bytes[..]);
            // padding
            for b in data.iter_mut().take(bytes_per_digit).skip(bytes.len()) {
                *b = 0;
            }
            data = &mut data[bytes_per_digit..];
        }
    }
}

impl FromBytes for MaskObject {
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MaskObjectBuffer::new(buffer.as_ref())?;

        let config = MaskConfig::from_bytes(&reader.config())?;
        let mut data = Vec::with_capacity(reader.digits() as usize);
        let bytes_per_digit = config.bytes_per_digit();
        for chunk in reader.data().chunks(bytes_per_digit) {
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
        // config.order() = 20_000 with this config, so the data
        // should be stored on 2 bytes.
        let config = MaskConfig {
            group_type: GroupType::Integer,
            data_type: DataType::I32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };
        // 4 weights, each stored on 2 bytes => 8 bytes.
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
            0x01, 0x00, // 1
            0x02, 0x00, // 2
            0x03, 0x00, // 3
            0x04, 0x00, // 4
        ]
    }

    #[test]
    fn serialize() {
        let mut buf = vec![0xff; 16];
        object().to_bytes(&mut buf);
        assert_eq!(buf, bytes());
    }

    #[test]
    fn deserialize() {
        assert_eq!(MaskObject::from_bytes(&bytes()).unwrap(), object());
    }
}
