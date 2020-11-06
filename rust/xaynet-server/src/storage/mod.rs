pub(crate) mod impls;
pub mod redis;
#[cfg(feature = "model-persistence")]
pub mod s3;
#[cfg(test)]
pub(crate) mod tests;

pub use self::{
    impls::{
        MaskDictIncr,
        MaskDictIncrError,
        SeedDictUpdate,
        SeedDictUpdateError,
        SumDictAdd,
        SumDictAddError,
    },
    redis::{RedisError, RedisResult},
};
