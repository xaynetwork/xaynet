pub(crate) mod impls;
pub mod redis;
#[cfg(feature = "model-persistence")]
pub mod s3;

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

#[cfg(test)]
pub(crate) mod tests;
