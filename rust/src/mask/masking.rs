use rand::SeedableRng;
use std::iter::{self, Iterator};

use num::{
    bigint::{BigInt, BigUint, ToBigInt},
    clamp,
    rational::Ratio,
};
use rand_chacha::ChaCha20Rng;

use crate::{
    crypto::generate_integer,
    mask::{MaskConfig, MaskObject, MaskSeed, Model},
};

use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum UnmaskingError {
    #[error("there is no model to unmask")]
    NoModel,

    #[error("too many models were aggregated for the current unmasking configuration")]
    TooManyModels,

    #[error("the masked model is incompatible with the mask used for unmasking")]
    MaskMismatch,

    #[error("the mask is invalid")]
    InvalidMask,
}

#[derive(Debug, Error)]
pub enum AggregationError {
    #[error("the model to aggregate is invalid")]
    InvalidModel,

    #[error("too many models were aggregated for the current unmasking configuration")]
    TooManyModels,

    #[error("the model to aggregate is incompatible with the current aggregated model")]
    ModelMismatch,
}

#[derive(Debug)]
pub struct Aggregation {
    nb_models: usize,
    object: MaskObject,
}

impl From<MaskObject> for Aggregation {
    fn from(object: MaskObject) -> Self {
        Self {
            nb_models: 1,
            object,
        }
    }
}

impl Into<MaskObject> for Aggregation {
    fn into(self) -> MaskObject {
        self.object
    }
}

impl Aggregation {
    pub fn new(config: MaskConfig) -> Self {
        Self {
            nb_models: 0,
            object: MaskObject::new(config, vec![]),
        }
    }

    pub fn config(&self) -> MaskConfig {
        self.object.config
    }

    pub fn validate_unmasking(&self, mask: &MaskObject) -> Result<(), UnmaskingError> {
        // We cannot perform unmasking without at least one real model
        if self.nb_models == 0 {
            return Err(UnmaskingError::NoModel);
        }

        if self.nb_models > self.object.config.model_type.nb_models_max() {
            return Err(UnmaskingError::TooManyModels);
        }

        if self.object.config != mask.config || self.object.data.len() != mask.data.len() {
            return Err(UnmaskingError::MaskMismatch);
        }

        if !mask.is_valid() {
            return Err(UnmaskingError::InvalidMask);
        }

        Ok(())
    }

    pub fn unmask(mut self, mask: MaskObject) -> Model {
        let scaled_add_shift = self.object.config.add_shift() * BigInt::from(self.nb_models);
        let exp_shift = self.object.config.exp_shift();
        let order = self.object.config.order();
        self.object
            .data
            .drain(..)
            .zip(mask.data.into_iter())
            .map(|(masked_weight, mask)| {
                // PANIC_SAFE: The substraction panics if it
                // underflows, which can only happen if:
                //
                //     mask > self.object.config.order()
                //
                // If the mask is valid, we are guaranteed that this
                // cannot happen. Thus this method may panic only if
                // given an invalid mask.
                let n = (masked_weight + &order - mask) % &order;

                // UNWRAP_SAFE: to_bigint never fails for BigUint
                let ratio = Ratio::<BigInt>::from(n.to_bigint().unwrap());

                ratio / &exp_shift - &scaled_add_shift
            })
            .collect()
    }

    pub fn validate_aggregation(&self, object: &MaskObject) -> Result<(), AggregationError> {
        if self.object.config != object.config {
            return Err(AggregationError::ModelMismatch);
        }

        // If we have at least one object, make sure the object we're
        // trying to aggregate has the same length.
        if self.nb_models > 0 && (self.object.data.len() != object.data.len()) {
            return Err(AggregationError::ModelMismatch);
        }

        if self.nb_models == self.object.config.model_type.nb_models_max() {
            return Err(AggregationError::TooManyModels);
        }

        if !object.is_valid() {
            return Err(AggregationError::InvalidModel);
        }

        Ok(())
    }

