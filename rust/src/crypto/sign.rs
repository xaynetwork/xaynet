//! Wrappers around some of the [sodiumoxide] signing primitives.
//!
//! See the [crypto module] documentation since this is a private module anyways.
//!
//! [sodiumoxide]: https://docs.rs/sodiumoxide/
//! [crypto module]: ../index.html

use derive_more::{AsMut, AsRef, From};
use num::{
    bigint::{BigUint, ToBigInt},
    rational::Ratio,
};
use sodiumoxide::crypto::{hash::sha256, sign};

use super::ByteObject;

/// Generates a new random key pair for signing.
pub fn generate_signing_key_pair() -> (PublicSigningKey, SecretSigningKey) {
    let (pk, sk) = sign::gen_keypair();
    (PublicSigningKey(pk), SecretSigningKey(sk))
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
/// A public key for signatures.
pub struct PublicSigningKey(sign::PublicKey);

impl PublicSigningKey {
    /// Length in bytes of a [`PublicSigningKey`].
    pub const LENGTH: usize = sign::PUBLICKEYBYTES;

    /// Verifies the signature `s` against the message `m` and this public key.
    ///
    /// Returns `true` if the signature is valid and `false` otherwise.
    pub fn verify_detached(&self, s: &Signature, m: &[u8]) -> bool {
        sign::verify_detached(s.as_ref(), m, self.as_ref())
    }
}

impl ByteObject for PublicSigningKey {
    fn zeroed() -> Self {
        Self(sign::PublicKey([0_u8; sign::PUBLICKEYBYTES]))
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn from_slice(bytes: &[u8]) -> Option<Self> {
        sign::PublicKey::from_slice(bytes).map(Self)
    }
}

#[derive(AsRef, AsMut, From, Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
/// A secret key for signatures.
///
/// When this goes out of scope, its contents will be zeroed out.
pub struct SecretSigningKey(sign::SecretKey);

impl SecretSigningKey {
    /// Length in bytes of a [`SecretSigningKey`].
    pub const LENGTH: usize = sign::SECRETKEYBYTES;

    /// Signs a message `m` with this secret key.
    pub fn sign_detached(&self, m: &[u8]) -> Signature {
        sign::sign_detached(m, self.as_ref()).into()
    }

    /// Computes the corresponding public key for this secret key.
    pub fn public_key(&self) -> PublicSigningKey {
        PublicSigningKey(self.0.public_key())
    }
}

impl ByteObject for SecretSigningKey {
    fn zeroed() -> Self {
        Self(sign::SecretKey([0_u8; sign::SECRETKEYBYTES]))
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn from_slice(bytes: &[u8]) -> Option<Self> {
        sign::SecretKey::from_slice(bytes).map(Self)
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
/// A signature detached from its message.
pub struct Signature(sign::Signature);

impl ByteObject for Signature {
    fn zeroed() -> Self {
        Self(sign::Signature([0_u8; sign::SIGNATUREBYTES]))
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn from_slice(bytes: &[u8]) -> Option<Self> {
        sign::Signature::from_slice(bytes).map(Self)
    }
}

impl Signature {
    /// Length in bytes of a [`Signature`].
    pub const LENGTH: usize = sign::SIGNATUREBYTES;

    /// Computes the floating point representation of the hashed
    /// signature and ensure that it is below the given threshold:
    /// ```no_rust
    /// int(hash(signature)) / (2**hashbits - 1) <= threshold.
    /// ```
    pub fn is_eligible(&self, threshold: f64) -> bool {
        if threshold < 0_f64 {
            return false;
        } else if threshold > 1_f64 {
            return true;
        }
        // safe unwraps: `to_bigint` never fails for `BigUint`s
        let numer = BigUint::from_bytes_le(sha256::hash(self.as_slice()).as_ref())
            .to_bigint()
            .unwrap();
        let denom = BigUint::from_bytes_le([u8::MAX; sha256::DIGESTBYTES].as_ref())
            .to_bigint()
            .unwrap();
        // safe unwrap: `threshold` is guaranteed to be finite
        Ratio::new(numer, denom) <= Ratio::from_float(threshold).unwrap()
    }
}

#[derive(AsRef, AsMut, From, Serialize, Deserialize, Eq, PartialEq, Clone)]
/// A seed that can be used for signing key pair generation.
///
/// When this goes out of scope, its contents will be zeroed out.
pub struct SigningKeySeed(sign::Seed);

impl SigningKeySeed {
    /// Length in bytes of a [`SigningKeySeed`].
    pub const LENGTH: usize = sign::SEEDBYTES;

    /// Deterministically derives a new signing key pair from this seed.
    pub fn derive_signing_key_pair(&self) -> (PublicSigningKey, SecretSigningKey) {
        let (pk, sk) = sign::keypair_from_seed(&self.0);
        (PublicSigningKey(pk), SecretSigningKey(sk))
    }
}

impl ByteObject for SigningKeySeed {
    fn from_slice(bytes: &[u8]) -> Option<Self> {
        sign::Seed::from_slice(bytes).map(Self)
    }

    fn zeroed() -> Self {
        Self(sign::Seed([0; sign::PUBLICKEYBYTES]))
    }

    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_is_eligible() {
        // eligible signature
        let sig = Signature::from_slice_unchecked(&[
            172, 29, 85, 219, 118, 44, 107, 32, 219, 253, 25, 242, 53, 45, 111, 62, 102, 130, 24,
            8, 222, 199, 34, 120, 166, 163, 223, 229, 100, 50, 252, 244, 250, 88, 196, 151, 136,
            48, 39, 198, 166, 86, 29, 151, 13, 81, 69, 198, 40, 148, 134, 126, 7, 202, 1, 56, 174,
            43, 89, 28, 242, 194, 4, 214,
        ]);
        assert!(sig.is_eligible(0.5_f64));

        // ineligible signature
        let sig = Signature::from_slice_unchecked(&[
            119, 2, 197, 174, 52, 165, 229, 22, 218, 210, 240, 188, 220, 232, 149, 129, 211, 13,
            61, 217, 186, 79, 102, 15, 109, 237, 83, 193, 12, 117, 210, 66, 99, 230, 30, 131, 63,
            108, 28, 222, 48, 92, 153, 71, 159, 220, 115, 181, 183, 155, 146, 182, 205, 89, 140,
            234, 100, 40, 199, 248, 23, 147, 172, 248,
        ]);
        assert!(!sig.is_eligible(0.5_f64));
    }
}
