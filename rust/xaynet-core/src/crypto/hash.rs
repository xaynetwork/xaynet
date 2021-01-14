//! Wrappers around some of the [sodiumoxide] hashing primitives.
//!
//! See the [crypto module] documentation since this is a private module anyways.
//!
//! [sodiumoxide]: https://docs.rs/sodiumoxide/
//! [crypto module]: crate::crypto

use derive_more::{AsMut, AsRef, From};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::hash::sha256;

use super::ByteObject;

#[derive(
    AsRef,
    AsMut,
    From,
    Serialize,
    Deserialize,
    Hash,
    Eq,
    Ord,
    PartialEq,
    Copy,
    Clone,
    PartialOrd,
    Debug,
)]
/// A digest of the `SHA256` hash function.
pub struct Sha256(sha256::Digest);

impl ByteObject for Sha256 {
    const LENGTH: usize = sha256::DIGESTBYTES;

    fn zeroed() -> Self {
        Self(sha256::Digest([0_u8; sha256::DIGESTBYTES]))
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn from_slice(bytes: &[u8]) -> Option<Self> {
        sha256::Digest::from_slice(bytes).map(Self)
    }
}

impl Sha256 {
    /// Computes the digest of the message `m`.
    pub fn hash(m: &[u8]) -> Self {
        Self(sha256::hash(m))
    }
}
