//! Wrappers around some of the [sodiumoxide] encryption primitives.
//!
//! See the [crypto module] documentation since this is a private module anyways.
//!
//! [sodiumoxide]: https://docs.rs/sodiumoxide/
//! [crypto module]: crate::crypto

use derive_more::{AsMut, AsRef, From};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::{box_, sealedbox};

use super::ByteObject;

/// Number of additional bytes in a ciphertext compared to the corresponding plaintext.
pub const SEALBYTES: usize = sealedbox::SEALBYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// A `C25519` key pair for asymmetric authenticated encryption.
pub struct EncryptKeyPair {
    /// The `C25519` public key.
    pub public: PublicEncryptKey,
    /// The `C25519` secret key.
    pub secret: SecretEncryptKey,
}

impl EncryptKeyPair {
    /// Generates a new random `C25519` key pair for encryption.
    pub fn generate() -> Self {
        let (pk, sk) = box_::gen_keypair();
        Self {
            public: PublicEncryptKey(pk),
            secret: SecretEncryptKey(sk),
        }
    }

    /// Deterministically derives a new `C25519` key pair for encryption from a seed.
    pub fn derive_from_seed(seed: &EncryptKeySeed) -> Self {
        let (pk, sk) = seed.derive_encrypt_key_pair();
        Self {
            public: pk,
            secret: sk,
        }
    }
}

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
/// A `C25519` public key for asymmetric authenticated encryption.
pub struct PublicEncryptKey(box_::PublicKey);

impl ByteObject for PublicEncryptKey {
    const LENGTH: usize = box_::PUBLICKEYBYTES;

    fn zeroed() -> Self {
        Self(box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]))
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn from_slice(bytes: &[u8]) -> Option<Self> {
        box_::PublicKey::from_slice(bytes).map(Self)
    }
}

impl PublicEncryptKey {
    /// Encrypts a message `m` with this public key.
    ///
    /// The resulting ciphertext length is [`SEALBYTES`]` + m.len()`.
    ///
    /// The function creates a new ephemeral key pair for the message and attaches the ephemeral
    /// public key to the ciphertext. The ephemeral secret key is zeroed out and is not accessible
    /// after this function returns.
    pub fn encrypt(&self, m: &[u8]) -> Vec<u8> {
        sealedbox::seal(m, self.as_ref())
    }
}

#[derive(thiserror::Error, Debug)]
#[error("decryption of a message failed")]
/// An error related to the decryption of a message.
pub struct DecryptionError;

#[derive(AsRef, AsMut, From, Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
/// A `C25519` secret key for asymmetric authenticated encryption.
///
/// When this goes out of scope, its contents will be zeroed out.
pub struct SecretEncryptKey(box_::SecretKey);

impl SecretEncryptKey {
    /// Decrypts the ciphertext `c` using this secret key and the associated public key, and returns
    /// the decrypted message.
    ///
    /// # Errors
    /// Returns `Err(DecryptionError)` if decryption fails.
    pub fn decrypt(&self, c: &[u8], pk: &PublicEncryptKey) -> Result<Vec<u8>, DecryptionError> {
        sealedbox::open(c, pk.as_ref(), self.as_ref()).map_err(|_| DecryptionError)
    }

    /// Computes the corresponding public key for this secret key.
    pub fn public_key(&self) -> PublicEncryptKey {
        PublicEncryptKey(self.0.public_key())
    }
}

impl ByteObject for SecretEncryptKey {
    const LENGTH: usize = box_::SECRETKEYBYTES;

    fn zeroed() -> Self {
        Self(box_::SecretKey([0_u8; box_::SECRETKEYBYTES]))
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn from_slice(bytes: &[u8]) -> Option<Self> {
        box_::SecretKey::from_slice(bytes).map(Self)
    }
}

#[derive(AsRef, AsMut, From, Serialize, Deserialize, Eq, PartialEq, Clone)]
/// A seed that can be used for `C25519` encryption key pair generation.
///
/// When this goes out of scope, its contents will be zeroed out.
pub struct EncryptKeySeed(box_::Seed);

impl EncryptKeySeed {
    /// Deterministically derives a new key pair from this seed.
    pub fn derive_encrypt_key_pair(&self) -> (PublicEncryptKey, SecretEncryptKey) {
        let (pk, sk) = box_::keypair_from_seed(self.as_ref());
        (PublicEncryptKey(pk), SecretEncryptKey(sk))
    }
}

impl ByteObject for EncryptKeySeed {
    const LENGTH: usize = box_::SEEDBYTES;

    fn from_slice(bytes: &[u8]) -> Option<Self> {
        box_::Seed::from_slice(bytes).map(Self)
    }

    fn zeroed() -> Self {
        Self(box_::Seed([0; box_::SEEDBYTES]))
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }
}
