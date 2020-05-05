pub mod config;
pub mod seed;

use std::convert::TryInto;

use num::{
    bigint::{BigInt, BigUint, ToBigInt},
    rational::Ratio,
    traits::{cast::ToPrimitive, float::FloatCore},
};

use self::config::MaskConfig;
use crate::{model::Model, PetError};

#[allow(clippy::len_without_is_empty)]
/// Aggregation and serialization of vectors of arbitrarily large integers.
pub trait Integers: Sized {
    type Error;

    define_trait_fields!(
        integers, Vec<BigUint>;
        config, MaskConfig;
    );

    /// Get an error value of the error type to be used in the default implementations.
    fn error_value() -> Self::Error;

    /// Create the object from its parts. Fails if the integers don't conform to the mask
    /// configuration.
    fn from_parts(integers: Vec<BigUint>, config: MaskConfig) -> Result<Self, Self::Error>;

    /// Get the length of the serialized object.
    fn len(&self) -> usize {
        4 + self.integers().len() * self.config().element_len()
    }

    /// Serialize the object into bytes.
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

    /// Deserialize the object from bytes. Fails if the bytes don't conform to the mask
    /// configuration.
    fn deserialize(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 4 {
            return Err(Self::error_value());
        }
        let config = MaskConfig::deserialize(&bytes[..4]).or_else(|_| Err(Self::error_value()))?;
        let element_len = config.element_len();
        if bytes[4..].len() % element_len != 0 {
            return Err(Self::error_value());
        }
        let integers = bytes[4..]
            .chunks_exact(element_len)
            .map(|chunk| BigUint::from_bytes_le(chunk))
            .collect::<Vec<BigUint>>();
        Self::from_parts(integers, config)
    }

    /// Aggregate the object with another one. Fails if the mask configurations or the integer
    /// lengths don't conform.
    fn aggregate(&self, other: &Self) -> Result<Self, Self::Error> {
        if self.integers().len() == other.integers().len() && self.config() == other.config() {
            let aggregated_integers = self
                .integers()
                .iter()
                .zip(other.integers().iter())
                .map(|(integer, other_integer)| (integer + other_integer) % self.config().order())
                .collect();
            Self::from_parts(aggregated_integers, self.config().clone())
        } else {
            Err(Self::error_value())
        }
    }
}

/// Unmasking of vectors of arbitrarily large integers.
pub trait MaskIntegers<N>: Integers {
    /// Unmask the masked model with a mask. Fails if the mask configurations or the integer lengths
    /// don't conform or the number of models is zero.
    fn unmask(&self, mask: &Mask, no_models: usize) -> Result<Model<N>, Self::Error>;

    /// Cast the ratios as numbers.
    fn numbers_from(ratios: Vec<Ratio<BigInt>>) -> Option<Vec<N>>;

