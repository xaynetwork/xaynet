use crate::storage::traits::{StorageResult, TrustAnchor};
use async_trait::async_trait;
use xaynet_core::mask::Model;

#[derive(Clone)]
pub struct NoOp;

#[async_trait]
impl TrustAnchor for NoOp {
    async fn publish_proof(&mut self, _global_model: &Model) -> StorageResult<()> {
        Ok(())
    }

    async fn is_ready(&mut self) -> StorageResult<()> {
        Ok(())
    }
}
