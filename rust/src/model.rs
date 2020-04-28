use std::convert::TryFrom;

use num::{
    bigint::{BigInt, BigUint},
    clamp,
    rational::Ratio,
    traits::float::FloatCore,
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::{
    mask::{config::MaskConfig, seed::MaskSeed, MaskIntegers, MaskedModel},
    utils::generate_integer,
    PetError,
};

#[derive(Clone, Debug, PartialEq)]
/// A model with parameters represented as a vector of numbers.
pub enum Model {
    F32(FloatModel<f32>),
    F64(FloatModel<f64>),
    // I32(IntModel<i32>),
    // I64(IntModel<i64>),
}

impl TryFrom<Vec<f32>> for Model {
    type Error = PetError;

    /// Create a model from its weights. Fails if the weights are not finite.
    fn try_from(weights: Vec<f32>) -> Result<Self, Self::Error> {
        if weights.iter().all(|weight| weight.is_finite()) {
            Ok(Self::F32(FloatModel::<f32> { weights }))
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
            Ok(Self::F64(FloatModel::<f64> { weights }))
        } else {
            Err(Self::Error::InvalidModel)
        }
    }
}

// impl TryFrom<Vec<i32>> for Model {
//     type Error = PetError;

//     /// Create a model from its weights. Fails if the weights are not finite.
//     fn try_from(weights: Vec<i32>) -> Result<Self, Self::Error> {
//         if weights.iter().all(|weight| weight.is_finite()) {
//             Ok(Self::I32(IntModel::<i32> { weights }))
//         } else {
//             Err(Self::Error::InvalidModel)
//         }
//     }
// }

// impl TryFrom<Vec<i64>> for Model {
//     type Error = PetError;

//     /// Create a model from its weights. Fails if the weights are not finite.
//     fn try_from(weights: Vec<i64>) -> Result<Self, Self::Error> {
//         if weights.iter().all(|weight| weight.is_finite()) {
//             Ok(Self::I64(IntModel::<i64> { weights }))
//         } else {
//             Err(Self::Error::InvalidModel)
//         }
//     }
// }

impl Model {
    pub fn f32(&'_ self) -> Option<&'_ FloatModel<f32>> {
        if let Self::F32(model) = self {
            Some(model)
        } else {
            None
        }
    }

    pub fn f32_unchecked(&'_ self) -> &'_ FloatModel<f32> {
        self.f32().unwrap()
    }

    pub fn f64(&'_ self) -> Option<&'_ FloatModel<f64>> {
        if let Self::F64(model) = self {
            Some(model)
        } else {
            None
        }
    }

    pub fn f64_unchecked(&'_ self) -> &'_ FloatModel<f64> {
        self.f64().unwrap()
    }

    // pub fn i32(&'_ self) -> Option<&'_ IntModel<i32>> {
    //     if let Self::I32(model) = self {
    //         Some(model)
    //     } else {
    //         None
    //     }
    // }

    // pub fn i32_unchecked(&'_ self) -> &'_ IntModel<i32> {
    //     self.i32().unwrap()
    // }

    // pub fn i64(&'_ self) -> Option<&'_ IntModel<i64>> {
    //     if let Self::I64(model) = self {
    //         Some(model)
    //     } else {
    //         None
    //     }
    // }

    // pub fn i64_unchecked(&'_ self) -> &'_ IntModel<i64> {
    //     self.i64().unwrap()
    // }
}

#[derive(Clone, Debug, PartialEq)]
/// A model with parameters represented as a vector of floats.
pub struct FloatModel<F: FloatCore> {
    weights: Vec<F>,
}

impl<F: FloatCore> FloatModel<F> {
    /// Get a reference to the model weights.
    pub fn weights(&'_ self) -> &'_ Vec<F> {
        &self.weights
    }

    /// Mask the model wrt the mask configuration. Enforces bounds on the scalar and weights.
    pub fn mask(&self, scalar: f64, config: &MaskConfig) -> (MaskSeed, MaskedModel) {
        // safe unwrap: clamped scalar is finite
        let scalar = &Ratio::<BigInt>::from_float(clamp(scalar, 0_f64, 1_f64)).unwrap();
        let negative_bound = &-config.add_shift();
        let positive_bound = config.add_shift();
        let mask_seed = MaskSeed::generate();
        let mut prng = ChaCha20Rng::from_seed(mask_seed.as_array());
        let integers = self
            .weights
            .iter()
            .map(|weight| {
                // clamp, scale and shift the weight into the non-negative integers
                let integer = (((scalar
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
                .unwrap();
                // shift the masked weight into the finite group
                let masked_weight =
                    (integer + generate_integer(&mut prng, config.order())) % config.order();
                masked_weight
            })
            .collect::<Vec<BigUint>>();
        // safe unwrap: integer conformity is guaranteed by modulo operation during shifting
        let masked_model = MaskedModel::from_parts(integers, config.clone()).unwrap();
        (mask_seed, masked_model)
    }
}
