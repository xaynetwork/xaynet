//! Masking, aggregation and unmasking of models.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: crate::mask

use std::iter::{self, Iterator};

use num::{
    bigint::{BigInt, BigUint, ToBigInt},
    clamp,
    rational::Ratio,
    traits::clamp_max,
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use thiserror::Error;

use crate::{
    crypto::{prng::generate_integer, ByteObject},
    mask::{
        config::MaskConfigPair,
        model::Model,
        object::{MaskObject, MaskUnit, MaskVect},
        scalar::Scalar,
        seed::MaskSeed,
    },
};

#[derive(Debug, Error, Eq, PartialEq)]
/// Errors related to the unmasking of models.
pub enum UnmaskingError {
    #[error("there is no model to unmask")]
    NoModel,

    #[error("too many models were aggregated for the current unmasking configuration")]
    TooManyModels,

    #[error("too many scalars were aggregated for the current unmasking configuration")]
    TooManyScalars,

    #[error("the masked model is incompatible with the mask used for unmasking")]
    MaskManyMismatch,

    #[error("the masked scalar is incompatible with the mask used for unmasking")]
    MaskOneMismatch,

    #[error("the mask is invalid")]
    InvalidMask,
}

#[derive(Debug, Error)]
/// Errors related to the aggregation of masks and models.
pub enum AggregationError {
    // TODO rename Model -> Vector; or use MaskMany/One terminology
    #[error("the object to aggregate is invalid")]
    InvalidObject,

    #[error("too many models were aggregated for the current unmasking configuration")]
    TooManyModels,

    #[error("too many scalars were aggregated for the current unmasking configuration")]
    TooManyScalars,

    #[error("the model to aggregate is incompatible with the current aggregated scalar")]
    ModelMismatch,

    #[error("the scalar to aggregate is incompatible with the current aggregated scalar")]
    ScalarMismatch,
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
            object_size: object.vect.data.len(),
            object,
        }
    }
}

impl From<Aggregation> for MaskObject {
    fn from(aggr: Aggregation) -> Self {
        aggr.object
    }
}

#[allow(clippy::len_without_is_empty)]
impl Aggregation {
    /// Creates a new, empty aggregator for masks or masked models.
    pub fn new(config: MaskConfigPair, object_size: usize) -> Self {
        Self {
            nb_models: 0,
            object: MaskObject::empty(config, object_size),
            object_size,
        }
    }

    /// Gets the length of the aggregated mask object.
    pub fn len(&self) -> usize {
        self.object_size
    }

