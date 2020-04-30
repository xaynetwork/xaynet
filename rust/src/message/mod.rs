pub(crate) mod utils;

mod traits;
pub use self::traits::{FromBytes, LengthValueBuffer, ToBytes};

mod buffer;
pub use self::buffer::*;

#[repr(u8)]
/// Message tags.
enum Tag {
    #[allow(dead_code)] // None is used for tests
    None,
    Sum,
    Update,
    Sum2,
}

pub(crate) mod payload;
pub use self::payload::*;

mod message;
pub use self::message::*;

/// Error that signals a failure when trying to decrypt and parse a
/// message
pub type DecodeError = anyhow::Error;
