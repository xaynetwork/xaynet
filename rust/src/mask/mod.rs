//! Masking of models according to the PET protocol.

pub(crate) mod config;
mod masking;
pub(crate) mod model;
pub(crate) mod object;
mod seed;

pub use self::{
    config::{
        serialization::MaskConfigBuffer,
        BoundType,
        DataType,
        GroupType,
        InvalidMaskConfigError,
        MaskConfig,
        ModelType,
    },
    masking::{Aggregation, AggregationError, Masker, UnmaskingError},
    model::{FromPrimitives, IntoPrimitives, Model, ModelCastError, PrimitiveCastError},
    object::{serialization::MaskObjectBuffer, InvalidMaskObjectError, MaskObject},
    seed::{EncryptedMaskSeed, MaskSeed},
};
