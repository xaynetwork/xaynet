//! PRNG utilities for the crypto primitives.
//!
//! See the [crypto module] documentation since this is a private module anyways.
//!
//! [sodiumoxide]: https://docs.rs/sodiumoxide/
//! [crypto module]: crate::crypto

use num::{bigint::BigUint, traits::identities::Zero};
use rand::RngCore;
use rand_chacha::ChaCha20Rng;

/// Generates a secure pseudo-random integer.
///
/// Draws from a uniform distribution over the integers between zero (included) and
/// `max_int` (excluded). Employs the `ChaCha20` stream cipher as a PRNG.
pub fn generate_integer(prng: &mut ChaCha20Rng, max_int: &BigUint) -> BigUint {
    if max_int.is_zero() {
        return BigUint::zero();
    }
    let mut bytes = max_int.to_bytes_le();
    let mut rand_int = max_int.clone();
    while &rand_int >= max_int {
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
