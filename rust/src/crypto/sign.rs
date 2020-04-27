use super::ByteObject;
use derive_more::{AsMut, AsRef, From};
use sodiumoxide::crypto::sign;

/// Generate a new random signing key pair
pub fn generate_signing_key_pair() -> (PublicSigningKey, SecretSigningKey) {
    let (pk, sk) = sign::gen_keypair();
    (PublicSigningKey(pk), SecretSigningKey(sk))
}

/// Public key for signatures
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
pub struct PublicSigningKey(sign::PublicKey);

impl PublicSigningKey {
    /// Verify the signature `s` against the message `m` and the
    /// signer's public key `&self`.
    ///
    /// # Return value
    ///
    /// This method returns `true` if the signature is valid, `false`
    /// otherwise.
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

/// Secret key for signatures
#[derive(AsRef, AsMut, From, Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct SecretSigningKey(sign::SecretKey);

impl SecretSigningKey {
    /// Sign a message `m`
    pub fn sign_detached(&self, m: &[u8]) -> Signature {
        sign::sign_detached(m, self.as_ref()).into()
    }

    /// Compute the corresponding public key for this secret key
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

/// Detached signature
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

/// A seed that can be used for signing key pair generation. When
/// `KeySeed` goes out of scope, its contents will be zeroed out.
#[derive(AsRef, AsMut, From, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct SigningKeySeed(sign::Seed);

impl SigningKeySeed {
    /// Deterministically derive a new signing key pair from this seed
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
