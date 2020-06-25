//! Wrappers around some of the [sodiumoxide] encrytion primitives.
//!
//! See the [crypto module] documentation since this is a private module anyways.
//!
//! [sodiumoxide]: https://docs.rs/sodiumoxide/
//! [crypto module]: ../index.html

use derive_more::{AsMut, AsRef, From};
use sodiumoxide::crypto::{box_, sealedbox};

use super::ByteObject;

/// Number of additional bytes in a ciphertext compared to the corresponding plaintext.
pub const SEALBYTES: usize = sealedbox::SEALBYTES;

/// Generates a new random `C25519` key pair for encryption.
pub fn generate_encrypt_key_pair() -> (PublicEncryptKey, SecretEncryptKey) {
    let (pk, sk) = box_::gen_keypair();
    (PublicEncryptKey(pk), SecretEncryptKey(sk))
}

#[derive(Debug, Clone)]
pub struct KeyPair {
    pub public: PublicEncryptKey,
    pub secret: SecretEncryptKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        let (public, secret) = generate_encrypt_key_pair();
        Self { public, secret }
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
    /// Length in bytes of this public key.
    pub const LENGTH: usize = box_::PUBLICKEYBYTES;

    /// Encrypts a message `m` with this public key. The resulting ciphertext length is
    /// [`SEALBYTES`]` + m.len()`.
    ///
    /// The function creates a new ephemeral key pair for each message, and attaches the ephemeral
    /// public key to the ciphertext. The ephemeral secret key is zeroed out and is not accessible
    /// after this function returns.
    pub fn encrypt(&self, m: &[u8]) -> Vec<u8> {
        sealedbox::seal(m, self.as_ref())
    }
}

#[derive(AsRef, AsMut, From, Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
/// A `C25519` secret key for asymmetric authenticated encryption.
///
/// When this goes out of scope, its contents will be zeroed out.
pub struct SecretEncryptKey(box_::SecretKey);

impl SecretEncryptKey {
    /// Length in bytes of this secret key.
    pub const LENGTH: usize = box_::SECRETKEYBYTES;

    /// Decrypts the ciphertext `c` using this secret key and the associated public key, and returns
    /// the decrypted message.
    ///
    /// # Errors
    /// Returns `Err(())` if decryption fails.
    pub fn decrypt(&self, c: &[u8], pk: &PublicEncryptKey) -> Result<Vec<u8>, ()> {
        sealedbox::open(c, pk.as_ref(), self.as_ref())
    }

    /// Computes the corresponding public key for this secret key.
    pub fn public_key(&self) -> PublicEncryptKey {
        PublicEncryptKey(self.0.public_key())
    }
}

impl ByteObject for SecretEncryptKey {
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
    /// Length in bytes of this seed.
    pub const LENGTH: usize = box_::SEEDBYTES;

    /// Deterministically derives a new key pair from this seed.
    pub fn derive_encrypt_key_pair(&self) -> (PublicEncryptKey, SecretEncryptKey) {
        let (pk, sk) = box_::keypair_from_seed(self.as_ref());
        (PublicEncryptKey(pk), SecretEncryptKey(sk))
    }
}

impl ByteObject for EncryptKeySeed {
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
