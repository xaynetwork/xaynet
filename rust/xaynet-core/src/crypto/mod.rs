//! Wrappers around some of the [sodiumoxide] crypto primitives.
//!
//! The wrappers provide methods defined on structs instead of the sodiumoxide functions. This is
//! done for the `C25519` encryption and `Ed25519` signature key pairs and their corresponding seeds
//! as well as the `SHA256` hash function. Additionally, some methods for slicing and signature
//! eligibility are available.
//!
//! # Examples
//! ## Encryption of messages
//! ```
//! # use xaynet_core::crypto::EncryptKeyPair;
//! let keys = EncryptKeyPair::generate();
//! let message = b"Hello world!".to_vec();
//! let cipher = keys.public.encrypt(&message);
//! assert_eq!(message, keys.secret.decrypt(&cipher, &keys.public).unwrap());
//! ```
//!
//! ## Signing of messages
//! ```
//! # use xaynet_core::crypto::SigningKeyPair;
//! let keys = SigningKeyPair::generate();
//! let message = b"Hello world!".to_vec();
//! let signature = keys.secret.sign_detached(&message);
//! assert!(keys.public.verify_detached(&signature, &message));
//! ```
//!
//! [sodiumoxide]: https://docs.rs/sodiumoxide/

pub(crate) mod encrypt;
pub(crate) mod hash;
pub(crate) mod prng;
pub(crate) mod sign;

use sodiumoxide::randombytes::randombytes;

pub use self::{
    encrypt::{EncryptKeyPair, EncryptKeySeed, PublicEncryptKey, SecretEncryptKey, SEALBYTES},
    hash::Sha256,
    prng::generate_integer,
    sign::{PublicSigningKey, SecretSigningKey, Signature, SigningKeyPair, SigningKeySeed},
};

/// An interface for slicing into cryptographic byte objects.
pub trait ByteObject: Sized {
    /// Length in bytes of this object
    const LENGTH: usize;

    /// Creates a new object with all the bytes initialized to `0`.
    fn zeroed() -> Self;

    /// Gets the object byte representation.
    fn as_slice(&self) -> &[u8];

    /// Creates an object from the given buffer.
    ///
    /// # Errors
    /// Returns `None` if the length of the byte-slice isn't equal to the length of the object.
    fn from_slice(bytes: &[u8]) -> Option<Self>;

    /// Creates an object from the given buffer.
    ///
    /// # Panics
    /// Panics if the length of the byte-slice isn't equal to the length of the object.
    fn from_slice_unchecked(bytes: &[u8]) -> Self {
        Self::from_slice(bytes).unwrap()
    }

    /// Generates an object with random bytes
    fn generate() -> Self {
        // safe unwrap: length of slice is guaranteed by constants
        Self::from_slice_unchecked(randombytes(Self::LENGTH).as_slice())
    }

    /// A helper for instantiating an object filled with the given value
    fn fill_with(value: u8) -> Self {
        Self::from_slice_unchecked(&vec![value; Self::LENGTH])
    }
}
