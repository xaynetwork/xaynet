use num::{
    bigint::{BigUint, ToBigInt},
    rational::Ratio,
};
use sodiumoxide::crypto::{hash::sha256, sign};

/// Compute the floating point representation of the hashed signature and ensure that it
/// is below the given threshold: int(hash(signature)) / (2**hashbits - 1) <= threshold.
pub fn is_eligible(signature: &sign::Signature, threshold: f64) -> Option<bool> {
    (0_f64 <= threshold && threshold <= 1_f64).then_some(
        Ratio::new(
            BigUint::from_bytes_be(&sha256::hash(&signature.0[..]).0[..]).to_bigint()?,
            BigUint::from_bytes_be(&[255_u8; 32][..]).to_bigint()?,
        ) <= Ratio::from_float(threshold)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_eligible() {
        // eligible signature
        let sig = sign::Signature([
            229, 191, 74, 163, 113, 6, 242, 191, 255, 225, 40, 89, 210, 94, 25, 50, 44, 129, 155,
            241, 99, 64, 25, 212, 157, 235, 102, 95, 115, 18, 158, 115, 253, 136, 178, 223, 4, 47,
            54, 162, 236, 78, 126, 114, 205, 217, 250, 163, 223, 149, 31, 65, 179, 179, 60, 64, 34,
            1, 78, 245, 1, 50, 165, 47,
        ]);
        assert_eq!(is_eligible(&sig, 0.5_f64), Some(true));

        // ineligible signature
        let sig = sign::Signature([
            15, 107, 81, 84, 105, 246, 165, 81, 76, 125, 140, 172, 113, 85, 51, 173, 119, 123, 78,
            114, 249, 182, 135, 212, 134, 38, 125, 153, 120, 45, 179, 55, 116, 155, 205, 51, 247,
            37, 78, 147, 63, 231, 28, 61, 251, 41, 48, 239, 125, 0, 129, 126, 194, 123, 183, 11,
            215, 220, 1, 225, 248, 131, 64, 242,
        ]);
        assert_eq!(is_eligible(&sig, 0.5_f64), Some(false));

        // invalid thresholds
        assert_eq!(is_eligible(&sig, -0.1_f64), None);
        assert_eq!(is_eligible(&sig, 1.1_f64), None);
    }
}
