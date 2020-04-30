pub mod config;
pub mod seed;

use std::convert::TryInto;

use num::{
    bigint::{BigInt, BigUint, ToBigInt},
    rational::Ratio,
    traits::{cast::ToPrimitive, float::FloatCore},
};

use self::config::{DataType, MaskConfig};
use crate::{model::Model, PetError};

/// Aggregation and serialization for vectors of arbitrarily large integers.
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
        match self.config().name().data_type() {
            DataType::F32 => {
                let numbers = self.unmask_numbers(mask, self.config(), no_models)?;
                Self::ratios_as_floats::<f32>(numbers).try_into()
            }
            DataType::F64 => {
                let numbers = self.unmask_numbers(mask, self.config(), no_models)?;
                Self::ratios_as_floats::<f64>(numbers).try_into()
            }
            DataType::I32 => {
                let numbers = self.unmask_numbers(mask, self.config(), no_models)?;
                // safe ok: or should never happen because of shifting
                Ok(Self::ratios_as_i32s(numbers)
                    .ok_or(PetError::InvalidMask)?
                    .into())
            }
            DataType::I64 => {
                let numbers = self.unmask_numbers(mask, self.config(), no_models)?;
                // safe ok: or should never happen because of shifting
                Ok(Self::ratios_as_i64s(numbers)
                    .ok_or(PetError::InvalidMask)?
                    .into())
            }
        }
    }

    /// Unmask the masked numbers with a mask. Fails if the mask configurations don't conform or the
    /// number of models is zero.
    fn unmask_numbers(
        &self,
        mask: &Mask,
        config: &MaskConfig,
        no_models: usize,
    ) -> Result<Vec<Ratio<BigInt>>, PetError> {
        if no_models > 0 && self.config() == mask.config() {
            let scaled_add_shift = config.add_shift() * BigInt::from(no_models);
            let numbers = self
                .integers
                .iter()
                .zip(mask.integers().iter())
                .map(|(masked_weight, mask)| {
                    let unmasked = Ratio::<BigInt>::from(
                        ((masked_weight + self.config.order() - mask) % self.config.order())
                            .to_bigint()
                            // safe unwrap: `to_bigint` never fails for `BigUint`s
                            .unwrap(),
                    );
                    unmasked / config.exp_shift() - &scaled_add_shift
                })
                .collect::<Vec<Ratio<BigInt>>>();
            Ok(numbers)
        } else {
            Err(PetError::InvalidMask)
        }
    }

    /// Cast ratios as floats.
    fn ratios_as_floats<F: FloatCore>(ratios: Vec<Ratio<BigInt>>) -> Vec<F> {
        ratios
            .iter()
            .map(|ratio| {
                let mut numer = ratio.numer().clone();
                let mut denom = ratio.denom().clone();
                // safe loop: terminates after at most bitlength of ratio iterations
                loop {
                    if let (Some(n), Some(d)) = (F::from(numer.clone()), F::from(denom.clone())) {
                        if d == F::zero() {
                            return F::zero();
                        } else {
                            let float = n / d;
                            if float.is_finite() {
                                return float;
                            }
                        }
                    } else {
                        numer >>= 1_usize;
                        denom >>= 1_usize;
                    }
                }
            })
            .collect()
    }

    /// Cast ratios as i32 integers. Fails if any ratio overflows i32.
    fn ratios_as_i32s(ratios: Vec<Ratio<BigInt>>) -> Option<Vec<i32>> {
        ratios
            .iter()
            .map(|ratio| (ratio.to_integer().to_i32()))
            .collect()
    }

    /// Cast ratios as i64 integers. Fails if any ratio overflows i64.
    fn ratios_as_i64s(ratios: Vec<Ratio<BigInt>>) -> Option<Vec<i64>> {
        ratios
            .iter()
            .map(|ratio| (ratio.to_integer().to_i64()))
            .collect()
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
        let (mask_seed, masked_model) = model.mask(1_f64, &config).unwrap();
        assert_eq!(masked_model.integers().len(), 10);
        let mask = mask_seed.derive_mask(10, &config);
        let unmasked_model = masked_model.unmask(&mask, 1).unwrap();
        let weights = if let Model::F32(weights) = model {
            weights
        } else {
            panic!()
        };
        let unmasked_weights = if let Model::F32(weights) = unmasked_model {
            weights
        } else {
            panic!()
        };
        assert!(weights
            .iter()
            .zip(unmasked_weights.iter())
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
        let (mask_seed, masked_model) = model.mask(0.5_f64, &config).unwrap();
        let (other_mask_seed, other_masked_model) = other_model.mask(0.5_f64, &config).unwrap();
        let aggregated_masked_model = masked_model.aggregate(&other_masked_model).unwrap();
        let aggregated_mask = mask_seed
            .derive_mask(10, &config)
            .aggregate(&other_mask_seed.derive_mask(10, &config))
            .unwrap();
        let aggregated_model = aggregated_masked_model.unmask(&aggregated_mask, 2).unwrap();
        let weights = if let Model::F32(weights) = model {
            weights
        } else {
            panic!()
        };
        let other_weights = if let Model::F32(weights) = other_model {
            weights
        } else {
            panic!()
        };
        let aggregated_weights = if let Model::F32(weights) = aggregated_model {
            weights
        } else {
            panic!()
        };
        let averaged_weights = weights
            .iter()
            .zip(other_weights.iter())
            .map(|(weight, other_weight)| 0.5 * weight + 0.5 * other_weight)
            .collect::<Vec<f32>>();
        assert!(aggregated_weights.iter().zip(averaged_weights.iter()).all(
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

    #[test]
    fn test_ratio_as_float() {
        // f32
        let ratio = vec![Ratio::from_float(0_f32).unwrap()];
        assert_eq!(MaskedModel::ratios_as_floats::<f32>(ratio), vec![0_f32]);
        let ratio = vec![Ratio::from_float(0.1_f32).unwrap()];
        assert_eq!(MaskedModel::ratios_as_floats::<f32>(ratio), vec![0.1_f32]);
        let ratio = vec![
            (Ratio::from_float(f32::max_value()).unwrap() * BigInt::from(10_usize))
                / (Ratio::from_float(f32::max_value()).unwrap() * BigInt::from(100_usize)),
        ];
        assert_eq!(MaskedModel::ratios_as_floats::<f32>(ratio), vec![0.1_f32]);

        // f64
        let ratio = vec![Ratio::from_float(0_f64).unwrap()];
        assert_eq!(MaskedModel::ratios_as_floats::<f64>(ratio), vec![0_f64]);
        let ratio = vec![Ratio::from_float(0.1_f64).unwrap()];
        assert_eq!(MaskedModel::ratios_as_floats::<f64>(ratio), vec![0.1_f64]);
        let ratio = vec![
            (Ratio::from_float(f64::max_value()).unwrap() * BigInt::from(10_usize))
                / (Ratio::from_float(f64::max_value()).unwrap() * BigInt::from(100_usize)),
        ];
        assert_eq!(MaskedModel::ratios_as_floats::<f64>(ratio), vec![0.1_f64]);
    }
}
