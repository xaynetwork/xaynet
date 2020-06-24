//! Wrappers around some of the [sodiumoxide] crypto primitives.
//!
//! The wrappers provide methods defined on structs instead of the sodiumoxide functions. This is
//! done for the encryption and signature key pairs and their corresponding seeds as well as a hash
//! function. Additionally, some methods for slicing and signature eligibility are made available.
//!
//! # Examples
//!
//! ## Encryption of messages
//! ```
//! # use xain_fl::crypto::generate_encrypt_key_pair;
//! let (pk, sk) = generate_encrypt_key_pair();
//! let message = b"Hello world!".to_vec();
//! let cipher = pk.encrypt(&message);
//! assert_eq!(message, sk.decrypt(&cipher, &pk).unwrap());
//! ```
//!
//! ## Signing of messages
//! ```
//! # use xain_fl::crypto::generate_signing_key_pair;
//! let (pk, sk) = generate_signing_key_pair();
//! let message = b"Hello world!".to_vec();
//! let signature = sk.sign_detached(&message);
//! assert!(pk.verify_detached(&signature, &message));
//! ```
//!
//! [sodiumoxide]: https://docs.rs/sodiumoxide/

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

/// An interface for slicing into cryptographic byte objects.
pub trait ByteObject: Sized {
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
}
