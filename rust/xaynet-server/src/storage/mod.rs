pub(crate) mod impls;
pub mod redis;

pub use self::{
    impls::{
        MaskDictIncr,
        MaskDictIncrError,
        SeedDictUpdate,
        SeedDictUpdateError,
        SumDictAdd,
        SumDictAddError,
    },
    redis::RedisError,
};

#[cfg(test)]
pub(crate) mod tests;
