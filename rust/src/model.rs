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
