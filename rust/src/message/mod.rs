//! Messages.

pub(crate) mod buffer;
pub(crate) mod header;
#[allow(clippy::module_inception)]
pub(crate) mod message;
pub(crate) mod payload;
pub(crate) mod traits;
pub(crate) mod utils;

pub use self::{
    buffer::MessageBuffer,
    header::{Flags, Header, HeaderOwned, Tag},
    message::{Message, MessageOpen, MessageOwned, MessageSeal},
    payload::{
        sum::{Sum, SumBuffer, SumOwned},
        sum2::{Sum2, Sum2Buffer, Sum2Owned},
        update::{Update, UpdateBuffer, UpdateOwned},
        Payload,
        PayloadOwned,
    },
    traits::{FromBytes, LengthValueBuffer, ToBytes},
};

/// An error that signals a failure when trying to decrypt and parse a message.
///
/// This is kept generic on purpose to not reveal to the sender what specifically failed during
/// decryption or parsing.
pub type DecodeError = anyhow::Error;