    pub fn aggregate(&mut self, object: MaskObject) {
        if self.nb_models == 0 {
            self.object = object;
            self.nb_models = 1;
            return;
        }

        let order = self.object.config.order();
        for (i, j) in self.object.data.iter_mut().zip(object.data.into_iter()) {
            *i = (&*i + j) % &order
        }
        self.nb_models += 1;
    }
}

pub struct Masker {
    pub config: MaskConfig,
    pub seed: MaskSeed,
}

impl Masker {
    pub fn new(config: MaskConfig) -> Self {
        Self {
            config,
            seed: MaskSeed::generate(),
        }
    }

    pub fn with_seed(config: MaskConfig, seed: MaskSeed) -> Self {
        Self { config, seed }
    }
}

impl Masker {
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
    /// a more details see [the confluence page](https://xainag.atlassian.net/wiki/spaces/FP/pages/542408769/Masking).
    pub fn mask(self, scalar: f64, model: Model) -> (MaskSeed, MaskObject) {
        let random_ints = self.random_ints();

        let Self { seed, config } = self;

        let exp_shift = config.exp_shift();
        let add_shift = config.add_shift();
        let order = config.order();
        let higher_bound = &add_shift;
        let lower_bound = -&add_shift;
        let scalar = Ratio::<BigInt>::from_float(clamp(scalar, 0_f64, 1_f64)).unwrap();
        let masked_weights = model
            .into_iter()
            .zip(random_ints)
            .map(|(weight, rand_int)| {
                let scaled = &scalar * clamp(&weight, &lower_bound, higher_bound);
                // PANIC_SAFE: shifted weight is guaranteed to be non-negative
                let shifted = ((scaled + &add_shift) * &exp_shift)
                    .to_integer()
                    .to_biguint()
                    .unwrap();
                (shifted + rand_int) % &order
            })
            .collect();
        let masked_model = MaskObject::new(config, masked_weights);
        (seed, masked_model)
    }

    fn random_ints(&self) -> impl Iterator<Item = BigUint> {
        let order = self.config.order();
        let mut prng = ChaCha20Rng::from_seed(self.seed.as_array());

        iter::from_fn(move || Some(generate_integer(&mut prng, &order)))
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, iter};

    use num::traits::Signed;
    use rand::{
        distributions::{Distribution, Uniform},
        SeedableRng,
    };
    use rand_chacha::ChaCha20Rng;

    use super::*;
    use crate::mask::{
        config::{BoundType, DataType, GroupType, MaskConfig, ModelType},
        model::FromPrimitives,
    };

    fn config() -> MaskConfig {
        MaskConfig {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        }
    }

    /// Return an iterator that yield models
    fn models() -> impl Iterator<Item = Model> {
        fn model(prng: &mut ChaCha20Rng) -> Model {
            let uniform = Uniform::new(-1_f32, 1_f32);
            Model::from_primitives(iter::repeat_with(|| uniform.sample(prng)).take(10)).unwrap()
        }

        let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
        iter::repeat_with(move || model(&mut prng))
    }

    /// Return an iterator that mask objects
    fn mask_objects() -> impl Iterator<Item = MaskObject> {
        fn mask_object(prng: &mut ChaCha20Rng) -> MaskObject {
            let prng = RefCell::new(prng);
            let integers: Vec<_> =
                iter::repeat_with(|| generate_integer(&mut prng.borrow_mut(), &config().order()))
                    .take(10)
                    .collect();
            MaskObject::new(config(), integers)
        }
        let prng = RefCell::new(ChaCha20Rng::from_seed([0_u8; 32]));
        iter::repeat_with(move || mask_object(&mut prng.borrow_mut()))
    }

    #[test]
    fn test_aggregation() {
        let mut masked_models = mask_objects();
        let mut aggregated_model = Aggregation::from(masked_models.next().unwrap());
        let masked_model = masked_models.next().unwrap();
        let model_len = masked_model.data.len();
        assert!(aggregated_model.validate_aggregation(&masked_model).is_ok());
        aggregated_model.aggregate(masked_model);

        assert_eq!(aggregated_model.nb_models, 2);
        assert_eq!(aggregated_model.object.data.len(), model_len);
        assert_eq!(aggregated_model.object.config, config());
        assert!(aggregated_model
            .object
            .data
            .iter()
            .all(|i| i < &config().order()));
    }

