//! Message payloads.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: crate::message

pub(crate) mod chunk;
pub(crate) mod sum;
pub(crate) mod sum2;
pub(crate) mod update;

use derive_more::From;

use crate::message::{
    payload::{chunk::Chunk, sum::Sum, sum2::Sum2, update::Update},
    traits::ToBytes,
};

/// The payload of a [`Message`].
///
/// [`Message`]: crate::message::Message
#[derive(From, Eq, PartialEq, Debug, Clone)]
pub enum Payload {
    /// The payload of a [`Sum`] message.
    Sum(Sum),
    /// The payload of an [`Update`] message.
    Update(Update),
    /// The payload of a [`Sum2`] message.
    Sum2(Sum2),
    /// The payload of a [`Chunk`] message.
    Chunk(Chunk),
}

impl Payload {
    pub fn is_sum(&self) -> bool {
        matches!(self, Self::Sum(_))
    }

    pub fn is_update(&self) -> bool {
        matches!(self, Self::Update(_))
    }

    pub fn is_sum2(&self) -> bool {
        matches!(self, Self::Sum2(_))
    }

    pub fn is_chunk(&self) -> bool {
        matches!(self, Self::Chunk(_))
    }
}

impl ToBytes for Payload {
    fn buffer_length(&self) -> usize {
        match self {
            Payload::Sum(m) => m.buffer_length(),
            Payload::Sum2(m) => m.buffer_length(),
            Payload::Update(m) => m.buffer_length(),
            Payload::Chunk(m) => m.buffer_length(),
        }
    }

    fn to_bytes<T: AsMut<[u8]> + AsRef<[u8]>>(&self, buffer: &mut T) {
        match self {
            Payload::Sum(m) => m.to_bytes(buffer),
            Payload::Sum2(m) => m.to_bytes(buffer),
            Payload::Update(m) => m.to_bytes(buffer),
            Payload::Chunk(m) => m.to_bytes(buffer),
        }
    }
}
