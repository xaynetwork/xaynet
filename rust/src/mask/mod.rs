pub mod config;
pub mod seed;

use std::convert::{TryFrom, TryInto};

use num::{
    bigint::{BigInt, BigUint, ToBigInt},
    clamp,
    rational::Ratio,
    traits::float::FloatCore,
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use self::{config::MaskConfig, seed::MaskSeed};
use crate::{
    utils::{generate_integer, ratio_as},
    PetError,
};

#[derive(Clone, Debug, PartialEq)]
/// A model. Its parameters are represented as a vector of numerical values.
pub struct Model<F: FloatCore> {
    weights: Vec<F>,
}

impl<F: FloatCore> TryFrom<Vec<F>> for Model<F> {
    type Error = PetError;

    /// Create a model from its weights. Fails if the weights are not finite.
    fn try_from(weights: Vec<F>) -> Result<Self, Self::Error> {
        if weights.iter().all(|weight| weight.is_finite()) {
            Ok(Self { weights })
        } else {
            Err(Self::Error::InvalidMessage)
        }
    }
}

impl<F: FloatCore> Model<F> {
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

#[derive(Clone, Debug, PartialEq)]
/// A masked model. Its parameters are represented as a vector of integers from a finite group wrt
/// a mask configuration.
pub struct MaskedModel {
    integers: Vec<BigUint>,
    config: MaskConfig,
}

impl MaskedModel {
    /// Get a reference to the integers of the masked model.
    pub fn integers(&'_ self) -> &'_ Vec<BigUint> {
        &self.integers
    }

    /// Get a reference to the mask configuration of the masked model.
    pub fn config(&'_ self) -> &'_ MaskConfig {
        &self.config
    }

    /// Create a masked model from its parts. Fails if the integers don't conform to the mask
    /// configuration.
    pub fn from_parts(integers: Vec<BigUint>, config: MaskConfig) -> Result<Self, PetError> {
        if integers.iter().all(|integer| integer < config.order()) {
            Ok(Self { integers, config })
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Get the length of the serialized masked model.
    pub fn len(&self) -> usize {
        4 + self.integers.len() * self.config.element_len()
    }

    /// Serialize the masked model into bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let element_len = self.config.element_len();
        let bytes = self
            .integers
            .iter()
            .flat_map(|integer| {
                let mut bytes = integer.to_bytes_le();
                bytes.resize(element_len, 0_u8);
                bytes
            })
            .collect();
        [self.config.serialize(), bytes].concat()
    }

    /// Deserialize the masked model from bytes. Fails if the bytes don't conform to the mask
    /// configuration.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, PetError> {
        if bytes.len() < 4 {
            return Err(PetError::InvalidMessage);
        }
        let config = MaskConfig::deserialize(&bytes[..4])?;
        let element_len = config.element_len();
        if bytes[4..].len() % element_len != 0 {
            return Err(PetError::InvalidMessage);
        }
        let integers = bytes[4..]
            .chunks_exact(element_len)
            .map(|chunk| BigUint::from_bytes_le(chunk))
            .collect::<Vec<BigUint>>();
        Self::from_parts(integers, config)
    }

    /// Unmask the masked model with a mask. Requires the total positive number of models, fails
    /// otherwise.
    pub fn unmask<F: FloatCore>(
        &self,
        mask: &Mask,
        no_models: usize,
    ) -> Result<Model<F>, PetError> {
        if no_models == 0 {
            return Err(PetError::InvalidMessage);
        }
        let scaled_add_shift = self.config.add_shift() * BigInt::from(no_models);
        let weights = self
            .integers
            .iter()
            .zip(mask.integers().iter())
            .map(|(masked_weight, mask)| {
                // unmask the masked weight
                let integer = Ratio::<BigInt>::from(
                    ((masked_weight + self.config.order() - mask) % self.config.order())
                        .to_bigint()
                        // safe unwrap: `to_bigint` never fails for `BigUint`s
                        .unwrap(),
                );
                // shift the weight into the reals
                let weight =
                    ratio_as::<F>(&(integer / self.config.exp_shift() - &scaled_add_shift));
                weight
            })
            .collect::<Vec<F>>();
        weights.try_into()
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A mask. Its parameters are represented as a vector of integers from a finite group wrt a mask
/// configuration.
pub struct Mask {
    integers: Vec<BigUint>,
    config: MaskConfig,
}

impl Mask {
    /// Get a reference to the integers of the mask.
    pub fn integers(&'_ self) -> &'_ Vec<BigUint> {
        &self.integers
    }

    /// Get a reference to the mask configuration of the mask.
    pub fn config(&'_ self) -> &'_ MaskConfig {
        &self.config
    }

    /// Create a mask from its parts. Fails if the integers don't conform to the mask configuration.
    pub fn from_parts(integers: Vec<BigUint>, config: MaskConfig) -> Result<Self, PetError> {
        if integers.iter().all(|integer| integer < config.order()) {
            Ok(Self { integers, config })
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Get the length of the serialized masked model.
    pub fn len(&self) -> usize {
        4 + self.integers.len() * self.config.element_len()
    }

    /// Serialize the mask into bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let element_len = self.config.element_len();
        let bytes = self
            .integers
            .iter()
            .flat_map(|integer| {
                let mut bytes = integer.to_bytes_le();
                bytes.resize(element_len, 0_u8);
                bytes
            })
            .collect();
        [self.config.serialize(), bytes].concat()
    }

    /// Deserialize the mask from bytes. Fails if the bytes don't conform to the mask configuration.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, PetError> {
        if bytes.len() < 4 {
            return Err(PetError::InvalidMessage);
        }
        let config = MaskConfig::deserialize(&bytes[..4])?;
        let element_len = config.element_len();
        if bytes[4..].len() % element_len != 0 {
            return Err(PetError::InvalidMessage);
        }
        let integers = bytes[4..]
            .chunks_exact(element_len)
            .map(|chunk| BigUint::from_bytes_le(chunk))
            .collect::<Vec<BigUint>>();
        Self::from_parts(integers, config)
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use rand::distributions::{Distribution, Uniform};

    use super::*;
    use crate::{
        mask::config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
        utils::generate_integer,
    };

    #[test]
    fn test_masking() {
        let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
        let uniform = Uniform::new(-1_f32, 1_f32);
        let weights = iter::repeat_with(|| uniform.sample(&mut prng))
            .take(10)
            .collect::<Vec<f32>>();
        let model = Model::try_from(weights).unwrap();
        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        let (mask_seed, masked_model) = model.mask(1_f64, &config);
        assert_eq!(masked_model.integers().len(), 10);
        let mask = mask_seed.derive_mask(10, &config);
        let unmasked_model = masked_model.unmask::<f32>(&mask, 1).unwrap();
        assert!(model
            .weights()
            .iter()
            .zip(unmasked_model.weights().iter())
            .all(|(weight, unmasked_weight)| (weight - unmasked_weight).abs() < 1e-8_f32));
    }

    #[test]
    fn test_maskedmodel_serialization() {
        let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        let integers = iter::repeat_with(|| generate_integer(&mut prng, config.order()))
            .take(10)
            .collect();
        let masked_model = MaskedModel::from_parts(integers, config).unwrap();
        assert_eq!(masked_model.len(), 64);
        let serialized = masked_model.serialize();
        assert_eq!(serialized.len(), 64);
        let deserialized = MaskedModel::deserialize(serialized.as_slice()).unwrap();
        assert_eq!(masked_model, deserialized);
    }

    #[test]
    fn test_mask_serialization() {
        let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        let integers = iter::repeat_with(|| generate_integer(&mut prng, config.order()))
            .take(10)
            .collect();
        let mask = Mask::from_parts(integers, config).unwrap();
        assert_eq!(mask.len(), 64);
        let serialized = mask.serialize();
        assert_eq!(serialized.len(), 64);
        let deserialized = Mask::deserialize(serialized.as_slice()).unwrap();
        assert_eq!(mask, deserialized);
    }
}
