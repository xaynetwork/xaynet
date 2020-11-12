use crate::storage::ModelStorage;
use crate::storage::StorageResult;
use async_trait::async_trait;
use xaynet_core::common::RoundSeed;
use xaynet_core::mask::Model;

#[derive(Clone)]
struct NoOp;

#[async_trait]
impl ModelStorage for NoOp {
    async fn set_global_model(
        &mut self,
        round_id: u64,
        round_seed: &RoundSeed,
        _global_model: &Model,
    ) -> StorageResult<String> {
        Ok(Self::create_global_model_id(round_id, round_seed))
    }

    async fn global_model(&mut self, _id: &str) -> StorageResult<Option<Model>> {
        Err(anyhow::anyhow!("No-op model store"))
    }
}
