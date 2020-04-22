pub mod sum;
pub mod sum2;
pub mod update;

use std::{
    mem,
    ops::{Range, RangeFrom, RangeTo},
};

use sodiumoxide::crypto::{box_, sign};

#[repr(u8)]
/// Message tags.
enum Tag {
    None,
    Sum,
    Update,
    Sum2,
}

/// Get the number of bytes of a signature field.
const SIGNATURE_BYTES: usize = sign::SIGNATUREBYTES;

/// Get the number of bytes of a message tag field.
const TAG_BYTES: usize = 1;

/// Get the number of bytes of a public key field.
const PK_BYTES: usize = box_::PUBLICKEYBYTES;

/// Get the number of bytes of a length field.
const LEN_BYTES: usize = mem::size_of::<usize>();

trait MessageBuffer {
    /// Get the range of the signature field.
    const SIGNATURE_RANGE: RangeTo<usize> = ..SIGNATURE_BYTES;

    /// Get the range of the message field.
    const MESSAGE_RANGE: RangeFrom<usize> = Self::SIGNATURE_RANGE.end..;

    /// Get the range of the tag field.
    const TAG_RANGE: Range<usize> =
        Self::SIGNATURE_RANGE.end..Self::SIGNATURE_RANGE.end + TAG_BYTES;

    /// Get the range of the coordinator public key field.
    const COORD_PK_RANGE: Range<usize> = Self::TAG_RANGE.end..Self::TAG_RANGE.end + PK_BYTES;

    /// Get the range of the participant public key field.
    const PART_PK_RANGE: Range<usize> =
        Self::COORD_PK_RANGE.end..Self::COORD_PK_RANGE.end + PK_BYTES;

    /// Get the range of the sum signature field.
    const SUM_SIGNATURE_RANGE: Range<usize> =
        Self::PART_PK_RANGE.end..Self::PART_PK_RANGE.end + SIGNATURE_BYTES;

    /// Get a reference to the message buffer.
    fn bytes(&'_ self) -> &'_ [u8];

    /// Get a mutable reference to the message buffer.
    fn bytes_mut(&mut self) -> &mut [u8];

    /// Get the length of the message buffer.
    fn len(&self) -> usize {
        self.bytes().len()
    }

    /// Get a reference to the signature field.
    fn signature(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::SIGNATURE_RANGE]
    }

    /// Get a mutable reference to the signature field.
    fn signature_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[Self::SIGNATURE_RANGE]
    }

    /// Get a reference to the message field.
    fn message(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::MESSAGE_RANGE]
    }

    /// Get a reference to the tag field.
    fn tag(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::TAG_RANGE]
    }

    /// Get a mutable reference to the tag field.
    fn tag_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[Self::TAG_RANGE]
    }

    /// Get a reference to the coordinator public key field.
    fn coord_pk(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::COORD_PK_RANGE]
    }

    /// Get a mutable reference to the coordinator public key field.
    fn coord_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[Self::COORD_PK_RANGE]
    }

    /// Get a reference to the participant public key field.
    fn part_pk(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::PART_PK_RANGE]
    }

    /// Get a mutable reference to the participant public key field.
    fn part_pk_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[Self::PART_PK_RANGE]
    }

    /// Get a reference to the sum signature field.
    fn sum_signature(&'_ self) -> &'_ [u8] {
        &self.bytes()[Self::SUM_SIGNATURE_RANGE]
    }

    /// Get a mutable reference to the sum signature field.
    fn sum_signature_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[Self::SUM_SIGNATURE_RANGE]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        // just to make sure that the constants were not changed accidentally, because a lot of
        // assumptions are based on those
        assert_eq!(SIGNATURE_BYTES, sign::SIGNATUREBYTES);
        assert_eq!(TAG_BYTES, 1);
        assert_eq!(PK_BYTES, box_::PUBLICKEYBYTES);
        assert_eq!(PK_BYTES, sign::PUBLICKEYBYTES);
        assert_eq!(LEN_BYTES, mem::size_of::<usize>());
    }
}
