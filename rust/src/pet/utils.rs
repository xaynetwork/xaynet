use num::{
    bigint::{BigUint, ToBigInt},
    rational::Ratio,
};
use sodiumoxide::crypto::hash::sha256::hash;

/// Compute the floating point representation of the hashed signature and ensure that it
/// is below the given threshold: int(hash(signature)) / (2**hashbits - 1) <= threshold.
pub fn is_eligible(signature: &[u8], threshold: f64) -> Option<bool> {
    Some(
        Ratio::new(
            BigUint::from_bytes_be(&hash(signature).0[..]).to_bigint()?,
            BigUint::from_bytes_be(&[255_u8; 32][..]).to_bigint()?,
        ) <= Ratio::from_float(threshold)?,
    )
}
