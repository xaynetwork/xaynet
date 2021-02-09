//! Masking, aggregation and unmasking of models.
//!
//! # Models
//! A [`Model`] is a collection of weights/parameters which are represented as finite numerical
//! values (i.e. rational numbers) of arbitrary precision. As such, a model in itself is not bound
//! to any particular primitive data type, but it can be created from those and converted back into
//! them.
//!
//! Currently, the primitive data types [`f32`], [`f64`], [`i32`] and [`i64`] are supported and
//! this might be extended in the future.
//!
//! ```
//! # use xaynet_core::mask::{FromPrimitives, IntoPrimitives, Model};
//! let weights = vec![0_f32; 10];
//! let model = Model::from_primitives_bounded(weights.into_iter());
//! assert_eq!(
//!     model.into_primitives_unchecked().collect::<Vec<f32>>(),
//!     vec![0_f32; 10],
//! );
//! ```
//!
//! # Masking configurations
//! The masking, aggregation and unmasking of models requires certain information about the models
//! to guarantee that no information is lost during the process, which is configured via the
//! [`MaskConfig`]. Each masking configuration consists of the group type, data type, bound type and
//! model type. Usually, a masking configuration is decided on and configured depending on the
//! specific machine learning use case as part of the setup for the XayNet federated learning
//! platform.
//!
//! Currently, those choices are catalogued for certain fixed variants for each type, but we aim
//! to generalize this in the future to more flexible masking configurations to allow for a more
//! fine-grained tradeoff between representability and performance.
//!
//! ## Group type
//! The [`GroupType`] describes the order of the finite group in which the masked model weights are
//! embedded. The smaller the gap between the maximum possible embedded weights and the group order
//! is, the less theoretically possible information flow about the masks may be observed. Specific
//! group orders provide potentially higher performance on the other hand, which always makes this
//! a tradeoff between security and performance. The group type variants are:
//! - Integer: no gap but potentially slowest performance.
//! - Prime: usually small gap with higher performance.
//! - Power2: usually higher gap with potentially highest performance.
//!
//! ## Data type
//! The [`DataType`] describes the original primitive data type of the model weights. This in
//! combination with the bound type influences the preserved decimal places of the model weights
//! during the masking, aggregation and unmasking process, which are:
//! - F32: 10 decimal places for bounded model weights and 45 decimal places for unbounded.
//! - F64: 20 decimal places for bounded model weights and 324 decimal places for unbounded.
//! - I32 and I64: 10 decimal places (required for scaled aggregation).
//!
//! Currently the primitive data types [`f32`], [`f64`], [`i32`] and [`i64`] are supported via the
//! data type variants.
//!
//! ## Bound type
//! The [`BoundType`] describes the absolute bounds on all model weights. The smaller the bounds of
//! the model weights, the less bytes are required to represent the masked model weights. These
//! bounds are enforced on the model weights before masking them to prevent information loss during
//! the masking, aggregation and unmasking process. The bound type variants are:
//! - B0: all model weights are absolutely bounded by 1.
//! - B2: all model weights are absolutely bounded by 100.
//! - B4: all model weights are absolutely bounded by 10,000.
//! - B6: all model weights are absolutely bounded by 1,000,000.
//! - Bmax: all model weights are absolutely bounded by their primitive data type's absolute
//!   maximum value.
//!
//! ## Model type
//! The [`ModelType`] describes the maximum number of masked models that can be aggregated without
//! information loss. The smaller the number of masked models, the less bytes are required to
//! represent masked model weights. The model type variants are:
//! - M3: at most 1,000 masked models may be aggregated.
//! - M6: at most 1,000,000 masked models may be aggregated.
//! - M9: at most 1,000,000,000 masked models may be aggregated.
//! - M12: at most 1,000,000,000,000 masked models may be aggregated.
//!
//! # Masking, aggregation and unmasking
//! Local models should be masked (i.e. encrypted) before they are communicated somewhere else to
//! protect the possibly sensitive information learned from local data. The masking should allow
//! for masked models to be aggregated while they are still masked (i.e. homomorphic encryption).
//! Then the aggregated masked model can safely be unmasked without jeopardizing the secrecy of
//! personal information if the model is generalized enough.
//!
//! ## Masking
//! A [`Model`] can be masked with a [`Masker`], which requires a [`MaskConfig`]. During the
//! masking, the model weights are scaled, then embedded as elements of the chosen finite group and
//! finally masked by randomly generated elements from that very same finite group. The scalar
//! provides the necessary means to perform different aggregation strategies, for example federated
//! averaging. The masked model is returned as a [`MaskObject`] and the mask used to mask the model
//! can be generated via the additionally returned [`MaskSeed`].
//!
//! ```
//! # use xaynet_core::mask::{BoundType, DataType, FromPrimitives, GroupType, MaskConfig, Masker, Model, ModelType, Scalar};
//! // create local models and a fitting masking configuration
//! let number_weights = 10;
//! let scalar = Scalar::new(1, 2_u8);
//! let local_model_1 = Model::from_primitives_bounded(vec![0_f32; number_weights].into_iter());
//! let local_model_2 = Model::from_primitives_bounded(vec![1_f32; number_weights].into_iter());
//! let config = MaskConfig {
//!     group_type: GroupType::Prime,
//!     data_type: DataType::F32,
//!     bound_type: BoundType::B0,
//!     model_type: ModelType::M3,
//! };
//!
//! // mask the local models
//! let (local_mask_seed_1, masked_local_model_1) = Masker::new(config.into()).mask(scalar.clone(), &local_model_1);
//! let (local_mask_seed_2, masked_local_model_2) = Masker::new(config.into()).mask(scalar, &local_model_2);
//!
//! // derive the masks of the local masked models
//! let local_mask_1 = local_mask_seed_1.derive_mask(number_weights, config.into());
//! let local_mask_2 = local_mask_seed_2.derive_mask(number_weights, config.into());
//! ```
//!
//! ## Aggregation
//! Masked models can be aggregated via an [`Aggregation`]. Masks themselves can be aggregated via
//! an [`Aggregation`] as well. An aggregated masked model can only be unmasked by the aggregation
//! of masks for each model. Aggregation should always be validated beforehand so that it may be
//! safely performed wrt the chosen masking configuration without possible loss of information.
//!
//! ```
//! # use xaynet_core::mask::{Aggregation, BoundType, DataType, FromPrimitives, GroupType, MaskConfig, Masker, MaskObject, Model, ModelType, Scalar};
//! # let number_weights = 10;
//! # let scalar = Scalar::new(1, 2_u8);
//! # let local_model_1 = Model::from_primitives_bounded(vec![0_f32; number_weights].into_iter());
//! # let local_model_2 = Model::from_primitives_bounded(vec![1_f32; number_weights].into_iter());
//! # let config = MaskConfig { group_type: GroupType::Prime, data_type: DataType::F32, bound_type: BoundType::B0, model_type: ModelType::M3};
//! # let (local_mask_seed_1, masked_local_model_1) = Masker::new(config.into()).mask(scalar.clone(), &local_model_1);
//! # let (local_mask_seed_2, masked_local_model_2) = Masker::new(config.into()).mask(scalar, &local_model_2);
//! # let local_model_mask_1 = local_mask_seed_1.derive_mask(number_weights, config.into());
//! # let local_model_mask_2 = local_mask_seed_2.derive_mask(number_weights, config.into());
//! // aggregate the local model masks (similarly for local scalar masks)
//! let mut mask_aggregator = Aggregation::new(config.into(), number_weights);
//! if let Ok(_) = mask_aggregator.validate_aggregation(&local_model_mask_1) {
//!     mask_aggregator.aggregate(local_model_mask_1);
//! };
//! if let Ok(_) = mask_aggregator.validate_aggregation(&local_model_mask_2) {
//!     mask_aggregator.aggregate(local_model_mask_2);
//! };
//! let global_mask: MaskObject = mask_aggregator.into();
//!
//! // aggregate the local masked models
//! let mut model_aggregator = Aggregation::new(config.into(), number_weights);
//! if let Ok(_) = model_aggregator.validate_aggregation(&masked_local_model_1) {
//!     model_aggregator.aggregate(masked_local_model_1);
//! };
//! if let Ok(_) = model_aggregator.validate_aggregation(&masked_local_model_2) {
//!     model_aggregator.aggregate(masked_local_model_2);
//! };
//! ```
//!
//! ## Unmasking
//! A masked model can be unmasked by the corresponding mask via an [`Aggregation`]. Unmasking
//! should always be validated beforehand so that it may be safely performed wrt the chosen mask
//! configuration without possible loss of information.
//!
//! ```no_run
//! # use xaynet_core::mask::{Aggregation, BoundType, DataType, FromPrimitives, GroupType, MaskConfig, Masker, MaskObject, Model, ModelType, Scalar};
//! # let number_weights = 10;
//! # let scalar = Scalar::new(1, 2_u8);
//! # let local_model_1 = Model::from_primitives_bounded(vec![0_f32; number_weights].into_iter());
//! # let local_model_2 = Model::from_primitives_bounded(vec![1_f32; number_weights].into_iter());
//! # let config = MaskConfig { group_type: GroupType::Prime, data_type: DataType::F32, bound_type: BoundType::B0, model_type: ModelType::M3};
//! # let (local_mask_seed_1, masked_local_model_1) = Masker::new(config.into()).mask(scalar.clone(), &local_model_1);
//! # let (local_mask_seed_2, masked_local_model_2) = Masker::new(config.into()).mask(scalar, &local_model_2);
//! # let local_model_mask_1 = local_mask_seed_1.derive_mask(number_weights, config.into());
//! # let local_model_mask_2 = local_mask_seed_2.derive_mask(number_weights, config.into());
//! # let mut mask_aggregator = Aggregation::new(config.into(), number_weights);
//! # if let Ok(_) = mask_aggregator.validate_aggregation(&local_model_mask_1) { mask_aggregator.aggregate(local_model_mask_1); };
//! # if let Ok(_) = mask_aggregator.validate_aggregation(&local_model_mask_2) { mask_aggregator.aggregate(local_model_mask_2); };
//! # let global_mask: MaskObject = mask_aggregator.into();
//! # let mut model_aggregator = Aggregation::new(config.into(), number_weights);
//! # if let Ok(_) = model_aggregator.validate_aggregation(&masked_local_model_1) { model_aggregator.aggregate(masked_local_model_1); };
//! # if let Ok(_) = model_aggregator.validate_aggregation(&masked_local_model_2) { model_aggregator.aggregate(masked_local_model_2); };
//! // unmask the aggregated masked model with the aggregated mask
//! if let Ok(_) = model_aggregator.validate_unmasking(&global_mask) {
//!     let global_model = model_aggregator.unmask(global_mask);
//!     assert_eq!(
//!         global_model,
//!         Model::from_primitives_bounded(vec![0.5_f32; number_weights].into_iter()),
//!     );
//! };
//! ```

pub(crate) mod config;
pub(crate) mod masking;
pub(crate) mod model;
pub(crate) mod object;
pub(crate) mod scalar;
pub(crate) mod seed;

pub use self::{
    config::{
        serialization::MaskConfigBuffer,
        BoundType,
        DataType,
        GroupType,
        InvalidMaskConfigError,
        MaskConfig,
        MaskConfigPair,
        ModelType,
    },
    masking::{Aggregation, AggregationError, Masker, UnmaskingError},
    model::{FromPrimitives, IntoPrimitives, Model, ModelCastError, PrimitiveCastError},
    object::{
        serialization::vect::MaskVectBuffer,
        InvalidMaskObjectError,
        MaskObject,
        MaskUnit,
        MaskVect,
    },
    scalar::{FromPrimitive, IntoPrimitive, Scalar, ScalarCastError},
    seed::{EncryptedMaskSeed, MaskSeed},
};