    /// Gets the masking configurations of the aggregator.
    pub fn config(&self) -> MaskConfigPair {
        MaskConfigPair {
            vect: self.object.vect.config,
            unit: self.object.unit.config,
        }
    }

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
    /// [`unmask()`]: Aggregation::unmask
    pub fn validate_unmasking(&self, mask: &MaskObject) -> Result<(), UnmaskingError> {
        // We cannot perform unmasking without at least one real model
        if self.nb_models == 0 {
            return Err(UnmaskingError::NoModel);
        }

        if self.nb_models > self.object.vect.config.model_type.max_nb_models() {
            return Err(UnmaskingError::TooManyModels);
        }

        if self.nb_models > self.object.unit.config.model_type.max_nb_models() {
            return Err(UnmaskingError::TooManyScalars);
        }

        if self.object.vect.config != mask.vect.config || self.object_size != mask.vect.data.len() {
            return Err(UnmaskingError::MaskManyMismatch);
        }

        if self.object.unit.config != mask.unit.config {
            return Err(UnmaskingError::MaskOneMismatch);
        }

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
    /// [`validate_unmasking()`]: Aggregation::validate_unmasking
    /// [`mask()`]: Masker::mask
    pub fn unmask(self, mask_obj: MaskObject) -> Model {
        let MaskObject { vect, unit } = self.object;
        let (masked_n, config_n) = (vect.data, vect.config);
        let (masked_1, config_1) = (unit.data, unit.config);
        let mask_n = mask_obj.vect.data;
        let mask_1 = mask_obj.unit.data;

        // unmask scalar sum
        let scaled_add_shift_1 = config_1.add_shift() * BigInt::from(self.nb_models);
        let exp_shift_1 = config_1.exp_shift();
        let order_1 = config_1.order();
        let n = (masked_1 + &order_1 - mask_1) % &order_1;
        let ratio = Ratio::<BigInt>::from(n.to_bigint().unwrap());
        let scalar_sum = ratio / &exp_shift_1 - &scaled_add_shift_1;

        // unmask global model
        let scaled_add_shift_n = config_n.add_shift() * BigInt::from(self.nb_models);
        let exp_shift_n = config_n.exp_shift();
        let order_n = config_n.order();
        masked_n
            .into_iter()
            .zip(mask_n)
            .map(|(masked, mask)| {
                // PANIC_SAFE: The substraction panics if it
                // underflows, which can only happen if:
                //
                //     mask > order_n
                //
                // If the mask is valid, we are guaranteed that this
                // cannot happen. Thus this method may panic only if
                // given an invalid mask.
                let n = (masked + &order_n - mask) % &order_n;

                // UNWRAP_SAFE: to_bigint never fails for BigUint
                let ratio = Ratio::<BigInt>::from(n.to_bigint().unwrap());
                let unmasked = ratio / &exp_shift_n - &scaled_add_shift_n;

                // scaling correction
                unmasked / &scalar_sum
            })
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
    /// [`aggregate()`]: Aggregation::aggregate
    pub fn validate_aggregation(&self, object: &MaskObject) -> Result<(), AggregationError> {
        if self.object.vect.config != object.vect.config {
            return Err(AggregationError::ModelMismatch);
        }

        if self.object.unit.config != object.unit.config {
            return Err(AggregationError::ScalarMismatch);
        }

        if self.object_size != object.vect.data.len() {
            return Err(AggregationError::ModelMismatch);
        }

        if self.nb_models >= self.object.vect.config.model_type.max_nb_models() {
            return Err(AggregationError::TooManyModels);
        }

        if self.nb_models >= self.object.unit.config.model_type.max_nb_models() {
            return Err(AggregationError::TooManyScalars);
        }

        if !object.is_valid() {
            return Err(AggregationError::InvalidObject);
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
    /// [`validate_aggregation()`]: Aggregation::validate_aggregation
    pub fn aggregate(&mut self, object: MaskObject) {
        if self.nb_models == 0 {
            self.object = object;
            self.nb_models = 1;
            return;
        }

        let order_n = self.object.vect.config.order();
        for (i, j) in self
            .object
            .vect
            .data
            .iter_mut()
            .zip(object.vect.data.into_iter())
        {
            *i = (&*i + j) % &order_n
        }

        let order_1 = self.object.unit.config.order();
        let a = &mut self.object.unit.data;
        let b = object.unit.data;
        *a = (&*a + b) % &order_1;

        self.nb_models += 1;
    }
}

/// A masker for models.
pub struct Masker {
    config: MaskConfigPair,
    seed: MaskSeed,
}

impl Masker {
    /// Creates a new masker with the given masking `config`uration with a randomly generated seed.
    pub fn new(config: MaskConfigPair) -> Self {
        Self {
            config,
            seed: MaskSeed::generate(),
        }
    }

    /// Creates a new masker with the given masking `config`uration and `seed`.
    pub fn with_seed(config: MaskConfigPair, seed: MaskSeed) -> Self {
        Self { config, seed }
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
    /// [`unmask()`]: Aggregation::unmask
    pub fn mask(self, scalar: Scalar, model: &Model) -> (MaskSeed, MaskObject) {
        let (random_int, mut random_ints) = self.random_ints();
        let Self { config, seed } = self;
        let MaskConfigPair {
            vect: config_n,
            unit: config_1,
        } = config;

        // clamp the scalar
        let add_shift_1 = config_1.add_shift();
        let scalar_ratio = scalar.into();
        let scalar_clamped = clamp_max(&scalar_ratio, &add_shift_1);

        let exp_shift_n = config_n.exp_shift();
        let add_shift_n = config_n.add_shift();
        let order_n = config_n.order();
        let higher_bound = &add_shift_n;
        let lower_bound = -&add_shift_n;

        // mask the (scaled) weights
        let masked_weights = model
            .iter()
            .zip(&mut random_ints)
            .map(|(weight, rand_int)| {
                let scaled = scalar_clamped * weight;
                let scaled_clamped = clamp(&scaled, &lower_bound, higher_bound);
                // PANIC_SAFE: shifted weight is guaranteed to be non-negative
                let shifted = ((scaled_clamped + &add_shift_n) * &exp_shift_n)
                    .to_integer()
                    .to_biguint()
                    .unwrap();
                (shifted + rand_int) % &order_n
            })
            .collect();
        let masked_model = MaskVect::new_unchecked(config_n, masked_weights);

        // mask the scalar
        // PANIC_SAFE: shifted scalar is guaranteed to be non-negative
        let shifted = ((scalar_clamped + &add_shift_1) * config_1.exp_shift())
            .to_integer()
            .to_biguint()
            .unwrap();
        let masked = (shifted + random_int) % config_1.order();
        let masked_scalar = MaskUnit::new_unchecked(config_1, masked);

        (seed, MaskObject::new_unchecked(masked_model, masked_scalar))
    }

    /// Randomly generates integers wrt the masking configurations.
    ///
    /// The first is generated wrt the scalar configuration, while the rest are
    /// wrt the vector configuration and returned as an iterator.
    fn random_ints(&self) -> (BigUint, impl Iterator<Item = BigUint>) {
        let order_n = self.config.vect.order();
        let order_1 = self.config.unit.order();
        let mut prng = ChaCha20Rng::from_seed(self.seed.as_array());
        let int = generate_integer(&mut prng, &order_1);
        let ints = iter::from_fn(move || Some(generate_integer(&mut prng, &order_n)));
        (int, ints)
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
        scalar::FromPrimitive,
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
                    let vect_len = $len as usize;

                    // Step 2: Generate a random model
                    let bound = if $bound == 0 {
                        paste::expr! { [<$data:lower>]::MAX / (2.1 as [<$data:lower>]) }
                    } else {
                        paste::expr! { $bound as [<$data:lower>] }
                    };
                    let mut prng = ChaCha20Rng::from_seed(MaskSeed::generate().as_array());
                    let random_weights = Uniform::new_inclusive(-bound, bound)
                        .sample_iter(&mut prng)
                        .take(vect_len);
                    let model = Model::from_primitives(random_weights).unwrap();
                    assert_eq!(model.len(), vect_len);

                    // Step 3 (actual test):
                    // a. mask the model
                    // b. derive the mask corresponding to the seed used
                    // c. unmask the model and check it against the original one.
                    let (mask_seed, masked_model) =
                        Masker::new(config.into()).mask(Scalar::unit(), &model);
                    assert_eq!(masked_model.vect.data.len(), vect_len);
                    assert!(masked_model.is_valid());

                    let mask = mask_seed.derive_mask(vect_len, config.into());
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

    /// Generate tests for masking and unmasking of a single model:
    /// - generate random scalar from a uniform distribution with a seeded PRNG
    /// - scale a model of unit weights and mask it
    /// - check that all masked weights belong to the chosen finite group
    /// - unmask the masked model
    /// - check that all unmasked weights are equal to the original weights (up to a tolerance
    ///   determined by the masking configuration)
    ///
    /// The arguments to the macro are:
    /// - a suffix for the test name
    /// - the group type of the model and scalar (variants of `GroupType`)
    /// - the data type of the model and scalar (either float primitives or float variants of
    ///   `DataType`)
    /// - an absolute bound for the scalar (optional, choices: 1, 100, 10_000, 1_000_000)
    /// - the number of weights
    macro_rules! test_masking_scalar {
        ($suffix:ident, $group:ty, $data:ty, $bound:expr, $len:expr $(,)?) => {
            paste::item! {
                #[test]
                fn [<test_masking_scalar_ $suffix>]() {
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
                    let vect_len = $len as usize;

                    // Step 2: Generate a random scalar from (0, bound]
                    // take vector [1, ..., 1] as the model to scale
                    let bound = if $bound == 0 {
                        paste::expr! { [<$data:lower>]::MAX / (2.1 as [<$data:lower>]) }
                    } else {
                        paste::expr! { $bound as [<$data:lower>] }
                    };
                    let eps = [<$data:lower>]::EPSILON;
                    let mut prng = ChaCha20Rng::from_seed(MaskSeed::generate().as_array());
                    let random_weight = Uniform::new_inclusive(eps, bound).sample(&mut prng);
                    let scalar = Scalar::from_primitive(random_weight).unwrap();
                    let model = Model::from_primitives(iter::repeat(1).take(vect_len)).unwrap();
                    assert_eq!(model.len(), vect_len);

                    // Step 3 (actual test):
                    // a. mask the model
                    // b. derive the mask corresponding to the seed used
                    // c. unmask the model and check it against the expected [1, ..., 1]
                    let (mask_seed, masked_model) =
                        Masker::new(config.into()).mask(scalar, &model);
                    assert_eq!(masked_model.vect.data.len(), vect_len);
                    assert!(masked_model.is_valid());

                    let mask = mask_seed.derive_mask(vect_len, config.into());
                    let unmasked_model = Aggregation::from(masked_model).unmask(mask);

                    let tolerance = Ratio::from_integer(config.exp_shift()).recip();
                    let expected_weight = Ratio::from_integer(BigInt::from(1));
                    assert!(
                        unmasked_model
                            .iter()
                            .all(|unmasked_weight| {
                                (unmasked_weight - &expected_weight).abs() <= tolerance
                            })
                    );
                }
            }
        };
        ($suffix:ident, $group:ty, $data:ty, $len:expr $(,)?) => {
            test_masking_scalar!($suffix, $group, $data, 0, $len);
        };
    }

    test_masking_scalar!(int_f32_b0, Integer, f32, 1, 10);
    test_masking_scalar!(int_f32_b2, Integer, f32, 100, 10);
    test_masking_scalar!(int_f32_b4, Integer, f32, 10_000, 10);
    test_masking_scalar!(int_f32_b6, Integer, f32, 1_000_000, 10);
    test_masking_scalar!(int_f32_bmax, Integer, f32, 10);

    test_masking_scalar!(prime_f32_b0, Prime, f32, 1, 10);
    test_masking_scalar!(prime_f32_b2, Prime, f32, 100, 10);
    test_masking_scalar!(prime_f32_b4, Prime, f32, 10_000, 10);
    test_masking_scalar!(prime_f32_b6, Prime, f32, 1_000_000, 10);
    test_masking_scalar!(prime_f32_bmax, Prime, f32, 10);

    test_masking_scalar!(pow_f32_b0, Power2, f32, 1, 10);
    test_masking_scalar!(pow_f32_b2, Power2, f32, 100, 10);
    test_masking_scalar!(pow_f32_b4, Power2, f32, 10_000, 10);
    test_masking_scalar!(pow_f32_b6, Power2, f32, 1_000_000, 10);
    test_masking_scalar!(pow_f32_bmax, Power2, f32, 10);

    test_masking_scalar!(int_f64_b0, Integer, f64, 1, 10);
    test_masking_scalar!(int_f64_b2, Integer, f64, 100, 10);
    test_masking_scalar!(int_f64_b4, Integer, f64, 10_000, 10);
    test_masking_scalar!(int_f64_b6, Integer, f64, 1_000_000, 10);
    test_masking_scalar!(int_f64_bmax, Integer, f64, 10);

    test_masking_scalar!(prime_f64_b0, Prime, f64, 1, 10);
    test_masking_scalar!(prime_f64_b2, Prime, f64, 100, 10);
    test_masking_scalar!(prime_f64_b4, Prime, f64, 10_000, 10);
    test_masking_scalar!(prime_f64_b6, Prime, f64, 1_000_000, 10);
    test_masking_scalar!(prime_f64_bmax, Prime, f64, 10);

    test_masking_scalar!(pow_f64_b0, Power2, f64, 1, 10);
    test_masking_scalar!(pow_f64_b2, Power2, f64, 100, 10);
    test_masking_scalar!(pow_f64_b4, Power2, f64, 10_000, 10);
    test_masking_scalar!(pow_f64_b6, Power2, f64, 1_000_000, 10);
    test_masking_scalar!(pow_f64_bmax, Power2, f64, 10);

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
                    let vect_len = $len as usize;

                    // Step 2: generate random masked models
                    let mut prng = ChaCha20Rng::from_seed(MaskSeed::generate().as_array());
                    let mut masked_models = iter::repeat_with(move || {
                        let order = config.order();
                        let integer = generate_integer(&mut prng, &order);
                        let integers = iter::repeat_with(|| generate_integer(&mut prng, &order))
                            .take(vect_len)
                            .collect::<Vec<_>>();
                        MaskObject::new(config.into(), integers, integer).unwrap()
                    });

                    // Step 3 (actual test):
                    // a. aggregate the masked models
                    // b. check the aggregated masked model
                    let mut aggregated_masked_model = Aggregation::new(config.into(), vect_len);
                    for nb in 1..$count as usize + 1 {
                        let masked_model = masked_models.next().unwrap();
                        assert!(
                            aggregated_masked_model.validate_aggregation(&masked_model).is_ok()
                        );
                        aggregated_masked_model.aggregate(masked_model);

                        assert_eq!(aggregated_masked_model.nb_models, nb);
                        assert_eq!(aggregated_masked_model.object.vect.data.len(), vect_len);
                        assert_eq!(aggregated_masked_model.object.vect.config, config);
                        assert_eq!(aggregated_masked_model.object.unit.config, config);
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
                    let vect_len = $len as usize;
                    let model_count = $count as usize;

                    // Step 2: Generate random models
                    let bound = if $bound == 0 {
                        paste::expr! { [<$data:lower>]::MAX / (2.1 as [<$data:lower>]) }
                    } else {
                        paste::expr! { $bound as [<$data:lower>] }
                    };
                    let mut prng = ChaCha20Rng::from_seed(MaskSeed::generate().as_array());
                    let mut models = iter::repeat_with(move || {
                        Model::from_primitives(
                            Uniform::new_inclusive(-bound, bound)
                                .sample_iter(&mut prng)
                                .take(vect_len)
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
                        iter::repeat(paste::expr! { 0 as [<$data:lower>] }).take(vect_len)
                    )
                    .unwrap();
                    let mut aggregated_masked_model = Aggregation::new(config.into(), vect_len);
                    let mut aggregated_mask = Aggregation::new(config.into(), vect_len);
                    let scalar = Scalar::new(1, model_count);
                    let scalar_ratio = &scalar.to_ratio();
                    for _ in 0..model_count {
                        let model = models.next().unwrap();
                        averaged_model
                            .iter_mut()
                            .zip(model.iter())
                            .for_each(|(averaged_weight, weight)| {
                                *averaged_weight += scalar_ratio * weight;
                            });

                        let (mask_seed, masked_model) =
                            Masker::new(config.into()).mask(scalar.clone(), &model);
                        let mask = mask_seed.derive_mask(vect_len, config.into());

                        assert!(
                            aggregated_masked_model.validate_aggregation(&masked_model).is_ok()
                        );
                        aggregated_masked_model.aggregate(masked_model);
                        assert!(aggregated_mask.validate_aggregation(&mask).is_ok());
                        aggregated_mask.aggregate(mask);
                    }

                    let mask = aggregated_mask.into();
                    assert!(aggregated_masked_model.validate_unmasking(&mask).is_ok());
                    let unmasked_model = aggregated_masked_model.unmask(mask);
                    let tolerance = Ratio::from_integer(BigInt::from(model_count))
                        / Ratio::from_integer(config.exp_shift());
                    assert!(
                        averaged_model.iter()
                            .zip(unmasked_model.iter())
                            .all(|(averaged_weight, unmasked_weight)| {
                                (averaged_weight - unmasked_weight).abs() <= tolerance
                            })
                    );
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

    /// Generate tests for masking, aggregation and unmasking of multiple models:
    /// - generate random scalars from a uniform distribution with a seeded PRNG
    /// - scale a model of unit weights, mask and aggregate it to the aggregated masked models
    /// - derive a mask from the mask seed and aggregate it to the aggregated masks
    /// - unmask the aggregated masked model
    /// - check that all aggregated unmasked weights are equal to the original unit weights (up
    ///   to a tolerance determined by the masking configuration)
    ///
    /// The arguments to the macro are:
    /// - a suffix for the test name
    /// - the group type of the model and scalar (variants of `GroupType`)
    /// - the data type of the model and scalar (either float primitives or float variants of
    ///   `DataType`)
    /// - an absolute bound for the scalar (optional, choices: 1, 100, 10_000, 1_000_000)
    /// - the number of weights per model
    /// - the number of models
    macro_rules! test_masking_and_aggregation_scalar {
        ($suffix:ident, $group:ty, $data:ty, $bound:expr, $len:expr, $count:expr $(,)?) => {
            paste::item! {
                #[test]
                fn [<test_masking_and_aggregation_scalar $suffix>]() {
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
                    let vect_len = $len as usize;
                    let model_count = $count as usize;

                    // Step 2: Generate random scalars
                    // take vectors [1, ..., 1] as models to scale
                    let bound = if $bound == 0 {
                        paste::expr! { [<$data:lower>]::MAX / (2 as [<$data:lower>]) }
                    } else {
                        paste::expr! { $bound as [<$data:lower>] }
                    };
                    let eps = [<$data:lower>]::EPSILON;
                    let mut prng = ChaCha20Rng::from_seed(MaskSeed::generate().as_array());
                    let mut scalars = iter::repeat_with(move || {
                        let random_weight = Uniform::new_inclusive(eps, bound).sample(&mut prng);
                        Scalar::from_primitive(random_weight).unwrap()
                    });
                    let mut models =
                        iter::repeat(Model::from_primitives(iter::repeat(1).take(vect_len)).unwrap());

                    // Step 3 (actual test):
                    // a. mask the model
                    // b. derive the mask corresponding to the seed used
                    // c. aggregate the masked model resp. mask
                    // d. repeat a-c, unmask the model and check it against the expected [1, ..., 1]
                    let mut aggregated_masked_model = Aggregation::new(config.into(), vect_len);
                    let mut aggregated_mask = Aggregation::new(config.into(), vect_len);
                    for _ in 0..model_count {
                        let model = models.next().unwrap();
                        let scalar = scalars.next().unwrap();

                        let (mask_seed, masked_model) =
                            Masker::new(config.into()).mask(scalar, &model);
                        let mask = mask_seed.derive_mask(vect_len, config.into());

                        assert!(
                            aggregated_masked_model.validate_aggregation(&masked_model).is_ok()
                        );
                        aggregated_masked_model.aggregate(masked_model);
                        assert!(aggregated_mask.validate_aggregation(&mask).is_ok());
                        aggregated_mask.aggregate(mask);
                    }

                    let mask = aggregated_mask.into();
                    assert!(aggregated_masked_model.validate_unmasking(&mask).is_ok());
                    let unmasked_model = aggregated_masked_model.unmask(mask);
                    let tolerance = Ratio::from_integer(BigInt::from(model_count))
                        / Ratio::from_integer(config.exp_shift());
                    let expected_weight = Ratio::from_integer(BigInt::from(1));
                    assert!(
                        unmasked_model
                            .iter()
                            .all(|unmasked_weight| {
                                (unmasked_weight - &expected_weight).abs() <= tolerance
                            })
                    );
                }
            }
        };
        ($suffix:ident, $group:ty, $data:ty, $len:expr, $count:expr $(,)?) => {
            test_masking_and_aggregation_scalar!($suffix, $group, $data, 0, $len, $count);
        };
    }

    test_masking_and_aggregation_scalar!(int_f32_b0, Integer, f32, 1, 10, 5);
    test_masking_and_aggregation_scalar!(int_f32_b2, Integer, f32, 100, 10, 5);
    test_masking_and_aggregation_scalar!(int_f32_b4, Integer, f32, 10_000, 10, 5);
    test_masking_and_aggregation_scalar!(int_f32_b6, Integer, f32, 1_000_000, 10, 5);
    test_masking_and_aggregation_scalar!(int_f32_bmax, Integer, f32, 10, 2);

    test_masking_and_aggregation_scalar!(prime_f32_b0, Prime, f32, 1, 10, 5);
    test_masking_and_aggregation_scalar!(prime_f32_b2, Prime, f32, 100, 10, 5);
    test_masking_and_aggregation_scalar!(prime_f32_b4, Prime, f32, 10_000, 10, 5);
    test_masking_and_aggregation_scalar!(prime_f32_b6, Prime, f32, 1_000_000, 10, 5);
    test_masking_and_aggregation_scalar!(prime_f32_bmax, Prime, f32, 10, 2);

    test_masking_and_aggregation_scalar!(pow_f32_b0, Power2, f32, 1, 10, 5);
    test_masking_and_aggregation_scalar!(pow_f32_b2, Power2, f32, 100, 10, 5);
    test_masking_and_aggregation_scalar!(pow_f32_b4, Power2, f32, 10_000, 10, 5);
    test_masking_and_aggregation_scalar!(pow_f32_b6, Power2, f32, 1_000_000, 10, 5);
    test_masking_and_aggregation_scalar!(pow_f32_bmax, Power2, f32, 10, 2);

    test_masking_and_aggregation_scalar!(int_f64_b0, Integer, f64, 1, 10, 2);
    test_masking_and_aggregation_scalar!(int_f64_b2, Integer, f64, 100, 10, 2);
    test_masking_and_aggregation_scalar!(int_f64_b4, Integer, f64, 10_000, 10, 2);
    test_masking_and_aggregation_scalar!(int_f64_b6, Integer, f64, 1_000_000, 10, 2);
    test_masking_and_aggregation_scalar!(int_f64_bmax, Integer, f64, 10, 2);

    test_masking_and_aggregation_scalar!(prime_f64_b0, Prime, f64, 1, 10, 2);
    test_masking_and_aggregation_scalar!(prime_f64_b2, Prime, f64, 100, 10, 2);
    test_masking_and_aggregation_scalar!(prime_f64_b4, Prime, f64, 10_000, 10, 2);
    test_masking_and_aggregation_scalar!(prime_f64_b6, Prime, f64, 1_000_000, 10, 2);
    test_masking_and_aggregation_scalar!(prime_f64_bmax, Prime, f64, 10, 2);

    test_masking_and_aggregation_scalar!(pow_f64_b0, Power2, f64, 1, 10, 2);
    test_masking_and_aggregation_scalar!(pow_f64_b2, Power2, f64, 100, 10, 2);
    test_masking_and_aggregation_scalar!(pow_f64_b4, Power2, f64, 10_000, 10, 2);
    test_masking_and_aggregation_scalar!(pow_f64_b6, Power2, f64, 1_000_000, 10, 2);
    test_masking_and_aggregation_scalar!(pow_f64_bmax, Power2, f64, 10, 2);
}
