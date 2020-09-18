//! Masking, aggregation and unmasking of models.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: ../index.html

use std::iter::{self, Iterator};

use num::{
    bigint::{BigInt, BigUint, ToBigInt},
    clamp,
    rational::Ratio,
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use thiserror::Error;

use crate::{
    crypto::{prng::generate_integer, ByteObject},
    mask::{
        config::MaskConfig, model::float_to_ratio_bounded, model::Model, object::MaskMany,
        object::MaskObject, object::MaskOne, seed::MaskSeed,
    },
};

#[derive(Debug, Error, Eq, PartialEq)]
/// Errors related to the unmasking of models.
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
/// Errors related to the aggregation of masks and models.
pub enum AggregationError {
    #[error("the model to aggregate is invalid")]
    InvalidModel,

    #[error("too many models were aggregated for the current unmasking configuration")]
    TooManyModels,

    #[error("the model to aggregate is incompatible with the current aggregated model")]
    ModelMismatch,
}

#[derive(Debug, Clone)]
/// An aggregator for masks and masked models.
pub struct Aggregation {
    nb_models: usize,
    object: MaskObject,
    object_size: usize,
}

impl From<MaskObject> for Aggregation {
    fn from(object: MaskObject) -> Self {
        Self {
            nb_models: 1,
            object_size: object.vector.data.len(),
            object,
        }
    }
}

impl Into<MaskObject> for Aggregation {
    fn into(self) -> MaskObject {
        self.object
    }
}

#[allow(clippy::len_without_is_empty)]
impl Aggregation {
    /// Creates a new, empty aggregator for masks or masked models.
    pub fn new(config_many: MaskConfig, config_one: MaskConfig, object_size: usize) -> Self {
        Self {
            nb_models: 0,
            object: MaskObject::empty(config_many, config_one, object_size),
            object_size,
        }
    }

    /// Gets the length of the aggregated mask object.
    pub fn len(&self) -> usize {
        self.object_size
    }

    /// Gets the masking configuration of the aggregator.
    pub fn config(&self) -> MaskConfig {
        // TODO rename to config_many or sth
        self.object.vector.config
    }

    // TODO config_one

    /// Validates if unmasking of the aggregated masked model with the given `mask` may be
    /// safely performed.
    ///
    /// This should be checked before calling [`unmask()`], since unmasking may return garbage
    /// values otherwise.
    ///
    /// # Errors
    /// Fails in one of the following cases:
    /// - The aggregator has not yet aggregated any models.
    /// - The number of aggregated masked models is larger than the chosen masking configuration
    ///   allows.
    /// - The masking configuration of the aggregator and of the `mask` don't coincide.
    /// - The length of the aggregated masked model and the `mask` don't coincide.
    /// - The `mask` itself is invalid.
    ///
    /// Even though it does not produce any meaningful values, it is safe and technically possible
    /// due to the [`MaskObject`] type to validate, that:
    /// - a mask may unmask another mask
    /// - a masked model may unmask a mask
    /// - a masked model may unmask another masked model
    ///
    /// [`unmask()`]: #method.unmask
    pub fn validate_unmasking(&self, mask: &MaskMany) -> Result<(), UnmaskingError> {
        // TODO later: mask is MaskObject
        // We cannot perform unmasking without at least one real model
        if self.nb_models == 0 {
            return Err(UnmaskingError::NoModel);
        }

        if self.nb_models > self.object.vector.config.model_type.max_nb_models() {
            return Err(UnmaskingError::TooManyModels);
        }
        // TODO analogous check for scalar - could fail independently!

        if self.object.vector.config != mask.config || self.object_size != mask.data.len() {
            return Err(UnmaskingError::MaskMismatch);
        }
        // TODO similar config check for scalar

        if !mask.is_valid() {
            return Err(UnmaskingError::InvalidMask);
        }

        Ok(())
    }

    /// Unmasks the aggregated masked model with the given `mask`.
    ///
    /// It should be checked that [`validate_unmasking()`] succeeds before calling this, since
    /// unmasking may return garbage values otherwise. The unmasking is performed in opposite order
    /// as described for [`mask()`].
    ///
    /// # Panics
    /// This may only panic if [`validate_unmasking()`] fails.
    ///
    /// Even though it does not produce any meaningful values, it is safe and technically possible
    /// due to the [`MaskObject`] type to unmask:
    /// - a mask with another mask
    /// - a mask with a masked model
    /// - a masked model with another masked model
    ///
    /// if [`validate_unmasking()`] returns `true`.
    ///
    /// [`validate_unmasking()`]: #method.validate_unmasking
    /// [`mask()`]: struct.Masker.html#method.mask
    pub fn unmask(mut self, mask: MaskMany) -> Model {
        let scaled_add_shift = self.object.vector.config.add_shift() * BigInt::from(self.nb_models);
        let exp_shift = self.object.vector.config.exp_shift();
        let order = self.object.vector.config.order();
        self.object
            .vector
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

    /// Applies a correction to the given unmasked model based on the associated
    /// unmasked scalar sum, in order to scale it correctly.
    ///
    /// This should be called after [`unmask()`] is called for both the model
    /// and scalar aggregations.
    ///
    /// [`unmask()`]: struct.Aggregation.html#method.unmask
    pub fn correct(overscaled: Model, scalar_sum: Model) -> Model {
        // FIXME later on, tidy up API so that scalar_sum is encapsulated away
        let correction = scalar_sum.into_iter().next().unwrap();
        overscaled
            .into_iter()
            .map(|weight| weight / &correction)
            .collect()
    }

    /// Validates if aggregation of the aggregated mask object with the given `object` may be safely
    /// performed.
    ///
    /// This should be checked before calling [`aggregate()`], since aggregation may return garbage
    /// values otherwise.
    ///
    /// # Errors
    /// Fails in one of the following cases:
    /// - The masking configuration of the aggregator and of the `object` don't coincide.
    /// - The length of the aggregated masks or masked model and the `object` don't coincide. If the
    ///   aggregator is empty, then an `object` of any length may be aggregated.
    /// - The new number of aggregated masks or masked models would exceed the number that the
    ///   chosen masking configuration allows.
    /// - The `object` itself is invalid.
    ///
    /// Even though it does not produce any meaningful values, it is safe and technically possible
    /// due to the [`MaskObject`] type to validate, that a mask may be aggregated with a masked
    /// model.
    ///
    /// [`aggregate()`]: #method.aggregate
    pub fn validate_aggregation(&self, object: &MaskMany) -> Result<(), AggregationError> {
        // TODO object should be MaskObject; adjust checks below
        if self.object.vector.config != object.config {
            return Err(AggregationError::ModelMismatch);
        }

        if self.object_size != object.data.len() {
            return Err(AggregationError::ModelMismatch);
        }

        if self.nb_models >= self.object.vector.config.model_type.max_nb_models() {
            return Err(AggregationError::TooManyModels);
        }

        if !object.is_valid() {
            return Err(AggregationError::InvalidModel);
        }

        Ok(())
    }

    /// Aggregates the aggregated mask object with the given `object`.
    ///
    /// It should be checked that [`validate_aggregation()`] succeeds before calling this, since
    /// aggregation may return garbage values otherwise.
    ///
    /// # Errors
    /// Even though it does not produce any meaningful values, it is safe and technically possible
    /// due to the [`MaskObject`] type to aggregate a mask with a masked model if
    /// [`validate_aggregation()`] returns `true`.
    ///
    /// [`validate_aggregation()`]: #method.validate_aggregation
    pub fn aggregate(&mut self, object: MaskMany) {
        if self.nb_models == 0 {
            self.object.vector = object;
            self.nb_models = 1;
            return;
        }

        let order = self.object.vector.config.order();
        for (i, j) in self
            .object
            .vector
            .data
            .iter_mut()
            .zip(object.data.into_iter())
        {
            *i = (&*i + j) % &order
        }
        self.nb_models += 1;
    }
}

/// A masker for models.
pub struct Masker {
    config_model: MaskConfig,
    config_scalar: MaskConfig,
    seed: MaskSeed,
}

impl Masker {
    /// Creates a new masker with the given masking `config`uration with a randomly generated seed.
    pub fn new(config_model: MaskConfig, config_scalar: MaskConfig) -> Self {
        Self {
            config_model,
            config_scalar,
            seed: MaskSeed::generate(),
        }
    }

    /// Creates a new masker with the given masking `config`uration and `seed`.
    pub fn with_seed(config_model: MaskConfig, config_scalar: MaskConfig, seed: MaskSeed) -> Self {
        Self {
            config_model,
            config_scalar,
            seed,
        }
    }
}

impl Masker {
    /// Masks the given `model` wrt the masking configuration. Enforces bounds on the scalar and
    /// weights.
    ///
    /// The masking proceeds in the following steps:
    /// - Clamp the scalar and the weights according to the masking configuration.
    /// - Scale the weights by the scalar.
    /// - Shift the weights into the non-negative reals.
    /// - Shift the weights into the non-negative integers.
    /// - Shift the weights into the finite group.
    /// - Mask the weights with random elements from the finite group.
    ///
    /// The `scalar` is also masked, following a similar process.
    ///
    /// The random elements are derived from a seeded PRNG. Unmasking as performed in [`unmask()`]
    /// proceeds in reverse order.
    ///
    /// [`unmask()`]: struct.Aggregation.html#method.unmask
    pub fn mask(self, scalar: f64, model: Model) -> (MaskSeed, MaskObject) {
        let mut random_ints = self.random_ints();
        let random_int = self.random_int();
        let Self {
            config_model,
            config_scalar,
            seed,
        } = self;

        // clamp the scalar
        let add_shift_scalar = config_scalar.add_shift();
        let scalar_ratio = float_to_ratio_bounded(scalar);
        let zero = Ratio::<BigInt>::from_float(0_f64).unwrap();
        let scalar_clamped = clamp(&scalar_ratio, &zero, &add_shift_scalar);

        let exp_shift = config_model.exp_shift();
        let add_shift = config_model.add_shift();
        let order = config_model.order();
        let higher_bound = &add_shift;
        let lower_bound = -&add_shift;

        // mask the (scaled) weights
        let masked_weights = model
            .into_iter()
            .zip(&mut random_ints)
            .map(|(weight, rand_int)| {
                let scaled = scalar_clamped * &weight;
                let scaled_clamped = clamp(&scaled, &lower_bound, higher_bound);
                // PANIC_SAFE: shifted weight is guaranteed to be non-negative
                let shifted = ((scaled_clamped + &add_shift) * &exp_shift)
                    .to_integer()
                    .to_biguint()
                    .unwrap();
                (shifted + rand_int) % &order
            })
            .collect();
        let masked_model = MaskMany::new(config_model, masked_weights);

        // mask the scalar
        // PANIC_SAFE: shifted scalar is guaranteed to be non-negative
        let shifted = ((scalar_clamped + &add_shift_scalar) * config_scalar.exp_shift())
            .to_integer()
            .to_biguint()
            .unwrap();
        let masked = (shifted + random_int) % config_scalar.order();
        let masked_scalar = MaskOne::new(config_scalar, masked);

        (seed, MaskObject::new(masked_model, masked_scalar))
    }

    /// Creates an iterator that yields randomly generated integers wrt the
    /// model masking configuration.
    fn random_ints(&self) -> impl Iterator<Item = BigUint> {
        let order = self.config_model.order();
        let mut prng = ChaCha20Rng::from_seed(self.seed.as_array());
        iter::from_fn(move || Some(generate_integer(&mut prng, &order)))
    }

    /// Generates a random integer wrt the scalar masking configuration.
    fn random_int(&self) -> BigUint {
        let order = self.config_scalar.order();
        let mut prng = ChaCha20Rng::from_seed(self.seed.as_array());
        generate_integer(&mut prng, &order)
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use num::traits::Signed;
    use rand::{
        distributions::{Distribution, Uniform},
        SeedableRng,
    };
    use rand_chacha::ChaCha20Rng;

    use super::*;
    use crate::mask::{
        config::{
            BoundType::{Bmax, B0, B2, B4, B6},
            DataType::{F32, F64, I32, I64},
            GroupType::{Integer, Power2, Prime},
            MaskConfig,
            ModelType::M3,
        },
        model::FromPrimitives,
    };

    /// Generate tests for masking and unmasking of a single model:
    /// - generate random weights from a uniform distribution with a seeded PRNG
    /// - create a model from the weights and mask it
    /// - check that all masked weights belong to the chosen finite group
    /// - unmask the masked model
    /// - check that all unmasked weights are equal to the original weights (up to a tolerance
    ///   determined by the masking configuration)
    ///
    /// The arguments to the macro are:
    /// - a suffix for the test name
    /// - the group type of the model (variants of `GroupType`)
    /// - the data type of the model (either primitives or variants of `DataType`)
    /// - an absolute bound for the weights (optional, choices: 1, 100, 10_000, 1_000_000)
    /// - the number of weights
    macro_rules! test_masking {
        ($suffix:ident, $group:ty, $data:ty, $bound:expr, $len:expr $(,)?) => {
            paste::item! {
                #[test]
                fn [<test_masking_ $suffix>]() {
                    // Step 1: Build the masking config
                    let config = MaskConfig {
                        group_type: $group,
                        data_type: paste::expr! { [<$data:upper>] },
                        bound_type: match $bound {
                            1 => B0,
                            100 => B2,
                            10_000 => B4,
                            1_000_000 => B6,
                            _ => Bmax,
                        },
                        model_type: M3,
                    };

                    // Step 2: Generate a random model
                    let bound = if $bound == 0 {
                        paste::expr! { [<$data:lower>]::MAX / (2 as [<$data:lower>]) }
                    } else {
                        paste::expr! { $bound as [<$data:lower>] }
                    };
                    let mut prng = ChaCha20Rng::from_seed(MaskSeed::generate().as_array());
                    let random_weights = Uniform::new_inclusive(-bound, bound)
                        .sample_iter(&mut prng)
                        .take($len as usize);
                    let model = Model::from_primitives(random_weights).unwrap();

                    // Step 3 (actual test):
                    // a. mask the model
                    // b. derive the mask corresponding to the seed used
                    // c. unmask the model and check it against the original one.
                    let (mask_seed, masked_model, masked_scalar) =
                        Masker::new(config.clone()).mask(1_f64, model.clone());
                    assert_eq!(masked_model.data.len(), model.len());
                    assert!(masked_model.is_valid());
                    assert_eq!(masked_scalar.data.len(), 1);
                    assert!(masked_scalar.is_valid());

                    let (mask, _scalar_mask) = mask_seed.derive_mask(model.len(), config);
                    let aggregation = Aggregation::from(masked_model);
                    let unmasked_model = aggregation.unmask(mask);

                    let tolerance = Ratio::from_integer(config.exp_shift()).recip();
                    assert!(
                        model.iter()
                            .zip(unmasked_model.iter())
                            .all(|(weight, unmasked_weight)| {
                                (weight - unmasked_weight).abs() <= tolerance
                            })
                    );
                }
            }
        };
        ($suffix:ident, $group:ty, $data:ty, $len:expr $(,)?) => {
            test_masking!($suffix, $group, $data, 0, $len);
        };
    }

    test_masking!(int_f32_b0, Integer, f32, 1, 10);
    test_masking!(int_f32_b2, Integer, f32, 100, 10);
    test_masking!(int_f32_b4, Integer, f32, 10_000, 10);
    test_masking!(int_f32_b6, Integer, f32, 1_000_000, 10);
    test_masking!(int_f32_bmax, Integer, f32, 10);

    test_masking!(prime_f32_b0, Prime, f32, 1, 10);
    test_masking!(prime_f32_b2, Prime, f32, 100, 10);
    test_masking!(prime_f32_b4, Prime, f32, 10_000, 10);
    test_masking!(prime_f32_b6, Prime, f32, 1_000_000, 10);
    test_masking!(prime_f32_bmax, Prime, f32, 10);

    test_masking!(pow_f32_b0, Power2, f32, 1, 10);
    test_masking!(pow_f32_b2, Power2, f32, 100, 10);
    test_masking!(pow_f32_b4, Power2, f32, 10_000, 10);
    test_masking!(pow_f32_b6, Power2, f32, 1_000_000, 10);
    test_masking!(pow_f32_bmax, Power2, f32, 10);

    test_masking!(int_f64_b0, Integer, f64, 1, 10);
    test_masking!(int_f64_b2, Integer, f64, 100, 10);
    test_masking!(int_f64_b4, Integer, f64, 10_000, 10);
    test_masking!(int_f64_b6, Integer, f64, 1_000_000, 10);
    test_masking!(int_f64_bmax, Integer, f64, 10);

    test_masking!(prime_f64_b0, Prime, f64, 1, 10);
    test_masking!(prime_f64_b2, Prime, f64, 100, 10);
    test_masking!(prime_f64_b4, Prime, f64, 10_000, 10);
    test_masking!(prime_f64_b6, Prime, f64, 1_000_000, 10);
    test_masking!(prime_f64_bmax, Prime, f64, 10);

    test_masking!(pow_f64_b0, Power2, f64, 1, 10);
    test_masking!(pow_f64_b2, Power2, f64, 100, 10);
    test_masking!(pow_f64_b4, Power2, f64, 10_000, 10);
    test_masking!(pow_f64_b6, Power2, f64, 1_000_000, 10);
    test_masking!(pow_f64_bmax, Power2, f64, 10);

    test_masking!(int_i32_b0, Integer, i32, 1, 10);
    test_masking!(int_i32_b2, Integer, i32, 100, 10);
    test_masking!(int_i32_b4, Integer, i32, 10_000, 10);
    test_masking!(int_i32_b6, Integer, i32, 1_000_000, 10);
    test_masking!(int_i32_bmax, Integer, i32, 10);

    test_masking!(prime_i32_b0, Prime, i32, 1, 10);
    test_masking!(prime_i32_b2, Prime, i32, 100, 10);
    test_masking!(prime_i32_b4, Prime, i32, 10_000, 10);
    test_masking!(prime_i32_b6, Prime, i32, 1_000_000, 10);
    test_masking!(prime_i32_bmax, Prime, i32, 10);

    test_masking!(pow_i32_b0, Power2, i32, 1, 10);
    test_masking!(pow_i32_b2, Power2, i32, 100, 10);
    test_masking!(pow_i32_b4, Power2, i32, 10_000, 10);
    test_masking!(pow_i32_b6, Power2, i32, 1_000_000, 10);
    test_masking!(pow_i32_bmax, Power2, i32, 10);

    test_masking!(int_i64_b0, Integer, i64, 1, 10);
    test_masking!(int_i64_b2, Integer, i64, 100, 10);
    test_masking!(int_i64_b4, Integer, i64, 10_000, 10);
    test_masking!(int_i64_b6, Integer, i64, 1_000_000, 10);
    test_masking!(int_i64_bmax, Integer, i64, 10);

    test_masking!(prime_i64_b0, Prime, i64, 1, 10);
    test_masking!(prime_i64_b2, Prime, i64, 100, 10);
    test_masking!(prime_i64_b4, Prime, i64, 10_000, 10);
    test_masking!(prime_i64_b6, Prime, i64, 1_000_000, 10);
    test_masking!(prime_i64_bmax, Prime, i64, 10);

    test_masking!(pow_i64_b0, Power2, i64, 1, 10);
    test_masking!(pow_i64_b2, Power2, i64, 100, 10);
    test_masking!(pow_i64_b4, Power2, i64, 10_000, 10);
    test_masking!(pow_i64_b6, Power2, i64, 1_000_000, 10);
    test_masking!(pow_i64_bmax, Power2, i64, 10);

    /// Generate tests for aggregation of multiple masked models:
    /// - generate random integers from a uniform distribution with a seeded PRNG
    /// - create a masked model from the integers and aggregate it to the aggregated masked models
    /// - check that all integers belong to the chosen finite group
    ///
    /// The arguments to the macro are:
    /// - a suffix for the test name
    /// - the group type of the model (variants of `GroupType`)
    /// - the data type of the model (variants of `DataType`)
    /// - the bound type of the model (variants of `BoundType`)
    /// - the number of integers per masked model
    /// - the number of masked models
    macro_rules! test_aggregation {
        ($suffix:ident, $group:ty, $data:ty, $bound:expr, $len:expr, $count:expr $(,)?) => {
            paste::item! {
                #[test]
                fn [<test_aggregation_ $suffix>]() {
                    // Step 1: Build the masking config
                    let config = MaskConfig {
                        group_type: $group,
                        data_type: $data,
                        bound_type: $bound,
                        model_type: M3,
                    };
                    let model_size = $len as usize;

                    // Step 2: generate random masked models
                    let mut prng = ChaCha20Rng::from_seed(MaskSeed::generate().as_array());
                    let mut masked_models = iter::repeat_with(move || {
                        let order = config.order();
                        let integers = iter::repeat_with(|| generate_integer(&mut prng, &order))
                            .take($len as usize)
                            .collect::<Vec<_>>();
                        MaskObject::new(config, integers)
                    });

                    // Step 3 (actual test):
                    // a. aggregate the masked models
                    // b. check the aggregated masked model
                    let mut aggregated_masked_model = Aggregation::new(config, model_size);
                    for nb in 1..$count as usize + 1 {
                        let masked_model = masked_models.next().unwrap();
                        assert!(
                            aggregated_masked_model.validate_aggregation(&masked_model).is_ok()
                        );
                        aggregated_masked_model.aggregate(masked_model);

                        assert_eq!(aggregated_masked_model.nb_models, nb);
                        assert_eq!(aggregated_masked_model.object.data.len(), $len as usize);
                        assert_eq!(aggregated_masked_model.object.config, config);
                        assert!(aggregated_masked_model.object.is_valid());
                    }
                }
            }
        };
    }

    test_aggregation!(int_f32_b0, Integer, F32, B0, 10, 5);
    test_aggregation!(int_f32_b2, Integer, F32, B2, 10, 5);
    test_aggregation!(int_f32_b4, Integer, F32, B4, 10, 5);
    test_aggregation!(int_f32_b6, Integer, F32, B6, 10, 5);
    test_aggregation!(int_f32_bmax, Integer, F32, Bmax, 10, 5);

    test_aggregation!(prime_f32_b0, Prime, F32, B0, 10, 5);
    test_aggregation!(prime_f32_b2, Prime, F32, B2, 10, 5);
    test_aggregation!(prime_f32_b4, Prime, F32, B4, 10, 5);
    test_aggregation!(prime_f32_b6, Prime, F32, B6, 10, 5);
    test_aggregation!(prime_f32_bmax, Prime, F32, Bmax, 10, 5);

    test_aggregation!(pow_f32_b0, Power2, F32, B0, 10, 5);
    test_aggregation!(pow_f32_b2, Power2, F32, B2, 10, 5);
    test_aggregation!(pow_f32_b4, Power2, F32, B4, 10, 5);
    test_aggregation!(pow_f32_b6, Power2, F32, B6, 10, 5);
    test_aggregation!(pow_f32_bmax, Power2, F32, Bmax, 10, 5);

    test_aggregation!(int_f64_b0, Integer, F64, B0, 10, 5);
    test_aggregation!(int_f64_b2, Integer, F64, B2, 10, 5);
    test_aggregation!(int_f64_b4, Integer, F64, B4, 10, 5);
    test_aggregation!(int_f64_b6, Integer, F64, B6, 10, 5);
    test_aggregation!(int_f64_bmax, Integer, F64, Bmax, 10, 5);

    test_aggregation!(prime_f64_b0, Prime, F64, B0, 10, 5);
    test_aggregation!(prime_f64_b2, Prime, F64, B2, 10, 5);
    test_aggregation!(prime_f64_b4, Prime, F64, B4, 10, 5);
    test_aggregation!(prime_f64_b6, Prime, F64, B6, 10, 5);
    test_aggregation!(prime_f64_bmax, Prime, F64, Bmax, 10, 5);

    test_aggregation!(pow_f64_b0, Power2, F64, B0, 10, 5);
    test_aggregation!(pow_f64_b2, Power2, F64, B2, 10, 5);
    test_aggregation!(pow_f64_b4, Power2, F64, B4, 10, 5);
    test_aggregation!(pow_f64_b6, Power2, F64, B6, 10, 5);
    test_aggregation!(pow_f64_bmax, Power2, F64, Bmax, 10, 5);

    test_aggregation!(int_i32_b0, Integer, I32, B0, 10, 5);
    test_aggregation!(int_i32_b2, Integer, I32, B2, 10, 5);
    test_aggregation!(int_i32_b4, Integer, I32, B4, 10, 5);
    test_aggregation!(int_i32_b6, Integer, I32, B6, 10, 5);
    test_aggregation!(int_i32_bmax, Integer, I32, Bmax, 10, 5);

    test_aggregation!(prime_i32_b0, Prime, I32, B0, 10, 5);
    test_aggregation!(prime_i32_b2, Prime, I32, B2, 10, 5);
    test_aggregation!(prime_i32_b4, Prime, I32, B4, 10, 5);
    test_aggregation!(prime_i32_b6, Prime, I32, B6, 10, 5);
    test_aggregation!(prime_i32_bmax, Prime, I32, Bmax, 10, 5);

    test_aggregation!(pow_i32_b0, Power2, I32, B0, 10, 5);
    test_aggregation!(pow_i32_b2, Power2, I32, B2, 10, 5);
    test_aggregation!(pow_i32_b4, Power2, I32, B4, 10, 5);
    test_aggregation!(pow_i32_b6, Power2, I32, B6, 10, 5);
    test_aggregation!(pow_i32_bmax, Power2, I32, Bmax, 10, 5);

    test_aggregation!(int_i64_b0, Integer, I64, B0, 10, 5);
    test_aggregation!(int_i64_b2, Integer, I64, B2, 10, 5);
    test_aggregation!(int_i64_b4, Integer, I64, B4, 10, 5);
    test_aggregation!(int_i64_b6, Integer, I64, B6, 10, 5);
    test_aggregation!(int_i64_bmax, Integer, I64, Bmax, 10, 5);

    test_aggregation!(prime_i64_b0, Prime, I64, B0, 10, 5);
    test_aggregation!(prime_i64_b2, Prime, I64, B2, 10, 5);
    test_aggregation!(prime_i64_b4, Prime, I64, B4, 10, 5);
    test_aggregation!(prime_i64_b6, Prime, I64, B6, 10, 5);
    test_aggregation!(prime_i64_bmax, Prime, I64, Bmax, 10, 5);

    test_aggregation!(pow_i64_b0, Power2, I64, B0, 10, 5);
    test_aggregation!(pow_i64_b2, Power2, I64, B2, 10, 5);
    test_aggregation!(pow_i64_b4, Power2, I64, B4, 10, 5);
    test_aggregation!(pow_i64_b6, Power2, I64, B6, 10, 5);
    test_aggregation!(pow_i64_bmax, Power2, I64, Bmax, 10, 5);

    /// Generate tests for masking, aggregation and unmasking of multiple models:
    /// - generate random weights from a uniform distribution with a seeded PRNG
    /// - create a model from the weights, mask and aggregate it to the aggregated masked models
    /// - derive a mask from the mask seed and aggregate it to the aggregated masks
    /// - unmask the aggregated masked model
    /// - check that all aggregated unmasked weights are equal to the averaged original weights (up
    ///   to a tolerance determined by the masking configuration)
    ///
    /// The arguments to the macro are:
    /// - a suffix for the test name
    /// - the group type of the model (variants of `GroupType`)
    /// - the data type of the model (either primitives or variants of `DataType`)
    /// - an absolute bound for the weights (optional, choices: 1, 100, 10_000, 1_000_000)
    /// - the number of weights per model
    /// - the number of models
    macro_rules! test_masking_and_aggregation {
        ($suffix:ident, $group:ty, $data:ty, $bound:expr, $len:expr, $count:expr $(,)?) => {
            paste::item! {
                #[test]
                fn [<test_masking_and_aggregation_ $suffix>]() {
                    // Step 1: Build the masking config
                    let config = MaskConfig {
                        group_type: $group,
                        data_type: paste::expr! { [<$data:upper>] },
                        bound_type: match $bound {
                            1 => B0,
                            100 => B2,
                            10_000 => B4,
                            1_000_000 => B6,
                            _ => Bmax,
                        },
                        model_type: M3,
                    };
                    let model_size = $len as usize;

                    // Step 2: Generate random models
                    let bound = if $bound == 0 {
                        paste::expr! { [<$data:lower>]::MAX / (2 as [<$data:lower>]) }
                    } else {
                        paste::expr! { $bound as [<$data:lower>] }
                    };
                    let mut prng = ChaCha20Rng::from_seed(MaskSeed::generate().as_array());
                    let mut models = iter::repeat_with(move || {
                        Model::from_primitives(
                            Uniform::new_inclusive(-bound, bound)
                                .sample_iter(&mut prng)
                                .take($len as usize)
                        )
                        .unwrap()
                    });

                    // Step 3 (actual test):
                    // a. average the model weights for later checks
                    // b. mask the model
                    // c. derive the mask corresponding to the seed used
                    // d. aggregate the masked model resp. mask
                    // e. repeat a-d, then unmask the model and check it against the averaged one
                    let mut averaged_model = Model::from_primitives(
                        iter::repeat(paste::expr! { 0 as [<$data:lower>] }).take($len as usize)
                    )
                    .unwrap();
                    let mut aggregated_masked_model = Aggregation::new(config, model_size);
                    let mut aggregated_mask = Aggregation::new(config, model_size);
                    let mut aggregated_masked_scalar = Aggregation::new(config, 1);
                    let mut aggregated_scalar_mask = Aggregation::new(config, 1);
                    let scalar = 1_f64 / ($count as f64);
                    let scalar_ratio = Ratio::from_float(scalar).unwrap();
                    for _ in 0..$count as usize {
                        let model = models.next().unwrap();
                        averaged_model
                            .iter_mut()
                            .zip(model.iter())
                            .for_each(|(averaged_weight, weight)| {
                                *averaged_weight += &scalar_ratio * weight;
                            });

                        let (mask_seed, masked_model, masked_scalar) =
                            Masker::new(config).mask(scalar, model);
                        let (mask, scalar_mask) = mask_seed.derive_mask($len as usize, config);

                        assert!(
                            aggregated_masked_model.validate_aggregation(&masked_model).is_ok()
                        );
                        aggregated_masked_model.aggregate(masked_model);
                        assert!(aggregated_mask.validate_aggregation(&mask).is_ok());
                        aggregated_mask.aggregate(mask);

                        assert!(
                            aggregated_masked_scalar.validate_aggregation(&masked_scalar).is_ok()
                        );
                        aggregated_masked_scalar.aggregate(masked_scalar);
                        assert!(aggregated_scalar_mask.validate_aggregation(&scalar_mask).is_ok());
                        aggregated_scalar_mask.aggregate(scalar_mask);
                    }

                    let unmasked_model = aggregated_masked_model.unmask(aggregated_mask.into());
                    let tolerance = Ratio::from_integer(BigInt::from($count as usize))
                        / Ratio::from_integer(config.exp_shift());
                    assert!(
                        averaged_model.iter()
                            .zip(unmasked_model.iter())
                            .all(|(averaged_weight, unmasked_weight)| {
                                (averaged_weight - unmasked_weight).abs() <= tolerance
                            })
                    );
                    // TODO check scalar as well, after future refactoring
                }
            }
        };
        ($suffix:ident, $group:ty, $data:ty, $len:expr, $count:expr $(,)?) => {
            test_masking_and_aggregation!($suffix, $group, $data, 0, $len, $count);
        };
    }

    test_masking_and_aggregation!(int_f32_b0, Integer, f32, 1, 10, 5);
    test_masking_and_aggregation!(int_f32_b2, Integer, f32, 100, 10, 5);
    test_masking_and_aggregation!(int_f32_b4, Integer, f32, 10_000, 10, 5);
    test_masking_and_aggregation!(int_f32_b6, Integer, f32, 1_000_000, 10, 5);
    test_masking_and_aggregation!(int_f32_bmax, Integer, f32, 10, 5);

    test_masking_and_aggregation!(prime_f32_b0, Prime, f32, 1, 10, 5);
    test_masking_and_aggregation!(prime_f32_b2, Prime, f32, 100, 10, 5);
    test_masking_and_aggregation!(prime_f32_b4, Prime, f32, 10_000, 10, 5);
    test_masking_and_aggregation!(prime_f32_b6, Prime, f32, 1_000_000, 10, 5);
    test_masking_and_aggregation!(prime_f32_bmax, Prime, f32, 10, 5);

    test_masking_and_aggregation!(pow_f32_b0, Power2, f32, 1, 10, 5);
    test_masking_and_aggregation!(pow_f32_b2, Power2, f32, 100, 10, 5);
    test_masking_and_aggregation!(pow_f32_b4, Power2, f32, 10_000, 10, 5);
    test_masking_and_aggregation!(pow_f32_b6, Power2, f32, 1_000_000, 10, 5);
    test_masking_and_aggregation!(pow_f32_bmax, Power2, f32, 10, 5);

    test_masking_and_aggregation!(int_f64_b0, Integer, f64, 1, 10, 5);
    test_masking_and_aggregation!(int_f64_b2, Integer, f64, 100, 10, 5);
    test_masking_and_aggregation!(int_f64_b4, Integer, f64, 10_000, 10, 5);
    test_masking_and_aggregation!(int_f64_b6, Integer, f64, 1_000_000, 10, 5);
    test_masking_and_aggregation!(int_f64_bmax, Integer, f64, 10, 5);

    test_masking_and_aggregation!(prime_f64_b0, Prime, f64, 1, 10, 5);
    test_masking_and_aggregation!(prime_f64_b2, Prime, f64, 100, 10, 5);
    test_masking_and_aggregation!(prime_f64_b4, Prime, f64, 10_000, 10, 5);
    test_masking_and_aggregation!(prime_f64_b6, Prime, f64, 1_000_000, 10, 5);
    test_masking_and_aggregation!(prime_f64_bmax, Prime, f64, 10, 5);

    test_masking_and_aggregation!(pow_f64_b0, Power2, f64, 1, 10, 5);
    test_masking_and_aggregation!(pow_f64_b2, Power2, f64, 100, 10, 5);
    test_masking_and_aggregation!(pow_f64_b4, Power2, f64, 10_000, 10, 5);
    test_masking_and_aggregation!(pow_f64_b6, Power2, f64, 1_000_000, 10, 5);
    test_masking_and_aggregation!(pow_f64_bmax, Power2, f64, 10, 5);

    test_masking_and_aggregation!(int_i32_b0, Integer, i32, 1, 10, 5);
    test_masking_and_aggregation!(int_i32_b2, Integer, i32, 100, 10, 5);
    test_masking_and_aggregation!(int_i32_b4, Integer, i32, 10_000, 10, 5);
    test_masking_and_aggregation!(int_i32_b6, Integer, i32, 1_000_000, 10, 5);
    test_masking_and_aggregation!(int_i32_bmax, Integer, i32, 10, 5);

    test_masking_and_aggregation!(prime_i32_b0, Prime, i32, 1, 10, 5);
    test_masking_and_aggregation!(prime_i32_b2, Prime, i32, 100, 10, 5);
    test_masking_and_aggregation!(prime_i32_b4, Prime, i32, 10_000, 10, 5);
    test_masking_and_aggregation!(prime_i32_b6, Prime, i32, 1_000_000, 10, 5);
    test_masking_and_aggregation!(prime_i32_bmax, Prime, i32, 10, 5);

    test_masking_and_aggregation!(pow_i32_b0, Power2, i32, 1, 10, 5);
    test_masking_and_aggregation!(pow_i32_b2, Power2, i32, 100, 10, 5);
    test_masking_and_aggregation!(pow_i32_b4, Power2, i32, 10_000, 10, 5);
    test_masking_and_aggregation!(pow_i32_b6, Power2, i32, 1_000_000, 10, 5);
    test_masking_and_aggregation!(pow_i32_bmax, Power2, i32, 10, 5);

    test_masking_and_aggregation!(int_i64_b0, Integer, i64, 1, 10, 5);
    test_masking_and_aggregation!(int_i64_b2, Integer, i64, 100, 10, 5);
    test_masking_and_aggregation!(int_i64_b4, Integer, i64, 10_000, 10, 5);
    test_masking_and_aggregation!(int_i64_b6, Integer, i64, 1_000_000, 10, 5);
    test_masking_and_aggregation!(int_i64_bmax, Integer, i64, 10, 5);

    test_masking_and_aggregation!(prime_i64_b0, Prime, i64, 1, 10, 5);
    test_masking_and_aggregation!(prime_i64_b2, Prime, i64, 100, 10, 5);
    test_masking_and_aggregation!(prime_i64_b4, Prime, i64, 10_000, 10, 5);
    test_masking_and_aggregation!(prime_i64_b6, Prime, i64, 1_000_000, 10, 5);
    test_masking_and_aggregation!(prime_i64_bmax, Prime, i64, 10, 5);

    test_masking_and_aggregation!(pow_i64_b0, Power2, i64, 1, 10, 5);
    test_masking_and_aggregation!(pow_i64_b2, Power2, i64, 100, 10, 5);
    test_masking_and_aggregation!(pow_i64_b4, Power2, i64, 10_000, 10, 5);
    test_masking_and_aggregation!(pow_i64_b6, Power2, i64, 1_000_000, 10, 5);
    test_masking_and_aggregation!(pow_i64_bmax, Power2, i64, 10, 5);
}
