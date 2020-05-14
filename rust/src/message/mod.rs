pub(crate) mod utils;

mod traits;
pub use self::traits::{FromBytes, LengthValueBuffer, ToBytes};

mod buffer;
pub use self::buffer::*;

mod header;
pub use self::header::*;

pub(crate) mod payload;
pub use self::payload::*;

#[allow(clippy::module_inception)]
mod message;
pub use self::message::*;

/// Error that signals a failure when trying to decrypt and parse a
/// message
pub type DecodeError = anyhow::Error;
