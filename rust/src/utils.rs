use crate::crypto::ByteObject;
use num::{
    bigint::{BigInt, BigUint, ToBigInt},
    rational::Ratio,
    traits::{cast::ToPrimitive, float::FloatCore, identities::Zero},
};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use sodiumoxide::crypto::hash::sha256;

use crate::ParticipantTaskSignature;

/// Compute the floating point representation of the hashed signature and ensure that it
/// is below the given threshold: int(hash(signature)) / (2**hashbits - 1) <= threshold.
pub fn is_eligible(signature: &ParticipantTaskSignature, threshold: f64) -> bool {
    if threshold < 0_f64 {
        false
    } else if threshold > 1_f64 {
        true
    } else {
        // safe unwraps: `to_bigint` never fails for `BigUint`s
        let numer = BigUint::from_bytes_le(sha256::hash(signature.as_slice()).as_ref())
            .to_bigint()
            .unwrap();
        let denom = BigUint::from_bytes_le([255_u8; 32].as_ref())
            .to_bigint()
            .unwrap();
        // safe unwrap: `threshold` is guaranteed to be finite
        Ratio::new(numer, denom) <= Ratio::from_float(threshold).unwrap()
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

/// Cast a ratio as float.
pub fn ratio_as<F: FloatCore>(ratio: &Ratio<BigInt>) -> F {
    let mut numer = ratio.numer().clone();
    let mut denom = ratio.denom().clone();
    loop {
        if let (Some(n), Some(d)) = (F::from(numer.clone()), F::from(denom.clone())) {
            if d == F::zero() {
                return F::zero();
            } else {
                let float = n / d;
                if float.is_finite() {
                    return float;
                }
            }
        } else {
            numer >>= 1_usize;
            denom >>= 1_usize;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use num::traits::pow::Pow;

    use super::*;
    use crate::{crypto::Signature, mask::MaskSeed};

    #[test]
    fn test_is_eligible() {
        // eligible signature
        let sig = Signature::from_slice_unchecked(&[
            229, 191, 74, 163, 113, 6, 242, 191, 255, 225, 40, 89, 210, 94, 25, 50, 44, 129, 155,
            241, 99, 64, 25, 212, 157, 235, 102, 95, 115, 18, 158, 115, 253, 136, 178, 223, 4, 47,
            54, 162, 236, 78, 126, 114, 205, 217, 250, 163, 223, 149, 31, 65, 179, 179, 60, 64, 34,
            1, 78, 245, 1, 50, 165, 47,
        ]);
        assert!(is_eligible(&sig, 0.5_f64));

        // ineligible signature
        let sig = Signature::from_slice_unchecked(&[
            15, 107, 81, 84, 105, 246, 165, 81, 76, 125, 140, 172, 113, 85, 51, 173, 119, 123, 78,
            114, 249, 182, 135, 212, 134, 38, 125, 153, 120, 45, 179, 55, 116, 155, 205, 51, 247,
            37, 78, 147, 63, 231, 28, 61, 251, 41, 48, 239, 125, 0, 129, 126, 194, 123, 183, 11,
            215, 220, 1, 225, 248, 131, 64, 242,
        ]);
        assert!(!is_eligible(&sig, 0.5_f64));
    }

    #[test]
    fn test_generate_integer() {
        let seed = MaskSeed::try_from(vec![0_u8; 32]).unwrap();
        let mut prng = ChaCha20Rng::from_seed(seed.seed());
        let max_int = BigUint::from(u128::max_value()).pow(2_usize);
        assert_eq!(
            generate_integer(&mut prng, &max_int).to_bytes_le(),
            [
                118, 184, 224, 173, 160, 241, 61, 144, 64, 93, 106, 229, 83, 134, 189, 40, 189,
                210, 25, 184, 160, 141, 237, 26, 168, 54, 239, 204, 139, 119, 13, 199,
            ],
        );
        assert_eq!(
            generate_integer(&mut prng, &max_int).to_bytes_le(),
            [
                218, 65, 89, 124, 81, 87, 72, 141, 119, 36, 224, 63, 184, 216, 74, 55, 106, 67,
                184, 244, 21, 24, 161, 28, 195, 135, 182, 105, 178, 238, 101, 134,
            ],
        );
        assert_eq!(
            generate_integer(&mut prng, &max_int).to_bytes_le(),
            [
                159, 7, 231, 190, 85, 81, 56, 122, 152, 186, 151, 124, 115, 45, 8, 13, 203, 15, 41,
                160, 72, 227, 101, 105, 18, 198, 83, 62, 50, 238, 122, 237,
            ],
        );
        assert_eq!(
            generate_integer(&mut prng, &max_int).to_bytes_le(),
            [
                41, 183, 33, 118, 156, 230, 78, 67, 213, 113, 51, 176, 116, 216, 57, 213, 49, 237,
                31, 40, 81, 10, 251, 69, 172, 225, 10, 31, 75, 121, 77, 111,
            ],
        );
        assert_eq!(
            generate_integer(&mut prng, &max_int).to_bytes_le(),
            [
                45, 9, 160, 230, 99, 38, 108, 225, 174, 126, 209, 8, 25, 104, 160, 117, 142, 113,
                142, 153, 123, 211, 98, 198, 176, 195, 70, 52, 169, 160, 179, 93,
            ],
        );
    }

    #[test]
    fn test_ratio_as() {
        // f32
        let ratio = Ratio::from_float(0_f32).unwrap();
        assert_eq!(ratio_as::<f32>(&ratio), 0_f32);
        let ratio = Ratio::from_float(0.1_f32).unwrap();
        assert_eq!(ratio_as::<f32>(&ratio), 0.1_f32);
        let ratio = (Ratio::from_float(f32::max_value()).unwrap() * BigInt::from(10_usize))
            / (Ratio::from_float(f32::max_value()).unwrap() * BigInt::from(100_usize));
        assert_eq!(ratio_as::<f32>(&ratio), 0.1_f32);

        // f64
        let ratio = Ratio::from_float(0_f64).unwrap();
        assert_eq!(ratio_as::<f64>(&ratio), 0_f64);
        let ratio = Ratio::from_float(0.1_f64).unwrap();
        assert_eq!(ratio_as::<f64>(&ratio), 0.1_f64);
        let ratio = (Ratio::from_float(f64::max_value()).unwrap() * BigInt::from(10_usize))
            / (Ratio::from_float(f64::max_value()).unwrap() * BigInt::from(100_usize));
        assert_eq!(ratio_as::<f64>(&ratio), 0.1_f64);
    }
}
