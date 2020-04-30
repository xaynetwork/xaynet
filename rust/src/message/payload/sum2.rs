use std::{borrow::Borrow, ops::Range};

use crate::{
    mask::Mask,
    message::{utils::range, DecodeError, FromBytes, LengthValueBuffer, ToBytes},
    ParticipantTaskSignature,
};
use anyhow::{anyhow, Context};

const SUM_SIGNATURE_RANGE: Range<usize> = range(0, ParticipantTaskSignature::LENGTH);

/// A wrapper around a buffer that contains a sum2 message. It provides
/// getters and setters to access the different fields of the message
/// safely.
///
/// # Examples
///
/// Decoding a sum2 message:
///
/// ```rust
/// # use xain_fl::message::Sum2Buffer;
/// let signature = vec![0x11; 64];
/// let mask = vec![
///     0x00, 0x00, 0x00, 0x08, // Length 8
///     0x00, 0x01, 0x02, 0x03, // Value: 0, 1, 2, 3
/// ];
/// let bytes = [signature.as_slice(), mask.as_slice()].concat();
///
/// let buffer = Sum2Buffer::new(&bytes).unwrap();
/// assert_eq!(buffer.sum_signature(), &bytes[..64]);
/// assert_eq!(buffer.mask(), &bytes[64..]);
/// ```
///
/// Encoding a sum2 message:
///
/// ```rust
/// # use xain_fl::message::Sum2Buffer;
/// let signature = vec![0x11; 64];
/// let mask = vec![
///     0x00, 0x00, 0x00, 0x08, // Length 8
///     0x00, 0x01, 0x02, 0x03, // Value: 0, 1, 2, 3
/// ];
/// let mut bytes = vec![0xff; 72];
/// {
///     let mut buffer = Sum2Buffer::new_unchecked(&mut bytes);
///     buffer.sum_signature_mut().copy_from_slice(&signature[..]);
///     buffer.mask_mut().copy_from_slice(&mask[..]);
/// }
/// assert_eq!(&bytes[..64], &signature[..]);
/// assert_eq!(&bytes[64..], &mask[..]);
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Sum2Buffer<T> {
    inner: T,
}

impl<T: AsRef<[u8]>> Sum2Buffer<T> {
    /// Perform bound checks for the various message fields on `bytes`
    /// and return a new `Sum2Buffer`.
    pub fn new(bytes: T) -> Result<Self, DecodeError> {
        let buffer = Self { inner: bytes };
        buffer
            .check_buffer_length()
            .context("not a valid Sum2Buffer")?;
        Ok(buffer)
    }

    /// Return a `Sum2Buffer` with the given `bytes` without
    /// performing bound checks. This means that accessing the message
    /// fields may panic.
    pub fn new_unchecked(bytes: T) -> Self {
        Self { inner: bytes }
    }

    /// Perform bound checks for the various message fields on this
    /// buffer.
    pub fn check_buffer_length(&self) -> Result<(), DecodeError> {
        let len = self.inner.as_ref().len();
        if len < SUM_SIGNATURE_RANGE.end {
            return Err(anyhow!(
                "invalid buffer length: {} < {}",
                len,
                SUM_SIGNATURE_RANGE.end
            ));
        }

        LengthValueBuffer::new(&self.inner.as_ref()[SUM_SIGNATURE_RANGE.end..])?;
        Ok(())
    }
}

impl<T: AsMut<[u8]>> Sum2Buffer<T> {
    /// Get a mutable reference to the sum signature field
    ///
    /// # Panic
    ///
    /// This may panic if the underlying buffer does not represent a
    /// valid sum2 message. If `self.check_buffer_length()` returned
    /// `Ok(())` this method is guaranteed not to panic.
    pub fn sum_signature_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[SUM_SIGNATURE_RANGE]
    }

    /// Get a mutable reference to the mask field
    ///
    /// # Panic
    ///
    /// This may panic if the underlying buffer does not represent a
    /// valid sum2 message. If `self.check_buffer_length()` returned
    /// `Ok(())` this method is guaranteed not to panic.
    pub fn mask_mut(&mut self) -> &mut [u8] {
        &mut self.inner.as_mut()[SUM_SIGNATURE_RANGE.end..]
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Sum2Buffer<&'a T> {
    /// Get a reference to the sum signature field
    ///
    /// # Panic
    ///
    /// This may panic if the underlying buffer does not represent a
    /// valid sum2 message. If `self.check_buffer_length()` returned
    /// `Ok(())` this method is guaranteed not to panic.
    pub fn sum_signature(&self) -> &'a [u8] {
        &self.inner.as_ref()[SUM_SIGNATURE_RANGE]
    }

    /// Get a reference to the mask field
    ///
    /// # Panic
    ///
    /// This may panic if the underlying buffer does not represent a
    /// valid sum2 message. If `self.check_buffer_length()` returned
    /// `Ok(())` this method is guaranteed not to panic.
    pub fn mask(&self) -> &'a [u8] {
        &self.inner.as_ref()[SUM_SIGNATURE_RANGE.end..]
    }
}

