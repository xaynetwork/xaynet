use crate::{
    state_machine::coordinator::CoordinatorState,
    storage::{
        coordinator_storage::redis,
        model_storage,
        CoordinatorStorage,
        LocalSeedDictAdd,
        MaskScoreIncr,
        ModelStorage,
        Storage,
        StorageResult,
        Store,
        SumPartAdd,
        TrustAnchor,
    },
};
use async_trait::async_trait;
use mockall::*;
use xaynet_core::{
    common::RoundSeed,
    mask::{MaskObject, Model},
    LocalSeedDict,
    SeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

pub mod utils;

pub async fn init_store() -> impl Storage {
    let coordinator_store = redis::tests::init_client().await;

    let model_store = {
        #[cfg(not(feature = "model-persistence"))]
        {
            model_storage::noop::NoOp
        }

        #[cfg(feature = "model-persistence")]
        {
            model_storage::s3::tests::init_client().await
        }
    };

    Store::new(coordinator_store, model_store)
}

mock! {
    pub CoordinatorStore {}

    #[async_trait]
    impl CoordinatorStorage for CoordinatorStore {
        async fn set_coordinator_state(&mut self, state: &CoordinatorState) -> StorageResult<()>;
        async fn coordinator_state(&mut self) -> StorageResult<Option<CoordinatorState>>;
        async fn add_sum_participant(
            &mut self,
            pk: &SumParticipantPublicKey,
            ephm_pk: &SumParticipantEphemeralPublicKey,
        ) -> StorageResult<SumPartAdd>;
        async fn sum_dict(&mut self) -> StorageResult<Option<SumDict>>;
        async fn add_local_seed_dict(
            &mut self,
            update_pk: &UpdateParticipantPublicKey,
            local_seed_dict: &LocalSeedDict,
        ) -> StorageResult<LocalSeedDictAdd>;
        async fn seed_dict(&mut self) -> StorageResult<Option<SeedDict>>;
        async fn incr_mask_score(
            &mut self,
            pk: &SumParticipantPublicKey,
            mask: &MaskObject,
        ) -> StorageResult<MaskScoreIncr>;
        async fn best_masks(&mut self) -> StorageResult<Option<Vec<(MaskObject, u64)>>>;
        async fn number_of_unique_masks(&mut self) -> StorageResult<u64>;
        async fn delete_coordinator_data(&mut self) -> StorageResult<()>;
        async fn delete_dicts(&mut self) -> StorageResult<()>;
        async fn set_latest_global_model_id(&mut self, id: &str) -> StorageResult<()>;
        async fn latest_global_model_id(&mut self) -> StorageResult<Option<String>>;
        async fn is_ready(&mut self) -> StorageResult<()>;
    }

    impl Clone for CoordinatorStore {
        fn clone(&self) -> Self;
    }
}

mock! {
    pub ModelStore {}

    #[async_trait]
    impl ModelStorage for ModelStore {
        async fn set_global_model(
            &mut self,
            round_id: u64,
            round_seed: &RoundSeed,
            global_model: &Model,
        ) -> StorageResult<String>;
        async fn global_model(&mut self, id: &str) -> StorageResult<Option<Model>>;
        async fn is_ready(&mut self) -> StorageResult<()>;
    }

    impl Clone for ModelStore {
        fn clone(&self) -> Self;
    }

}

mock! {
    pub TrustAnchor {}

    #[async_trait]
    impl TrustAnchor for TrustAnchor {
        async fn publish_proof(&mut self, global_model: &Model) -> StorageResult<()>;
        async fn is_ready(&mut self) -> StorageResult<()>;
    }

    impl Clone for TAnchor {
        fn clone(&self) -> Self;
    }
}
