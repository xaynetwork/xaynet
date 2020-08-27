//! Message payloads.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: ../index.html

pub(crate) mod sum;
pub(crate) mod sum2;
pub(crate) mod update;

use derive_more::From;

use crate::message::{
    payload::{sum::Sum, sum2::Sum2, update::Update},
    traits::ToBytes,
};

/// The payload of a [`Message`].
///
/// [`Message`]: struct.Message.html
#[derive(From, Eq, PartialEq, Debug, Clone)]
pub enum Payload {
    /// The payload of a [`Sum`] message.
    Sum(Sum),
    /// The payload of an [`Update`] message.
    Update(Update),
    /// The payload of a [`Sum2`] message.
    Sum2(Sum2),
}

impl ToBytes for Payload {
    fn buffer_length(&self) -> usize {
        match self {
            Payload::Sum(m) => m.buffer_length(),
            Payload::Sum2(m) => m.buffer_length(),
            Payload::Update(m) => m.buffer_length(),
        }
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        match self {
            Payload::Sum(m) => m.to_bytes(buffer),
            Payload::Sum2(m) => m.to_bytes(buffer),
            Payload::Update(m) => m.to_bytes(buffer),
        }
    }
}
