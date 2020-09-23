//! Serialization of masking configurations.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: ../index.html

use std::convert::TryInto;

use anyhow::{anyhow, Context};

use crate::{
    mask::config::MaskConfig,
    message::{
        traits::{FromBytes, ToBytes},
        DecodeError,
    },
};

const GROUP_TYPE_FIELD: usize = 0;
const DATA_TYPE_FIELD: usize = 1;
const BOUND_TYPE_FIELD: usize = 2;
const MODEL_TYPE_FIELD: usize = 3;
pub(crate) const MASK_CONFIG_BUFFER_LEN: usize = 4;

/// A buffer for serialized masking configurations.
pub struct MaskConfigBuffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> MaskConfigBuffer<T> {
    /// Creates a new buffer from `bytes`.
    ///
    /// # Errors
    /// Fails if the `bytes` don't conform to the required buffer length for masking configurations.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid MaskConfigBuffer")?;
        Ok(buffer)
    }

    /// Creates a new buffer from `bytes`.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Checks if this buffer conforms to the required buffer length for masking configurations.
    ///
    /// # Errors
    /// Fails if the buffer is too small.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        if len < MASK_CONFIG_BUFFER_LEN {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                MASK_CONFIG_BUFFER_LEN
            ));
        }
        Ok(())
    }

    /// Gets the serialized group type of the masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn group_type(&self) -> u8 {
        self.inner.as_ref()[GROUP_TYPE_FIELD]
    }

    /// Gets the serialized data type of the masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn data_type(&self) -> u8 {
        self.inner.as_ref()[DATA_TYPE_FIELD]
    }

    /// Gets the serialized bound type of the masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn bound_type(&self) -> u8 {
        self.inner.as_ref()[BOUND_TYPE_FIELD]
    }

    /// Gets the serialized model type of the masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn model_type(&self) -> u8 {
        self.inner.as_ref()[MODEL_TYPE_FIELD]
    }
}

impl<T: AsMut<[u8]>> MaskConfigBuffer<T> {
    /// Sets the serialized group type of the masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn set_group_type(&mut self, value: u8) {
        self.inner.as_mut()[GROUP_TYPE_FIELD] = value;
    }

    /// Sets the serialized data type of the masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn set_data_type(&mut self, value: u8) {
        self.inner.as_mut()[DATA_TYPE_FIELD] = value;
    }

    /// Sets the serialized bound type of the masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn set_bound_type(&mut self, value: u8) {
        self.inner.as_mut()[BOUND_TYPE_FIELD] = value;
    }

    /// Sets the serialized model type of the masking configuration.
    ///
    /// # Panics
    /// May panic if this buffer is unchecked.
    pub fn set_model_type(&mut self, value: u8) {
        self.inner.as_mut()[MODEL_TYPE_FIELD] = value;
    }
}

impl ToBytes for MaskConfig {
    fn buffer_length(&self) -> usize {
        MASK_CONFIG_BUFFER_LEN
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        let mut writer = MaskConfigBuffer::new_unchecked(buffer.as_mut());
        writer.set_group_type(self.group_type as u8);
        writer.set_data_type(self.data_type as u8);
        writer.set_bound_type(self.bound_type as u8);
        writer.set_model_type(self.model_type as u8);
    }
}

impl FromBytes for MaskConfig {
    fn from_byte_slice<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MaskConfigBuffer::new(buffer.as_ref())?;
        Ok(Self {
            group_type: reader
                .group_type()
                .try_into()
                .context("invalid masking config")?,
            data_type: reader
                .data_type()
                .try_into()
                .context("invalid masking config")?,
            bound_type: reader
                .bound_type()
                .try_into()
                .context("invalid masking config")?,
            model_type: reader
                .model_type()
                .try_into()
                .context("invalid masking config")?,
        })
    }

    fn from_byte_stream<I: Iterator<Item = u8> + ExactSizeIterator>(
        iter: &mut I,
    ) -> Result<Self, DecodeError> {
        let buf: Vec<u8> = iter.take(MASK_CONFIG_BUFFER_LEN).collect();
        Self::from_byte_slice(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::config::{BoundType, DataType, GroupType, MaskConfig, ModelType};

    #[test]
    fn serialize() {
        let config = MaskConfig {
            group_type: GroupType::Prime,
            data_type: DataType::F64,
            bound_type: BoundType::Bmax,
            model_type: ModelType::M9,
        };

        let mut buf = vec![0xff; 4];
        config.to_bytes(&mut buf);
        assert_eq!(buf, vec![1, 1, 255, 9]);
    }

    #[test]
    fn deserialize() {
        let bytes = vec![1, 1, 255, 9];
        let config = MaskConfig::from_byte_slice(&bytes).unwrap();
        assert_eq!(
            config,
            MaskConfig {
                group_type: GroupType::Prime,
                data_type: DataType::F64,
                bound_type: BoundType::Bmax,
                model_type: ModelType::M9,
            }
        );
    }

    #[test]
    fn stream_deserialize() {
        let mut bytes = vec![1, 1, 255, 9].into_iter();
        let config = MaskConfig::from_byte_stream(&mut bytes).unwrap();
        assert_eq!(
            config,
            MaskConfig {
                group_type: GroupType::Prime,
                data_type: DataType::F64,
                bound_type: BoundType::Bmax,
                model_type: ModelType::M9,
            }
        );
    }
}
