use std::convert::TryFrom;

use num::{bigint::BigInt, clamp, rational::Ratio, traits::float::FloatCore};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::{
    mask::{
        config::{DataType, MaskConfig},
        seed::MaskSeed,
        MaskIntegers,
        MaskedModel,
    },
    utils::generate_integer,
    PetError,
};

#[derive(Clone, Debug, PartialEq)]
/// A model with parameters represented as a vector of numbers.
pub enum Model {
    F32(Vec<f32>),
    F64(Vec<f64>),
    I32(Vec<i32>),
    I64(Vec<i64>),
}

impl TryFrom<Vec<f32>> for Model {
    type Error = PetError;

    /// Create a model from its weights. Fails if the weights are not finite.
    fn try_from(weights: Vec<f32>) -> Result<Self, Self::Error> {
        if weights.iter().all(|weight| weight.is_finite()) {
            Ok(Self::F32(weights))
        } else {
            Err(Self::Error::InvalidModel)
        }
    }
}

impl TryFrom<Vec<f64>> for Model {
    type Error = PetError;

    /// Create a model from its weights. Fails if the weights are not finite.
    fn try_from(weights: Vec<f64>) -> Result<Self, Self::Error> {
        if weights.iter().all(|weight| weight.is_finite()) {
            Ok(Self::F64(weights))
        } else {
            Err(Self::Error::InvalidModel)
        }
    }
}

impl From<Vec<i32>> for Model {
    /// Create a model from its weights.
    fn from(weights: Vec<i32>) -> Self {
        Self::I32(weights)
    }
}

impl From<Vec<i64>> for Model {
    /// Create a model from its weights. Fails if the weights are not finite.
    fn from(weights: Vec<i64>) -> Self {
        Self::I64(weights)
    }
}

impl Model {
    /// Mask the model wrt the mask configuration. Enforces bounds on the scalar and weights. Fails
    /// if the mask configuration doesn't conform to the model data type.
    pub fn mask(
        &self,
        scalar: f64,
        config: &MaskConfig,
    ) -> Result<(MaskSeed, MaskedModel), PetError> {
        match self {
            Model::F32(weights) => {
                if let DataType::F32 = config.name().data_type() {
                    // safe unwrap: `weights` are guaranteed to be finite because of `try_from`
                    Self::mask_numbers(Self::floats_as_ratios(weights).unwrap(), scalar, config)
                } else {
                    Err(PetError::InvalidMask)
                }
            }
            Model::F64(weights) => {
                if let DataType::F64 = config.name().data_type() {
                    // safe unwrap: `weights` are guaranteed to be finite because of `try_from`
                    Self::mask_numbers(Self::floats_as_ratios(weights).unwrap(), scalar, config)
                } else {
                    Err(PetError::InvalidMask)
                }
            }
            Model::I32(weights) => {
                if let DataType::I32 = config.name().data_type() {
                    Self::mask_numbers(Self::i32s_as_ratios(weights), scalar, config)
                } else {
                    Err(PetError::InvalidMask)
                }
            }
            Model::I64(weights) => {
                if let DataType::I64 = config.name().data_type() {
                    Self::mask_numbers(Self::i64s_as_ratios(weights), scalar, config)
                } else {
                    Err(PetError::InvalidMask)
                }
            }
        }
    }

    /// Mask the numbers wrt the mask configuration. Enforces bounds on the scalar and numbers.
    fn mask_numbers(
        numbers: Vec<Ratio<BigInt>>,
        scalar: f64,
        config: &MaskConfig,
        // ) -> Vec<BigUint> {
    ) -> Result<(MaskSeed, MaskedModel), PetError> {
        let scalar = &Ratio::<BigInt>::from_float(clamp(scalar, 0_f64, 1_f64)).unwrap();
        let negative_bound = &-config.add_shift();
        let positive_bound = config.add_shift();
        let mask_seed = MaskSeed::generate();
        let mut prng = ChaCha20Rng::from_seed(mask_seed.as_array());
        let masked_numbers = numbers
            .iter()
            .map(|number| {
                let scaled = scalar * clamp(number, negative_bound, positive_bound);
                let shifted = ((scaled + config.add_shift()) * config.exp_shift())
                    .to_integer()
                    .to_biguint()
                    // safe unwrap: shifted weight is guaranteed to be non-negative
                    .unwrap();
                let masked =
                    (shifted + generate_integer(&mut prng, config.order())) % config.order();
                masked
            })
            .collect();
        let masked_model = MaskedModel::from_parts(masked_numbers, config.clone())?;
        Ok((mask_seed, masked_model))
    }

    /// Cast floats as ratios. Fails if any float is not finite.
    fn floats_as_ratios<F: FloatCore>(floats: &Vec<F>) -> Option<Vec<Ratio<BigInt>>> {
        floats
            .iter()
            .map(|float| Ratio::<BigInt>::from_float(*float))
            .collect()
    }

    /// Cast i32 integers as ratios.
    fn i32s_as_ratios(ints: &Vec<i32>) -> Vec<Ratio<BigInt>> {
        ints.iter()
            .map(|int| Ratio::from_integer(BigInt::from(*int)))
            .collect()
    }

    /// Cast i64 integers as ratios.
    fn i64s_as_ratios(ints: &Vec<i64>) -> Vec<Ratio<BigInt>> {
        ints.iter()
            .map(|int| Ratio::from_integer(BigInt::from(*int)))
            .collect()
    }
}
