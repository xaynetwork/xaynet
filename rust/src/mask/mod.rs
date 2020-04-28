pub mod config;
pub mod seed;

use std::convert::TryInto;

use num::{
    bigint::{BigInt, BigUint, ToBigInt},
    rational::Ratio,
    traits::float::FloatCore,
};

use self::config::{DataType, MaskConfig};
use crate::{model::Model, utils::ratio_as_float, PetError};

pub trait MaskIntegers: Sized {
    /// Get a reference to the integers.
    fn integers(&'_ self) -> &'_ Vec<BigUint>;

    /// Get a reference to the mask configuration.
    fn config(&'_ self) -> &'_ MaskConfig;

    /// Create mask integers from its parts. Fails if the integers don't conform to the mask
    /// configuration.
    fn from_parts(integers: Vec<BigUint>, config: MaskConfig) -> Result<Self, PetError>;

    /// Get the length of the serialized object.
    fn len(&self) -> usize {
        4 + self.integers().len() * self.config().element_len()
    }

    /// Serialize the mask integers into bytes.
    fn serialize(&self) -> Vec<u8> {
        let element_len = self.config().element_len();
        let bytes = self
            .integers()
            .iter()
            .flat_map(|integer| {
                let mut bytes = integer.to_bytes_le();
                bytes.resize(element_len, 0_u8);
                bytes
            })
            .collect();
        [self.config().serialize(), bytes].concat()
    }

    /// Deserialize the mask integers from bytes. Fails if the bytes don't conform to the mask
    /// configuration.
    fn deserialize(bytes: &[u8]) -> Result<Self, PetError> {
        if bytes.len() < 4 {
            return Err(PetError::InvalidMask);
        }
        let config = MaskConfig::deserialize(&bytes[..4])?;
        let element_len = config.element_len();
        if bytes[4..].len() % element_len != 0 {
            return Err(PetError::InvalidMask);
        }
        let integers = bytes[4..]
            .chunks_exact(element_len)
            .map(|chunk| BigUint::from_bytes_le(chunk))
            .collect::<Vec<BigUint>>();
        Self::from_parts(integers, config)
    }

    /// Aggregate the mask integers with other mask integers. Fails if the mask configurations or
    /// the integer sizes don't conform.
    fn aggregate(&self, other: &Self) -> Result<Self, PetError> {
        if self.integers().len() == other.integers().len() && self.config() == other.config() {
            let aggregated_integers = self
                .integers()
                .iter()
                .zip(other.integers().iter())
                .map(|(integer, other_integer)| (integer + other_integer) % self.config().order())
                .collect();
            Self::from_parts(aggregated_integers, self.config().clone())
        } else {
            Err(PetError::InvalidMask)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A masked model. Its parameters are represented as a vector of integers from a finite group wrt
/// a mask configuration.
pub struct MaskedModel {
    integers: Vec<BigUint>,
    config: MaskConfig,
}

impl MaskIntegers for MaskedModel {
    /// Get a reference to the integers of the masked model.
    fn integers(&'_ self) -> &'_ Vec<BigUint> {
        &self.integers
    }

    /// Get a reference to the mask configuration of the masked model.
    fn config(&'_ self) -> &'_ MaskConfig {
        &self.config
    }

    /// Create a masked model from its parts. Fails if the integers don't conform to the mask
    /// configuration.
    fn from_parts(integers: Vec<BigUint>, config: MaskConfig) -> Result<Self, PetError> {
        if integers.iter().all(|integer| integer < config.order()) {
            Ok(Self { integers, config })
        } else {
            Err(PetError::InvalidMask)
        }
    }
}

impl MaskedModel {
    /// Unmask the masked model with a mask. Fails if the mask configurations don't conform or the
    /// number of models is zero.
    pub fn unmask(&self, mask: &Mask, no_models: usize) -> Result<Model, PetError> {
        if no_models > 0 && mask.config() == self.config() {
            match self.config().name().data_type() {
                DataType::F32 => {
                    Self::shift_floats::<f32>(self.unmask_integers(mask), self.config(), no_models)
                        .try_into()
                }
                DataType::F64 => {
                    Self::shift_floats::<f64>(self.unmask_integers(mask), self.config(), no_models)
                        .try_into()
                }
            }
        } else {
            Err(PetError::InvalidMask)
        }
    }

    /// Unmask the masked integers with a mask.
    fn unmask_integers(&self, mask: &Mask) -> Vec<Ratio<BigInt>> {
        self.integers
            .iter()
            .zip(mask.integers().iter())
            .map(|(masked_weight, mask)| {
                Ratio::<BigInt>::from(
                    ((masked_weight + self.config.order() - mask) % self.config.order())
                        .to_bigint()
                        // safe unwrap: `to_bigint` never fails for `BigUint`s
                        .unwrap(),
                )
            })
            .collect::<Vec<Ratio<BigInt>>>()
    }

    /// Shift the integers into floats.
    fn shift_floats<F: FloatCore>(
        integers: Vec<Ratio<BigInt>>,
        config: &MaskConfig,
        no_models: usize,
    ) -> Vec<F> {
        let scaled_add_shift = config.add_shift() * BigInt::from(no_models);
        integers
            .iter()
            .map(|integer| {
                // shift the weight into the reals
                ratio_as_float(&(integer / config.exp_shift() - &scaled_add_shift))
            })
            .collect::<Vec<F>>()
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A mask. Its parameters are represented as a vector of integers from a finite group wrt a mask
/// configuration.
pub struct Mask {
    integers: Vec<BigUint>,
    config: MaskConfig,
}

impl MaskIntegers for Mask {
    /// Get a reference to the integers of the mask.
    fn integers(&'_ self) -> &'_ Vec<BigUint> {
        &self.integers
    }

    /// Get a reference to the mask configuration of the mask.
    fn config(&'_ self) -> &'_ MaskConfig {
        &self.config
    }

    /// Create a mask from its parts. Fails if the integers don't conform to the mask configuration.
    fn from_parts(integers: Vec<BigUint>, config: MaskConfig) -> Result<Self, PetError> {
        if integers.iter().all(|integer| integer < config.order()) {
            Ok(Self { integers, config })
        } else {
            Err(PetError::InvalidMask)
        }
    }
}

impl Mask {
    /// Unmask a masked model with the mask. Fails if the mask configurations don't conform or the
    /// number of models is zero.
    pub fn unmask(&self, masked_model: &MaskedModel, no_models: usize) -> Result<Model, PetError> {
        masked_model.unmask(&self, no_models)
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::TryFrom, iter};

    use rand::{
        distributions::{Distribution, Uniform},
        SeedableRng,
    };
    use rand_chacha::ChaCha20Rng;

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
        let (mask_seed, masked_model) = model.f32_unchecked().mask(1_f64, &config);
        assert_eq!(masked_model.integers().len(), 10);
        let mask = mask_seed.derive_mask(10, &config);
        let unmasked_model = masked_model.unmask(&mask, 1).unwrap();
        assert!(model
            .f32_unchecked()
            .weights()
            .iter()
            .zip(unmasked_model.f32_unchecked().weights().iter())
            .all(|(weight, unmasked_weight)| (weight - unmasked_weight).abs() < 1e-8_f32));
    }

    #[test]
    fn test_aggregation() {
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
        let other_integers = iter::repeat_with(|| generate_integer(&mut prng, config.order()))
            .take(10)
            .collect();
        let masked_model = MaskedModel::from_parts(integers, config.clone()).unwrap();
        let other_masked_model = MaskedModel::from_parts(other_integers, config.clone()).unwrap();
        let aggregated_masked_model = masked_model.aggregate(&other_masked_model).unwrap();
        assert_eq!(
            aggregated_masked_model.integers().len(),
            masked_model.integers().len(),
        );
        assert_eq!(
            aggregated_masked_model.integers().len(),
            other_masked_model.integers().len(),
        );
        assert_eq!(aggregated_masked_model.config(), &config);
        assert!(aggregated_masked_model
            .integers()
            .iter()
            .all(|integer| integer < config.order()));
    }

    #[test]
    fn test_masking_and_aggregation() {
        let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
        let uniform = Uniform::new(-1_f32, 1_f32);
        let weights = iter::repeat_with(|| uniform.sample(&mut prng))
            .take(10)
            .collect::<Vec<f32>>();
        let other_weights = iter::repeat_with(|| uniform.sample(&mut prng))
            .take(10)
            .collect::<Vec<f32>>();
        let model = Model::try_from(weights).unwrap();
        let other_model = Model::try_from(other_weights).unwrap();
        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        let (mask_seed, masked_model) = model.f32_unchecked().mask(0.5_f64, &config);
        let (other_mask_seed, other_masked_model) =
            other_model.f32_unchecked().mask(0.5_f64, &config);
        let aggregated_masked_model = masked_model.aggregate(&other_masked_model).unwrap();
        let aggregated_mask = mask_seed
            .derive_mask(10, &config)
            .aggregate(&other_mask_seed.derive_mask(10, &config))
            .unwrap();
        let aggregated_model = aggregated_masked_model.unmask(&aggregated_mask, 2).unwrap();
        let averaged_weights = model
            .f32_unchecked()
            .weights()
            .iter()
            .zip(other_model.f32_unchecked().weights().iter())
            .map(|(weight, other_weight)| 0.5 * weight + 0.5 * other_weight)
            .collect::<Vec<f32>>();
        assert!(aggregated_model
            .f32_unchecked()
            .weights()
            .iter()
            .zip(averaged_weights.iter())
            .all(
                |(aggregated_weight, averaged_weight)| (aggregated_weight - averaged_weight).abs()
                    < 1e-8_f32
            ));
    }

    #[test]
    fn test_serialization() {
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
}
