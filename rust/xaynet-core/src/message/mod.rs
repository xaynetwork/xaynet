//! The messages of the PET protocol.
//!
//! # The sum message
//! The [`Sum`] message is an abstraction for the values which a sum participant communicates to
//! XayNet during the sum phase of the PET protocol. It contains the following values:
//! - The sum signature proves the eligibility of the participant for the sum task.
//! - The ephemeral public key is used by update participants to encrypt mask seeds in the update
//!   phase for the process of mask aggregation in the sum2 phase.
//!
//! # The update message
//! The [`Update`] message is an abstraction for the values which an update participant communicates
//! to XayNet during the update phase of the PET protocol. It contains the following values:
//! - The sum signature proves the ineligibility of the participant for the sum task.
//! - The update signature proves the eligibility of the participant for the update task.
//! - The masked model is the encrypted local update to the global model, which is trained on the
//!   local data of the update participant.
//! - The local seed dictionary stores the encrypted mask seed, which generates the local mask for
//!   the local model, which is encrypted by the ephemeral public keys of the sum participants.
//!
//! # The sum2 message
//! The [`Sum2`] message is an abstraction for the values which a sum participant communicates to
//! XayNet during the sum2 phase of the PET protocol. It contains the following values:
//! - The sum signature proves the eligibility of the participant for the sum task.
//! - The global mask is used by XayNet to unmask the aggregated global model.
//!
//! [crypto module]: ../crypto/index.html

#[allow(clippy::module_inception)]
pub(crate) mod message;
pub(crate) mod payload;
pub(crate) mod traits;

// FIXME: I'd like to make this `pub(crate)` but then the doc-tests in
// utils::chunkable_iterator cannot be compiled
#[doc(hidden)]
pub mod utils;

pub use self::{
    message::{Flags, Message, MessageBuffer, Tag},
    payload::{
        chunk::{Chunk, ChunkBuffer},
        sum::{Sum, SumBuffer},
        sum2::{Sum2, Sum2Buffer},
        update::{Update, UpdateBuffer},
        Payload,
    },
    traits::{FromBytes, LengthValueBuffer, ToBytes},
};

/// An error that signals a failure when trying to decrypt and parse a message.
///
/// This is kept generic on purpose to not reveal to the sender what specifically failed during
/// decryption or parsing.
pub type DecodeError = anyhow::Error;