    #[test]
    fn test_masking_and_aggregation() {
        let mut models = models();

        // Generate models
        let model_1 = models.next().unwrap();
        let model_2 = models.next().unwrap();
        // Mask the models
        let (mask_seed_1, masked_model_1) = Masker::new(config()).mask(0.5_f64, model_1.clone());
        let (mask_seed_2, masked_model_2) = Masker::new(config()).mask(0.5_f64, model_2.clone());
        // Derive the mask associated to each masked model
        let mask_1 = mask_seed_1.derive_mask(10, config());
        let mask_2 = mask_seed_2.derive_mask(10, config());

        // Aggregate the masks
        let mut mask_aggregation = Aggregation::from(mask_1);
        mask_aggregation.aggregate(mask_2);
        let aggregated_mask: MaskObject = mask_aggregation.into();

        // Aggregate the models
        let mut model_aggregation = Aggregation::from(masked_model_1);
        model_aggregation.aggregate(masked_model_2);

        // Use the aggregated mask to unmask the final model
        let aggregated_model = model_aggregation.unmask(aggregated_mask);

        // Verifications
        let expected_aggregated_model: Model = model_1
            .iter()
            .zip(model_2.iter())
            .map(|(w1, w2)| (w1 + w2) / BigInt::from(2))
            .collect();

        assert!(aggregated_model
            .iter()
            .zip(expected_aggregated_model.iter())
            .all(|(actual, expected)| {
                (actual - expected).abs() < Ratio::<BigInt>::from_float(1e-8_f32).unwrap()
            }));
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
                    // Step 1: Generate a random model
                    let uniform = paste::expr! {
                        if $bound == 0 {
                            Uniform::new([<$data:lower>]::MIN, [<$data:lower>]::MAX)
                        } else {
                            Uniform::new(-$bound as [<$data:lower>], $bound as [<$data:lower>])
                        }
                    };
                    let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
                    let random_weights = iter::repeat_with(|| uniform.sample(&mut prng)).take($len as usize);
                    let model = Model::from_primitives(random_weights).unwrap();

                    // Step 2: Build the masking config
                    let config = MaskConfig {
                        group_type: $group,
                        data_type: paste::expr! { [<$data:upper>] },
                        bound_type: match $bound {
                            1 => B0,
                            100 => B2,
                            10_000 => B4,
                            1_000_000 => B6,
                            0 => Bmax,
                            _ => panic!("Unknown bound!")
                        },
                        model_type: M3,
                    };

                    // Step 3 (actual test):
                    // a. mask the model
                    // b. derive the mask corresponding to the seed used
                    // c. unmask the model and check it against the original one.

                    let (mask_seed, masked_model) = Masker::new(config.clone()).mask(1_f64, model.clone());
                    assert_eq!(masked_model.data.len(), model.len());
                    assert!(masked_model.is_valid());

                    let mask = mask_seed.derive_mask(model.len(), config);
                    let aggregation = Aggregation::from(masked_model);
                    let unmasked_model = aggregation.unmask(mask);

                    let tolerance = Ratio::<BigInt>::from_float($tol).unwrap();
                    assert!(
                        model.iter()
                            .zip(unmasked_model.iter())
                            .all(|(weight, unmasked_weight)| {
                                (weight - unmasked_weight).abs() < tolerance
                            })
                    );
                }
            }
        };
        ($suffix:ident, $group:ty, $data:ty, $len:expr, $tol:expr $(,)?) => {
            test_masking!($suffix, $group, $data, 0, $len, $tol);
        };
    }

    use crate::mask::config::{BoundType::*, DataType::*, GroupType::*, ModelType::*};

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
