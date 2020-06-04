//! This module provides wrapper around some `sodiumoxide` crypto
//! primitives.

mod encrypt;
mod hash;
mod prng;
mod sign;
pub use self::{
    encrypt::{
        generate_encrypt_key_pair,
        EncryptKeySeed,
        KeyPair,
        PublicEncryptKey,
        SecretEncryptKey,
        SEALBYTES,
    },
    hash::Sha256,
    prng::generate_integer,
    sign::{
        generate_signing_key_pair,
        PublicSigningKey,
        SecretSigningKey,
        Signature,
        SigningKeySeed,
    },
};

pub trait ByteObject: Sized {
    /// Create a new object with all the bytes initialized to `0`.
    fn zeroed() -> Self;

    /// Get the object byte representation
    fn as_slice(&self) -> &[u8];

    /// Create a object from the given buffer. This function will fail
    /// and return `None` if the length of the byte-slice isn't equal to
    /// the length of the object.
    fn from_slice(bytes: &[u8]) -> Option<Self>;

    /// Create a object from the given buffer.
    ///
    /// # Panic
    ///
    /// This function will panic if the length of the byte-slice isn't
    /// equal to the length of the object.
    fn from_slice_unchecked(bytes: &[u8]) -> Self {
        Self::from_slice(bytes).unwrap()
    }
}
