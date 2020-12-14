//! Storage backends for the coordinator.

pub mod coordinator_storage;
pub mod model_storage;
pub mod store;
#[cfg(test)]
pub(crate) mod tests;
pub mod traits;
pub mod trust_anchor;

pub use self::{
    store::Store,
    traits::{
        CoordinatorStorage,
        LocalSeedDictAdd,
        LocalSeedDictAddError,
        MaskScoreIncr,
        MaskScoreIncrError,
        ModelStorage,
        Storage,
        StorageError,
        StorageResult,
        SumPartAdd,
        SumPartAddError,
        TrustAnchor,
    },
};
