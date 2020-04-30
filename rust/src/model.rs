use std::convert::TryFrom;

use num::{bigint::BigInt, clamp, rational::Ratio, traits::identities::Zero};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::{
    mask::{config::MaskConfig, seed::MaskSeed, Integers, MaskedModel},
    utils::generate_integer,
    PetError,
};

/// Masking of models.
pub trait MaskModels<N> {
    /// Get a reference to the weights.
    fn weights(&self) -> &Vec<N>;

    /// Cast the weights as ratios. Must handle non-finite weights.
    fn as_ratios(&self) -> Vec<Ratio<BigInt>>;

    /// Mask the model wrt the mask configuration. Enforces bounds on the scalar and weights.
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

#[derive(Clone, Debug, PartialEq)]
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

    /// Cast the weights as ratios. Positve/negative infinity is mapped to max/min and NaN to zero.
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
        // try from
        let weights = vec![-1_f32, 0_f32, 1_f32];
        let neg_inf_weights = vec![f32::NEG_INFINITY, 0_f32, 1_f32];
        let nan_weights = vec![-1_f32, f32::NAN, 1_f32];
        let inf_weights = vec![-1_f32, 0_f32, f32::INFINITY];
        let mut model = Model::<f32>::try_from(weights.clone()).unwrap();
        assert_eq!(model.weights(), &weights);
        assert_eq!(
            Model::<f32>::try_from(inf_weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );
        assert_eq!(
            Model::<f32>::try_from(neg_inf_weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );
        assert_eq!(
            Model::<f32>::try_from(nan_weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );

        // as ratio
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f32).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f32).unwrap(),
            ],
        );
        model.weights = neg_inf_weights;
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(f32::MIN).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f32).unwrap(),
            ],
        );
        model.weights = nan_weights;
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f32).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f32).unwrap(),
            ],
        );
        model.weights = inf_weights;
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f32).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(f32::MAX).unwrap(),
            ],
        );
    }

    #[test]
    fn test_model_f64() {
        // try from
        let weights = vec![-1_f64, 0_f64, 1_f64];
        let neg_inf_weights = vec![f64::NEG_INFINITY, 0_f64, 1_f64];
        let nan_weights = vec![-1_f64, f64::NAN, 1_f64];
        let inf_weights = vec![-1_f64, 0_f64, f64::INFINITY];
        let mut model = Model::<f64>::try_from(weights.clone()).unwrap();
        assert_eq!(model.weights(), &weights);
        assert_eq!(
            Model::<f64>::try_from(inf_weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );
        assert_eq!(
            Model::<f64>::try_from(neg_inf_weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );
        assert_eq!(
            Model::<f64>::try_from(nan_weights.clone()).unwrap_err(),
            PetError::InvalidModel,
        );

        // as ratio
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f64).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f64).unwrap(),
            ],
        );
        model.weights = neg_inf_weights;
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(f64::MIN).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f64).unwrap(),
            ],
        );
        model.weights = nan_weights;
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f64).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(1_f64).unwrap(),
            ],
        );
        model.weights = inf_weights;
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::<BigInt>::from_float(-1_f64).unwrap(),
                Ratio::<BigInt>::zero(),
                Ratio::<BigInt>::from_float(f64::MAX).unwrap(),
            ],
        );
    }

    #[test]
    fn test_model_i32() {
        // from
        let weights = vec![-1_i32, 0_i32, 1_i32];
        let model = Model::<i32>::from(weights.clone());
        assert_eq!(model.weights(), &weights);

        // as ratio
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
        // from
        let weights = vec![-1_i64, 0_i64, 1_i64];
        let model = Model::<i64>::from(weights.clone());
        assert_eq!(model.weights(), &weights);

        // as ratio
        assert_eq!(
            model.as_ratios(),
            vec![
                Ratio::from_integer(BigInt::from(-1_i64)),
                Ratio::<BigInt>::zero(),
                Ratio::from_integer(BigInt::from(1_i64)),
            ],
        );
    }

    // for float types the error depends on the order of magnitude of the weights => raise the
    // tolerance or bound the random weights if this test fails to often
    macro_rules! test_masking {
        // todo: remove the config argument when mask configurations can be derived programmatically
        ($($name:ident, $type:ty, $min:expr, $max:expr, $len:expr, $tol:expr, $config:expr $(,)?)?) => {
            $(
                #[test]
                fn $name() {
                    let mut prng = ChaCha20Rng::from_seed(MaskSeed::generate().as_array());
                    let uniform = Uniform::new($min, $max);
                    let weights = iter::repeat_with(|| uniform.sample(&mut prng))
                        .take($len)
                        .collect::<Vec<$type>>();
                    let model = Model::try_from(weights).unwrap();
                    let (mask_seed, masked_model) = model.mask(1_f64, &$config);
                    assert_eq!(masked_model.integers().len(), $len);
                    let mask = mask_seed.derive_mask($len, &$config);
                    let unmasked_model: Model<$type> = masked_model.unmask(&mask, 1_usize).unwrap();
                    assert!(model
                        .weights()
                        .iter()
                        .zip(unmasked_model.weights().iter())
                        .all(|(weight, unmasked_weight)| (weight - unmasked_weight).abs() <= $tol));
                }
            )?
        };
    }

    test_masking!(
        test_masking_f32_b0,
        f32,
        -1_f32,
        1_f32,
        10_usize,
        1e-3_f32,
        MaskConfigs::from_parts(Prime, F32, B0, M3).config(),
    );

    test_masking!(
        test_masking_f32_b2,
        f32,
        -100_f32,
        100_f32,
        10_usize,
        1e-3_f32,
        MaskConfigs::from_parts(Prime, F32, B2, M3).config(),
    );

    test_masking!(
        test_masking_f32_b4,
        f32,
        -10_000_f32,
        10_000_f32,
        10_usize,
        1e-3_f32,
        MaskConfigs::from_parts(Prime, F32, B4, M3).config(),
    );

    test_masking!(
        test_masking_f32_b6,
        f32,
        -1_000_000_f32,
        1_000_000_f32,
        10_usize,
        1e-3_f32,
        MaskConfigs::from_parts(Prime, F32, B6, M3).config(),
    );

    test_masking!(
        test_masking_f32_bmax,
        f32,
        f32::MIN,
        f32::MAX,
        10_usize,
        1e-3_f32,
        MaskConfigs::from_parts(Prime, F32, Bmax, M3).config(),
    );

    test_masking!(
        test_masking_f64_b0,
        f64,
        -1_f64,
        1_f64,
        10_usize,
        1e-6_f64,
        MaskConfigs::from_parts(Prime, F64, B0, M3).config(),
    );

    test_masking!(
        test_masking_f64_b2,
        f64,
        -100_f64,
        100_f64,
        10_usize,
        1e-6_f64,
        MaskConfigs::from_parts(Prime, F64, B2, M3).config(),
    );

    test_masking!(
        test_masking_f64_b4,
        f64,
        -10_000_f64,
        10_000_f64,
        10_usize,
        1e-6_f64,
        MaskConfigs::from_parts(Prime, F64, B4, M3).config(),
    );

    test_masking!(
        test_masking_f64_b6,
        f64,
        -1_000_000_f64,
        1_000_000_f64,
        10_usize,
        1e-6_f64,
        MaskConfigs::from_parts(Prime, F64, B6, M3).config(),
    );

    test_masking!(
        test_masking_f64_bmax,
        f64,
        f64::MIN,
        f64::MAX,
        10_usize,
        1e-6_f64,
        MaskConfigs::from_parts(Prime, F64, Bmax, M3).config(),
    );

    test_masking!(
        test_masking_i32_b0,
        i32,
        -1_i32,
        1_i32,
        10_usize,
        0_i32,
        MaskConfigs::from_parts(Prime, I32, B0, M3).config(),
    );

    test_masking!(
        test_masking_i32_b2,
        i32,
        -100_i32,
        100_i32,
        10_usize,
        0_i32,
        MaskConfigs::from_parts(Prime, I32, B2, M3).config(),
    );

    test_masking!(
        test_masking_i32_b4,
        i32,
        -10_000_i32,
        10_000_i32,
        10_usize,
        0_i32,
        MaskConfigs::from_parts(Prime, I32, B4, M3).config(),
    );

    test_masking!(
        test_masking_i32_b6,
        i32,
        -1_000_000_i32,
        1_000_000_i32,
        10_usize,
        0_i32,
        MaskConfigs::from_parts(Prime, I32, B6, M3).config(),
    );

    test_masking!(
        test_masking_i32_bmax,
        i32,
        i32::MIN,
        i32::MAX,
        10_usize,
        0_i32,
        MaskConfigs::from_parts(Prime, I32, Bmax, M3).config(),
    );

    test_masking!(
        test_masking_i64_b0,
        i64,
        -1_i64,
        1_i64,
        10_usize,
        0_i64,
        MaskConfigs::from_parts(Prime, I64, B0, M3).config(),
    );

    test_masking!(
        test_masking_i64_b2,
        i64,
        -100_i64,
        100_i64,
        10_usize,
        0_i64,
        MaskConfigs::from_parts(Prime, I64, B2, M3).config(),
    );

    test_masking!(
        test_masking_i64_b4,
        i64,
        -10_000_i64,
        10_000_i64,
        10_usize,
        0_i64,
        MaskConfigs::from_parts(Prime, I64, B4, M3).config(),
    );

    test_masking!(
        test_masking_i64_b6,
        i64,
        -1_000_000_i64,
        1_000_000_i64,
        10_usize,
        0_i64,
        MaskConfigs::from_parts(Prime, I64, B6, M3).config(),
    );

    test_masking!(
        test_masking_i64_bmax,
        i64,
        i64::MIN,
        i64::MAX,
        10_usize,
        0_i64,
        MaskConfigs::from_parts(Prime, I64, Bmax, M3).config(),
    );
}
