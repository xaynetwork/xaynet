use std::convert::TryFrom;

use num::{
    bigint::{BigInt, BigUint},
    clamp,
    rational::Ratio,
    traits::{float::FloatCore, int::PrimInt},
};
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
    // I32(Vec<i32>),
    // I64(Vec<i64>),
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

// impl From<Vec<i32>> for Model {
//     /// Create a model from its weights.
//     fn from(weights: Vec<i32>) -> Self {
//         Self::I32(weights)
//     }
// }

// impl From<Vec<i64>> for Model {
//     /// Create a model from its weights. Fails if the weights are not finite.
//     fn from(weights: Vec<i64>) -> Self {
//         Self::I64(weights)
//     }
// }

impl Model {
    /// Get a reference to the weights. Fails for nonconforming data types.
    pub fn weights_f32(&'_ self) -> Option<&'_ Vec<f32>> {
        if let Self::F32(weights) = self {
            Some(weights)
        } else {
            None
        }
    }

    /// Get a reference to the weights. Panics for nonconforming data types.
    pub fn weights_f32_unchecked(&'_ self) -> &'_ Vec<f32> {
        self.weights_f32().unwrap()
    }

    /// Get a reference to the weights. Fails for nonconforming data types.
    pub fn weights_f64(&'_ self) -> Option<&'_ Vec<f64>> {
        if let Self::F64(weights) = self {
            Some(weights)
        } else {
            None
        }
    }

    /// Get a reference to the weights. Panics for nonconforming data types.
    pub fn weights_f64_unchecked(&'_ self) -> &'_ Vec<f64> {
        self.weights_f64().unwrap()
    }

    // /// Get a reference to the weights. Fails for nonconforming data types.
    // pub fn weights_i32(&'_ self) -> Option<&'_ Vec<i32>> {
    //     if let Self::I32(weights) = self {
    //         Some(weights)
    //     } else {
    //         None
    //     }
    // }

    // /// Get a reference to the weights. Panics for nonconforming data types.
    // pub fn weights_i32_unchecked(&'_ self) -> &'_ Vec<i32> {
    //     self.weights_i32().unwrap()
    // }

    // /// Get a reference to the weights. Fails for nonconforming data types.
    // pub fn weights_i64(&'_ self) -> Option<&'_ Vec<i64>> {
    //     if let Self::I64(weights) = self {
    //         Some(weights)
    //     } else {
    //         None
    //     }
    // }

    // /// Get a reference to the weights. Panics for nonconforming data types.
    // pub fn weights_i64_unchecked(&'_ self) -> &'_ Vec<i64> {
    //     self.weights_i64().unwrap()
    // }

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
                    Self::mask_integers(Self::shift_floats(weights, scalar, config), config)
                } else {
                    Err(PetError::InvalidMask)
                }
            }
            Model::F64(weights) => {
                if let DataType::F64 = config.name().data_type() {
                    Self::mask_integers(Self::shift_floats(weights, scalar, config), config)
                } else {
                    Err(PetError::InvalidMask)
                }
            } // Model::I32(weights) => {
              //     if let DataType::I32 = config.name().data_type() {
              //         Self::mask_integers(Self::shift_ints(weights, scalar, config), config)
              //     } else {
              //         Err(PetError::AmbiguousMasks)
              //     }
              // }
              // Model::I64(weights) => {
              //     if let DataType::I64 = config.name().data_type() {
              //         Self::mask_integers(Self::shift_ints(weights, scalar, config), config)
              //     } else {
              //         Err(PetError::AmbiguousMasks)
              //     }
              // }
        }
    }

    /// Shift the float weights into non-negative integers. Enforces bounds on the scalar and
    /// weights.
    fn shift_floats<F: FloatCore>(
        weights: &Vec<F>,
        scalar: f64,
        config: &MaskConfig,
    ) -> Vec<BigUint> {
        let scalar = &Ratio::<BigInt>::from_float(clamp(scalar, 0_f64, 1_f64)).unwrap();
        let negative_bound = &-config.add_shift();
        let positive_bound = config.add_shift();
        weights
            .iter()
            .map(|weight| {
                (((scalar
                    * clamp(
                        // safe unwrap: `weight` is guaranteed to be finite because of `try_from`
                        &Ratio::<BigInt>::from_float(*weight).unwrap(),
                        negative_bound,
                        positive_bound,
                    ))
                    + config.add_shift())
                    * config.exp_shift())
                .to_integer()
                .to_biguint()
                // safe unwrap: shifted weight is guaranteed to be non-negative
                .unwrap()
            })
            .collect()
    }

    // /// Shift the integer weights into non-negative integers. Enforces bounds on the scalar and
    // /// weights.
    // fn shift_ints<I: PrimInt>(
    //     weights: &Vec<I>,
    //     scalar: f64,
    //     config: &MaskConfig,
    // ) -> Vec<BigUint> {}

    /// Mask the integers wrt the mask configuration.
    fn mask_integers(
        integers: Vec<BigUint>,
        config: &MaskConfig,
    ) -> Result<(MaskSeed, MaskedModel), PetError> {
        let mask_seed = MaskSeed::generate();
        let mut prng = ChaCha20Rng::from_seed(mask_seed.as_array());
        let masked_integers = integers
            .iter()
            .map(|integer| (integer + generate_integer(&mut prng, config.order())) % config.order())
            .collect();
        let masked_model = MaskedModel::from_parts(masked_integers, config.clone())?;
        Ok((mask_seed, masked_model))
    }
}
