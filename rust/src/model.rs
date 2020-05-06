use std::convert::TryFrom;

use num::{bigint::BigInt, clamp, rational::Ratio, traits::identities::Zero};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::{
    crypto::generate_integer,
    mask::{config::MaskConfig, seed::MaskSeed, Integers, MaskedModel},
    PetError,
};

/// Masking of models.
pub trait MaskModels<N> {
    /// Get a reference to the weights.
    fn weights(&self) -> &Vec<N>;

    /// Cast the weights as ratios. Must handle non-finite weights.
    fn as_ratios(&self) -> Vec<Ratio<BigInt>>;

    /// Mask the model wrt the mask configuration. Enforces bounds on the scalar and weights.
    ///
    /// The masking proceeds in the following steps:
    /// - clamp the scalar and the weights according to the mask configuration
    /// - shift the weights into the non-negative reals
    /// - shift the weights into the non-negative integers
    /// - shift the weights into the finite group
    /// - mask the weights with random elements from the finite group
    ///
    /// The random elements are derived from a seeded PRNG. Unmasking proceeds in reverse order. For
    /// a more detailes see [the confluence page](https://xainag.atlassian.net/wiki/spaces/FP/pages/542408769/Masking).
    fn mask(&self, scalar: f64, config: &MaskConfig) -> (MaskSeed, MaskedModel) {
        let scalar = &Ratio::<BigInt>::from_float(clamp(scalar, 0_f64, 1_f64)).unwrap();
        let negative_bound = &-config.add_shift();
        let positive_bound = config.add_shift();
        let mask_seed = MaskSeed::generate();
        let mut prng = ChaCha20Rng::from_seed(mask_seed.as_array());
        let masked_weights = self
            .as_ratios()
            .iter()
            .map(|weight| {
                let scaled = scalar * clamp(weight, negative_bound, positive_bound);
                let shifted = ((scaled + config.add_shift()) * config.exp_shift())
                    .to_integer()
                    .to_biguint()
                    // safe unwrap: shifted weight is guaranteed to be non-negative
                    .unwrap();
                (shifted + generate_integer(&mut prng, config.order())) % config.order()
            })
            .collect();
        // safe unwrap: masked weights are guaranteed to conform to the mask configuration
        let masked_model = MaskedModel::from_parts(masked_weights, config.clone()).unwrap();
        (mask_seed, masked_model)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A model with weights represented as a vector of primitive numbers.
pub struct Model<N> {
    weights: Vec<N>,
}

impl TryFrom<Vec<f32>> for Model<f32> {
    type Error = PetError;

    /// Create a model from its weights. Fails if the weights are not finite.
    fn try_from(weights: Vec<f32>) -> Result<Self, Self::Error> {
        if weights.iter().all(|weight| weight.is_finite()) {
            Ok(Self { weights })
        } else {
            Err(Self::Error::InvalidModel)
        }
    }
}

impl TryFrom<Vec<f64>> for Model<f64> {
    type Error = PetError;

    /// Create a model from its weights. Fails if the weights are not finite.
    fn try_from(weights: Vec<f64>) -> Result<Self, Self::Error> {
        if weights.iter().all(|weight| weight.is_finite()) {
            Ok(Self { weights })
        } else {
            Err(Self::Error::InvalidModel)
        }
    }
}

impl From<Vec<i32>> for Model<i32> {
    /// Create a model from its weights.
    fn from(weights: Vec<i32>) -> Self {
        Self { weights }
    }
}

impl From<Vec<i64>> for Model<i64> {
    /// Create a model from its weights.
    fn from(weights: Vec<i64>) -> Self {
        Self { weights }
    }
}

impl MaskModels<f32> for Model<f32> {
    /// Get a reference to the weights.
    fn weights(&self) -> &Vec<f32> {
        &self.weights
    }

    /// Cast the weights as ratios. Positive/negative infinity is mapped to max/min and NaN to zero.
    fn as_ratios(&self) -> Vec<Ratio<BigInt>> {
        self.weights
            .iter()
            .map(|weight| {
                if weight.is_nan() {
                    Ratio::<BigInt>::zero()
                } else {
                    // safe unwrap: clamped weight is guaranteed to be finite
                    Ratio::<BigInt>::from_float(clamp(*weight, f32::MIN, f32::MAX)).unwrap()
                }
            })
            .collect()
    }
}

impl MaskModels<f64> for Model<f64> {
    /// Get a reference to the weights.
    fn weights(&self) -> &Vec<f64> {
        &self.weights
    }

    /// Cast the weights as ratios. Positve/negative infinity is mapped to max/min and NaN to zero.
    fn as_ratios(&self) -> Vec<Ratio<BigInt>> {
        self.weights
            .iter()
            .map(|weight| {
                if weight.is_nan() {
                    Ratio::<BigInt>::zero()
                } else {
                    // safe unwrap: clamped weight is guaranteed to be finite
                    Ratio::<BigInt>::from_float(clamp(*weight, f64::MIN, f64::MAX)).unwrap()
                }
            })
            .collect()
    }
}

impl MaskModels<i32> for Model<i32> {
    /// Get a reference to the weights.
    fn weights(&self) -> &Vec<i32> {
        &self.weights
    }

    /// Cast the weights as ratios.
    fn as_ratios(&self) -> Vec<Ratio<BigInt>> {
        self.weights
            .iter()
            .map(|weight| Ratio::from_integer(BigInt::from(*weight)))
            .collect()
    }
}

impl MaskModels<i64> for Model<i64> {
    /// Get a reference to the weights.
    fn weights(&self) -> &Vec<i64> {
        &self.weights
    }

    /// Cast the weights as ratios.
    fn as_ratios(&self) -> Vec<Ratio<BigInt>> {
        self.weights
            .iter()
            .map(|weight| Ratio::from_integer(BigInt::from(*weight)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use rand::distributions::{Distribution, Uniform};

    use super::*;
    use crate::mask::{
        config::{
            BoundType::{Bmax, B0, B2, B4, B6},
            DataType::{F32, F64, I32, I64},
            GroupType::{Integer, Power2, Prime},
            MaskConfigs,
            ModelType::{M12, M3, M6, M9},
        },
        seed::MaskSeed,
        MaskIntegers,
    };

    #[test]
    fn test_model_f32() {
        let weights = vec![-1_f32, 0_f32, 1_f32];
        let model = Model::<f32>::try_from(weights.clone()).unwrap();
        assert_eq!(model.weights(), &weights);
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f32).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f32).unwrap(),
            ],
        );
    }

    #[test]
    fn test_model_f32_inf() {
        let weights = vec![-1_f32, 0_f32, f32::INFINITY];
        assert_eq!(
            Model::<f32>::try_from(weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );
        assert_eq!(
            Model::<f32> { weights }.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f32).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(f32::MAX).unwrap(),
            ],
        );
    }

