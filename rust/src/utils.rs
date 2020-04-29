use crate::crypto::ByteObject;
use num::{
    bigint::{BigUint, ToBigInt},
    rational::Ratio,
    traits::identities::Zero,
};
use rand::RngCore;
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

#[cfg(test)]
mod tests {
    use num::traits::pow::Pow;
    use rand::SeedableRng;

    use super::*;
    use crate::crypto::Signature;

    #[test]
    fn test_is_eligible() {
        // eligible signature
        let sig = Signature::from_slice_unchecked(&[
            172, 29, 85, 219, 118, 44, 107, 32, 219, 253, 25, 242, 53, 45, 111, 62, 102, 130, 24,
            8, 222, 199, 34, 120, 166, 163, 223, 229, 100, 50, 252, 244, 250, 88, 196, 151, 136,
            48, 39, 198, 166, 86, 29, 151, 13, 81, 69, 198, 40, 148, 134, 126, 7, 202, 1, 56, 174,
            43, 89, 28, 242, 194, 4, 214,
        ]);
        assert!(is_eligible(&sig, 0.5_f64));

        // ineligible signature
        let sig = Signature::from_slice_unchecked(&[
            119, 2, 197, 174, 52, 165, 229, 22, 218, 210, 240, 188, 220, 232, 149, 129, 211, 13,
            61, 217, 186, 79, 102, 15, 109, 237, 83, 193, 12, 117, 210, 66, 99, 230, 30, 131, 63,
            108, 28, 222, 48, 92, 153, 71, 159, 220, 115, 181, 183, 155, 146, 182, 205, 89, 140,
            234, 100, 40, 199, 248, 23, 147, 172, 248,
        ]);
        assert!(!is_eligible(&sig, 0.5_f64));
    }

    #[test]
    fn test_generate_integer() {
        let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
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
}
