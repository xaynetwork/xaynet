use std::borrow::Borrow;

pub(crate) mod sum;
pub use self::sum::*;
pub(crate) mod sum2;
pub use self::sum2::*;
pub(crate) mod update;
pub use self::update::*;

use derive_more::From;

use crate::{mask::MaskObject, message::traits::ToBytes, LocalSeedDict};

/// Payload of a [`Message`]
#[derive(From, Eq, PartialEq, Clone, Debug)]
pub enum Payload<D, M, N> {
    /// Payload of a sum message
    Sum(Sum),
    /// Payload of an update message
    Update(Update<D, M>),
    /// Payload of a sum2 message
    Sum2(Sum2<N>),
}

pub type PayloadOwned = Payload<LocalSeedDict, MaskObject, MaskObject>;

impl<D, M, N> ToBytes for Payload<D, M, N>
where
    D: Borrow<LocalSeedDict>,
    M: Borrow<MaskObject>,
    N: Borrow<MaskObject>,
{
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