    #[test]
    fn test_model_f32_neginf() {
        let weights = vec![f32::NEG_INFINITY, 0_f32, 1_f32];
        assert_eq!(
            Model::<f32>::try_from(weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );
        assert_eq!(
            Model::<f32> { weights }.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(f32::MIN).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f32).unwrap(),
            ],
        );
    }

    #[test]
    fn test_model_f32_nan() {
        let weights = vec![-1_f32, f32::NAN, 1_f32];
        assert_eq!(
            Model::<f32>::try_from(weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );
        assert_eq!(
            Model::<f32> { weights }.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f32).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f32).unwrap(),
            ],
        );
    }

    #[test]
    fn test_model_f64() {
        let weights = vec![-1_f64, 0_f64, 1_f64];
        let model = Model::<f64>::try_from(weights.clone()).unwrap();
        assert_eq!(model.weights(), &weights);
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f64).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f64).unwrap(),
            ],
        );
    }

    #[test]
    fn test_model_f64_inf() {
        let weights = vec![-1_f64, 0_f64, f64::INFINITY];
        assert_eq!(
            Model::<f64>::try_from(weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );
        assert_eq!(
            Model::<f64> { weights }.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f64).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(f64::MAX).unwrap(),
            ],
        );
    }

    #[test]
    fn test_model_f64_neginf() {
        let weights = vec![f64::NEG_INFINITY, 0_f64, 1_f64];
        assert_eq!(
            Model::<f64>::try_from(weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );
        assert_eq!(
            Model::<f64> { weights }.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(f64::MIN).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f64).unwrap(),
            ],
        );
    }

    #[test]
    fn test_model_f64_nan() {
        let weights = vec![-1_f64, f64::NAN, 1_f64];
        assert_eq!(
            Model::<f64>::try_from(weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );
        assert_eq!(
            Model::<f64> { weights }.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f64).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f64).unwrap(),
            ],
        );
    }

    #[test]
    fn test_model_i32() {
        let weights = vec![-1_i32, 0_i32, 1_i32];
        let model = Model::<i32>::from(weights.clone());
        assert_eq!(model.weights(), &weights);
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::from_integer(BigInt::from(-1_i32)),
                Ratio::<BigInt>::zero(),
                Ratio::from_integer(BigInt::from(1_i32)),
            ],
        );
    }

    #[test]
    fn test_model_i64() {
        let weights = vec![-1_i64, 0_i64, 1_i64];
        let model = Model::<i64>::from(weights.clone());
        assert_eq!(model.weights(), &weights);
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::from_integer(BigInt::from(-1_i64)),
                Ratio::<BigInt>::zero(),
                Ratio::from_integer(BigInt::from(1_i64)),
            ],
        );
    }

    /// Generate tests for masking and unmasking. The tests proceed in the following steps:
    /// - generate random weights from a uniform distribution with a seeded PRNG
    /// - create a model from the weights and mask it
    /// - check that all masked weights belong to the chosen finite group
    /// - unmask the masked model
    /// - check that all unmasked weights are equal to the original weights (up to a tolerance)
    ///
    /// The arguments to the macro are:
    /// - a suffix for the test name
    /// - the group type of the model (variants of `GroupType`)
    /// - the data type of the model (either primitives or variants of `DataType`)
    /// - an absolute bound for the weights (optional, choices: 1, 100, 10_000, 1_000_000)
    /// - the number of weights
    /// - a tolerance for the equality check
    ///
    /// For float data types the error depends on the order of magnitude of the weights, therefore
    /// it may be necessary to raise the tolerance or bound the random weights if this test fails.
    macro_rules! test_masking {
        ($suffix:ident, $group:ty, $data:ty, $bound:expr, $len:expr, $tol:expr $(,)?) => {
            paste::item! {
                #[test]
                fn [<test_masking_ $suffix>]() {
                    paste::expr! {
                        let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
                        let uniform = if $bound == 0 {
                            Uniform::new([<$data:lower>]::MIN, [<$data:lower>]::MAX)
                        } else {
                            Uniform::new(-$bound as [<$data:lower>], $bound as [<$data:lower>])
                        };
                        let weights = iter::repeat_with(|| uniform.sample(&mut prng))
                            .take($len as usize)
                            .collect::<Vec<_>>();
                        let model = Model::try_from(weights).unwrap();
                        let bound_type = match $bound {
                            1 => B0,
                            100 => B2,
                            10_000 => B4,
                            1_000_000 => B6,
                            0 => Bmax,
                            _ => panic!("Unknown bound!")
                        };
                        let config = MaskConfigs::from_parts(
                            $group,
                            [<$data:upper>],
                            bound_type,
                            M3
                        ).config();
                        let (mask_seed, masked_model) = model.mask(1_f64, &config);
                        assert_eq!(
                            masked_model.integers().len(),
                            $len as usize
                        );
                        assert!(
                            masked_model
                                .integers()
                                .iter()
                                .all(|integer| integer < config.order())
                        );
                        let mask = mask_seed.derive_mask($len as usize, &config);
                        let unmasked_model: Model<[<$data:lower>]> = masked_model
                            .unmask(&mask, 1_usize)
                            .unwrap();
                        assert!(
                            model
                                .weights()
                                .iter()
                                .zip(unmasked_model.weights().iter())
                                .all(
                                    |(weight, unmasked_weight)|
                                        (weight - unmasked_weight).abs()
                                            <= $tol as [<$data:lower>]
                                )
                        );
                    }
                }
            }
        };
        ($suffix:ident, $group:ty, $data:ty, $len:expr, $tol:expr $(,)?) => {
            test_masking!($suffix, $group, $data, 0, $len, $tol);
        };
    }

    test_masking!(int_f32_b0, Integer, f32, 1, 10, 1e-3);
    test_masking!(int_f32_b2, Integer, f32, 100, 10, 1e-3);
    test_masking!(int_f32_b4, Integer, f32, 10_000, 10, 1e-3);
    test_masking!(int_f32_b6, Integer, f32, 1_000_000, 10, 1e-3);
    test_masking!(int_f32_bmax, Integer, f32, 10, 1e-3);

    test_masking!(prime_f32_b0, Prime, f32, 1, 10, 1e-3);
    test_masking!(prime_f32_b2, Prime, f32, 100, 10, 1e-3);
    test_masking!(prime_f32_b4, Prime, f32, 10_000, 10, 1e-3);
    test_masking!(prime_f32_b6, Prime, f32, 1_000_000, 10, 1e-3);
    test_masking!(prime_f32_bmax, Prime, f32, 10, 1e-3);

    test_masking!(pow_f32_b0, Power2, f32, 1, 10, 1e-3);
    test_masking!(pow_f32_b2, Power2, f32, 100, 10, 1e-3);
    test_masking!(pow_f32_b4, Power2, f32, 10_000, 10, 1e-3);
    test_masking!(pow_f32_b6, Power2, f32, 1_000_000, 10, 1e-3);
    test_masking!(pow_f32_bmax, Power2, f32, 10, 1e-3);

    test_masking!(int_f64_b0, Integer, f64, 1, 10, 1e-3);
    test_masking!(int_f64_b2, Integer, f64, 100, 10, 1e-3);
    test_masking!(int_f64_b4, Integer, f64, 10_000, 10, 1e-3);
    test_masking!(int_f64_b6, Integer, f64, 1_000_000, 10, 1e-3);
    test_masking!(int_f64_bmax, Integer, f64, 10, 1e-3);

    test_masking!(prime_f64_b0, Prime, f64, 1, 10, 1e-3);
    test_masking!(prime_f64_b2, Prime, f64, 100, 10, 1e-3);
    test_masking!(prime_f64_b4, Prime, f64, 10_000, 10, 1e-3);
    test_masking!(prime_f64_b6, Prime, f64, 1_000_000, 10, 1e-3);
    test_masking!(prime_f64_bmax, Prime, f64, 10, 1e-3);

    test_masking!(pow_f64_b0, Power2, f64, 1, 10, 1e-3);
    test_masking!(pow_f64_b2, Power2, f64, 100, 10, 1e-3);
    test_masking!(pow_f64_b4, Power2, f64, 10_000, 10, 1e-3);
    test_masking!(pow_f64_b6, Power2, f64, 1_000_000, 10, 1e-3);
    test_masking!(pow_f64_bmax, Power2, f64, 10, 1e-3);

    test_masking!(int_i32_b0, Integer, i32, 1, 10, 1e-3);
    test_masking!(int_i32_b2, Integer, i32, 100, 10, 1e-3);
    test_masking!(int_i32_b4, Integer, i32, 10_000, 10, 1e-3);
    test_masking!(int_i32_b6, Integer, i32, 1_000_000, 10, 1e-3);
    test_masking!(int_i32_bmax, Integer, i32, 10, 1e-3);

    test_masking!(prime_i32_b0, Prime, i32, 1, 10, 1e-3);
    test_masking!(prime_i32_b2, Prime, i32, 100, 10, 1e-3);
    test_masking!(prime_i32_b4, Prime, i32, 10_000, 10, 1e-3);
    test_masking!(prime_i32_b6, Prime, i32, 1_000_000, 10, 1e-3);
    test_masking!(prime_i32_bmax, Prime, i32, 10, 1e-3);

    test_masking!(pow_i32_b0, Power2, i32, 1, 10, 1e-3);
    test_masking!(pow_i32_b2, Power2, i32, 100, 10, 1e-3);
    test_masking!(pow_i32_b4, Power2, i32, 10_000, 10, 1e-3);
    test_masking!(pow_i32_b6, Power2, i32, 1_000_000, 10, 1e-3);
    test_masking!(pow_i32_bmax, Power2, i32, 10, 1e-3);

    test_masking!(int_i64_b0, Integer, i64, 1, 10, 1e-3);
    test_masking!(int_i64_b2, Integer, i64, 100, 10, 1e-3);
    test_masking!(int_i64_b4, Integer, i64, 10_000, 10, 1e-3);
    test_masking!(int_i64_b6, Integer, i64, 1_000_000, 10, 1e-3);
    test_masking!(int_i64_bmax, Integer, i64, 10, 1e-3);

    test_masking!(prime_i64_b0, Prime, i64, 1, 10, 1e-3);
    test_masking!(prime_i64_b2, Prime, i64, 100, 10, 1e-3);
    test_masking!(prime_i64_b4, Prime, i64, 10_000, 10, 1e-3);
    test_masking!(prime_i64_b6, Prime, i64, 1_000_000, 10, 1e-3);
    test_masking!(prime_i64_bmax, Prime, i64, 10, 1e-3);

    test_masking!(pow_i64_b0, Power2, i64, 1, 10, 1e-3);
    test_masking!(pow_i64_b2, Power2, i64, 100, 10, 1e-3);
    test_masking!(pow_i64_b4, Power2, i64, 10_000, 10, 1e-3);
    test_masking!(pow_i64_b6, Power2, i64, 1_000_000, 10, 1e-3);
    test_masking!(pow_i64_bmax, Power2, i64, 10, 1e-3);
}