/// High level representation of a sum2 message. These messages are
/// sent by sum participants during the sum2 phase.
///
/// # Examples
///
/// ## Decoding a message
///
/// ```rust
/// # use xain_fl::{crypto::ByteObject, message::{FromBytes, Sum2Owned}, ParticipantTaskSignature, mask::Mask};
/// let signature = vec![0x11; 64];
/// let mask = vec![
///     0x00, 0x00, 0x00, 0x08, // Length 8
///     0x00, 0x01, 0x02, 0x03, // Value: 0, 1, 2, 3
/// ];
/// let bytes = [signature.as_slice(), mask.as_slice()].concat();
/// let parsed = Sum2Owned::from_bytes(&bytes).unwrap();
/// let expected = Sum2Owned {
///     sum_signature: ParticipantTaskSignature::from_slice(&bytes[..64]).unwrap(),
///     mask: Mask::from(&[0, 1, 2, 3][..]),
/// };
/// assert_eq!(parsed, expected);
/// ```
///
/// ## Encoding a message
///
/// ```rust
/// # use xain_fl::{crypto::ByteObject, message::{ToBytes, Sum2Owned}, ParticipantTaskSignature, mask::Mask};
/// let signature = vec![0x11; 64];
/// let mask = vec![
///     0x00, 0x00, 0x00, 0x08, // Length 8
///     0x00, 0x01, 0x02, 0x03, // Value: 0, 1, 2, 3
/// ];
/// let bytes = [signature.as_slice(), mask.as_slice()].concat();
///
/// let sum_signature = ParticipantTaskSignature::from_slice(&bytes[..64]).unwrap();
/// let mask = Mask::from(&[0, 1, 2, 3][..]);
/// let sum2 = Sum2Owned {
///     sum_signature,
///     mask,
/// };
/// // we need a 72 bytes long buffer to serialize that message
/// assert_eq!(sum2.buffer_length(), 72);
/// let mut buf = vec![0xff; 72];
/// sum2.to_bytes(&mut buf);
/// assert_eq!(bytes, buf);
/// ```
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Sum2<M> {
    /// Signature of the word "sum", using the participant's secret
    /// signing key. This is used by the coordinator to verify that
    /// the participant has been selected to perform the sum task.
    pub sum_signature: ParticipantTaskSignature,

    /// Mask computed by the participant.
    pub mask: M,
}

impl<M> ToBytes for Sum2<M>
where
    M: Borrow<Mask>,
{
    fn buffer_length(&self) -> usize {
        SUM_SIGNATURE_RANGE.end + self.mask.borrow().buffer_length()
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        let mut writer = Sum2Buffer::new_unchecked(buffer.as_mut());
        self.sum_signature.to_bytes(&mut writer.sum_signature_mut());
        self.mask.borrow().to_bytes(&mut writer.mask_mut());
    }
}

/// Owned version of a [`Sum2`]
pub type Sum2Owned = Sum2<Mask>;

impl FromBytes for Sum2Owned {
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = Sum2Buffer::new(buffer.as_ref())?;
        Ok(Self {
            sum_signature: ParticipantTaskSignature::from_bytes(&reader.sum_signature())
                .context("invalid sum signature")?,
            mask: Mask::from_bytes(&reader.mask()).context("invalid mask")?,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{crypto::ByteObject, mask::Mask};

    fn signature_bytes() -> Vec<u8> {
        vec![0x99; ParticipantTaskSignature::LENGTH]
    }

    fn mask_bytes() -> Vec<u8> {
        vec![
            0x00, 0x00, 0x00, 0x08, // Length 8
            0x00, 0x01, 0x02, 0x03, // Value: 0, 1, 2, 3
        ]
    }

    fn sum2_bytes() -> Vec<u8> {
        let mut bytes = signature_bytes();
        bytes.extend(mask_bytes());
        bytes
    }

    fn sum2() -> Sum2Owned {
        let sum_signature = ParticipantTaskSignature::from_slice(&signature_bytes()[..]).unwrap();
        let mask = Mask::from(&[0, 1, 2, 3][..]);
        Sum2Owned {
            sum_signature,
            mask,
        }
    }

    #[test]
    fn buffer_read() {
        let bytes = sum2_bytes();
        let buffer = Sum2Buffer::new(&bytes).unwrap();
        assert_eq!(buffer.sum_signature(), &signature_bytes()[..]);
        assert_eq!(buffer.mask(), &mask_bytes()[..]);
    }

    #[test]
    fn buffer_new_invalid() {
        let mut bytes = sum2_bytes();
        assert!(Sum2Buffer::new(&bytes[1..]).is_err());
        // make the length field for the mask invalid
        bytes[66] = 1;
        assert!(Sum2Buffer::new(&bytes[..]).is_err());
    }

    #[test]
    fn buffer_write() {
        let mut bytes = vec![0xff; 72];
        {
            let mut buffer = Sum2Buffer::new_unchecked(&mut bytes);
            buffer
                .sum_signature_mut()
                .copy_from_slice(&signature_bytes()[..]);
            buffer.mask_mut().copy_from_slice(&mask_bytes()[..]);
        }
        assert_eq!(&bytes[..], &sum2_bytes()[..]);
    }

    #[test]
    fn encode() {
        let message = sum2();
        assert_eq!(message.buffer_length(), sum2_bytes().len());

        let mut buf = vec![0xff; message.buffer_length()];
        message.to_bytes(&mut buf);
        assert_eq!(buf, sum2_bytes());
    }

    #[test]
    fn decode() {
        let bytes = sum2_bytes();
        let parsed = Sum2Owned::from_bytes(&bytes).unwrap();
        let expected = sum2();
        assert_eq!(parsed, expected);
    }
}
