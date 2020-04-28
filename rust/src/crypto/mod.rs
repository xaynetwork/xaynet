//! This module provides wrapper around some `sodiumoxide` crypto
//! primitives.

mod encrypt;
mod hash;
mod sign;

use num::{bigint::BigUint, traits::identities::Zero};
use rand::RngCore;
use rand_chacha::ChaCha20Rng;

pub use self::{
    encrypt::{
        generate_encrypt_key_pair,
        EncryptKeySeed,
        PublicEncryptKey,
        SecretEncryptKey,
        SEALBYTES,
    },
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

/// Generate a secure pseudo-random integer. Draws from a uniform distribution over the integers
/// between zero (included) and `max_int` (excluded).
pub fn generate_integer(prng: &mut ChaCha20Rng, max_int: &BigUint) -> BigUint {
    if max_int.is_zero() {
        return BigUint::zero();
    }
    let mut bytes = max_int.to_bytes_le();
    let mut rand_int = max_int.clone();
    while rand_int >= *max_int {
        prng.fill_bytes(&mut bytes);
        rand_int = BigUint::from_bytes_le(&bytes);
    }
    rand_int
}

#[cfg(test)]
mod tests {
    use num::traits::{pow::Pow, Num};
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn test_generate_integer() {
        let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
        let max_int = BigUint::from(u128::max_value()).pow(2_usize);
        assert_eq!(
            generate_integer(&mut prng, &max_int),
            BigUint::from_str_radix(
                "90034050956742099321159087842304570510687605373623064829879336909608119744630",
                10
            )
            .unwrap()
        );
        assert_eq!(
            generate_integer(&mut prng, &max_int),
            BigUint::from_str_radix(
                "60790020689334235010238064028215988394112077193561636249125918224917556969946",
                10
            )
            .unwrap()
        );
        assert_eq!(
            generate_integer(&mut prng, &max_int),
            BigUint::from_str_radix(
                "107415344426328791036720294006773438815099086866510488084511304829720271980447",
                10
            )
            .unwrap()
        );
        assert_eq!(
            generate_integer(&mut prng, &max_int),
            BigUint::from_str_radix(
                "50343610553303623842889112417183549658912134525854625844144939347139411162921",
                10
            )
            .unwrap()
        );
        assert_eq!(
            generate_integer(&mut prng, &max_int),
            BigUint::from_str_radix(
                "42382469383990928111449714288937630103705168010724718767641573929365517895981",
                10
            )
            .unwrap()
        );
    }
}