    /// Unmask the masked numbers with a mask. Fails if the mask configurations or the integer
    /// lengths don't conform or the number of models is zero.
    fn unmask_numbers(&self, mask: &Mask, no_models: usize) -> Result<Vec<N>, Self::Error> {
        if no_models > 0
            && self.integers().len() == mask.integers().len()
            && self.config() == mask.config()
        {
            let scaled_add_shift = self.config().add_shift() * BigInt::from(no_models);
            let ratios = self
                .integers()
                .iter()
                .zip(mask.integers().iter())
                .map(|(masked_weight, mask)| {
                    let unmasked = Ratio::<BigInt>::from(
                        ((masked_weight + self.config().order() - mask) % self.config().order())
                            .to_bigint()
                            // safe unwrap: `to_bigint` never fails for `BigUint`s
                            .unwrap(),
                    );
                    unmasked / self.config().exp_shift() - &scaled_add_shift
                })
                .collect::<Vec<Ratio<BigInt>>>();
            Self::numbers_from(ratios).ok_or_else(Self::error_value)
        } else {
            Err(Self::error_value())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A masked model. Its parameters are represented as a vector of integers from a finite group wrt
/// a mask configuration.
pub struct MaskedModel {
    integers: Vec<BigUint>,
    config: MaskConfig,
}

impl Integers for MaskedModel {
    type Error = PetError;

    derive_trait_fields!(
        integers, Vec<BigUint>;
        config, MaskConfig;
    );

    /// Get an error value of the error type to be used in the default implementations.
    fn error_value() -> Self::Error {
        Self::Error::InvalidModel
    }

    /// Create a masked model from its parts. Fails if the integers don't conform to the mask
    /// configuration.
    fn from_parts(integers: Vec<BigUint>, config: MaskConfig) -> Result<Self, Self::Error> {
        if integers.iter().all(|integer| integer < config.order()) {
            Ok(Self { integers, config })
        } else {
            Err(Self::Error::InvalidModel)
        }
    }
}

impl MaskIntegers<f32> for MaskedModel {
    /// Unmask the masked model with a mask. Fails if the mask configurations or the integer lengths
    /// don't conform or the number of models is zero.
    fn unmask(&self, mask: &Mask, no_models: usize) -> Result<Model<f32>, PetError> {
        <Self as MaskIntegers<f32>>::unmask_numbers(&self, mask, no_models)?.try_into()
    }

    /// Cast the ratios as numbers.
    fn numbers_from(ratios: Vec<Ratio<BigInt>>) -> Option<Vec<f32>> {
        Some(floats_from(ratios))
    }
}

impl MaskIntegers<f64> for MaskedModel {
    /// Unmask the masked model with a mask. Fails if the mask configurations or the integer lengths
    /// don't conform or the number of models is zero.
    fn unmask(&self, mask: &Mask, no_models: usize) -> Result<Model<f64>, PetError> {
        <Self as MaskIntegers<f64>>::unmask_numbers(&self, mask, no_models)?.try_into()
    }

    /// Cast the ratios as numbers.
    fn numbers_from(ratios: Vec<Ratio<BigInt>>) -> Option<Vec<f64>> {
        Some(floats_from(ratios))
    }
}

impl MaskIntegers<i32> for MaskedModel {
    /// Unmask the masked model with a mask. Fails if the mask configurations or the integer lengths
    /// don't conform or the number of models is zero.
    fn unmask(&self, mask: &Mask, no_models: usize) -> Result<Model<i32>, PetError> {
        Ok(<Self as MaskIntegers<i32>>::unmask_numbers(&self, mask, no_models)?.into())
    }

    /// Cast the ratios as numbers.
    fn numbers_from(ratios: Vec<Ratio<BigInt>>) -> Option<Vec<i32>> {
        ratios
            .iter()
            .map(|ratio| (ratio.to_integer().to_i32()))
            .collect()
    }
}

impl MaskIntegers<i64> for MaskedModel {
    /// Unmask the masked model with a mask. Fails if the mask configurations or the integer lengths
    /// don't conform or the number of models is zero.
    fn unmask(&self, mask: &Mask, no_models: usize) -> Result<Model<i64>, PetError> {
        Ok(<Self as MaskIntegers<i64>>::unmask_numbers(&self, mask, no_models)?.into())
    }

    /// Cast the ratios as numbers.
    fn numbers_from(ratios: Vec<Ratio<BigInt>>) -> Option<Vec<i64>> {
        ratios
            .iter()
            .map(|ratio| (ratio.to_integer().to_i64()))
            .collect()
    }
}

/// Cast the ratios as floats.
fn floats_from<F: FloatCore>(ratios: Vec<Ratio<BigInt>>) -> Vec<F> {
    ratios
        .iter()
        .map(|ratio| {
            let mut numer = ratio.numer().clone();
            let mut denom = ratio.denom().clone();
            // safe loop: terminates after at most bit-length of ratio iterations
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// A mask. Its parameters are represented as a vector of integers from a finite group wrt a mask
/// configuration.
pub struct Mask {
    integers: Vec<BigUint>,
    config: MaskConfig,
}

impl Integers for Mask {
    type Error = PetError;

    derive_trait_fields!(
        integers, Vec<BigUint>;
        config, MaskConfig;
    );

    /// Get an error value of the error type to be used in the default implementations.
    fn error_value() -> Self::Error {
        Self::Error::InvalidMask
    }

    /// Create a mask from its parts. Fails if the integers don't conform to the mask configuration.
    fn from_parts(integers: Vec<BigUint>, config: MaskConfig) -> Result<Self, Self::Error> {
        if integers.iter().all(|integer| integer < config.order()) {
            Ok(Self { integers, config })
        } else {
            Err(Self::Error::InvalidMask)
        }
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
        crypto::generate_integer,
        mask::config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
        model::MaskModels,
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
        let unmasked_model: Model<f32> = masked_model.unmask(&mask, 1).unwrap();
        assert!(model
            .weights()
            .iter()
            .zip(unmasked_model.weights().iter())
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
        let (mask_seed, masked_model) = model.mask(0.5_f64, &config);
        let (other_mask_seed, other_masked_model) = other_model.mask(0.5_f64, &config);
        let aggregated_masked_model = masked_model.aggregate(&other_masked_model).unwrap();
        let aggregated_mask = mask_seed
            .derive_mask(10, &config)
            .aggregate(&other_mask_seed.derive_mask(10, &config))
            .unwrap();
        let aggregated_model: Model<f32> =
            aggregated_masked_model.unmask(&aggregated_mask, 2).unwrap();
        let averaged_weights = model
            .weights()
            .iter()
            .zip(other_model.weights().iter())
            .map(|(weight, other_weight)| 0.5 * weight + 0.5 * other_weight)
            .collect::<Vec<f32>>();
        assert!(aggregated_model
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

    #[test]
    fn test_floats_from() {
        // f32
        let ratio = vec![Ratio::from_float(0_f32).unwrap()];
        assert_eq!(floats_from::<f32>(ratio), vec![0_f32]);
        let ratio = vec![Ratio::from_float(0.1_f32).unwrap()];
        assert_eq!(floats_from::<f32>(ratio), vec![0.1_f32]);
        let ratio = vec![
            (Ratio::from_float(f32::max_value()).unwrap() * BigInt::from(10_usize))
                / (Ratio::from_float(f32::max_value()).unwrap() * BigInt::from(100_usize)),
        ];
        assert_eq!(floats_from::<f32>(ratio), vec![0.1_f32]);

        // f64
        let ratio = vec![Ratio::from_float(0_f64).unwrap()];
        assert_eq!(floats_from::<f64>(ratio), vec![0_f64]);
        let ratio = vec![Ratio::from_float(0.1_f64).unwrap()];
        assert_eq!(floats_from::<f64>(ratio), vec![0.1_f64]);
        let ratio = vec![
            (Ratio::from_float(f64::max_value()).unwrap() * BigInt::from(10_usize))
                / (Ratio::from_float(f64::max_value()).unwrap() * BigInt::from(100_usize)),
        ];
        assert_eq!(floats_from::<f64>(ratio), vec![0.1_f64]);
    }
}
