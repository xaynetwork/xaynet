//! A generic store.

use async_trait::async_trait;

use crate::{
    state_machine::coordinator::CoordinatorState,
    storage::{
        trust_anchor::noop::NoOp,
        CoordinatorStorage,
        LocalSeedDictAdd,
        MaskScoreIncr,
        ModelStorage,
        Storage,
        StorageResult,
        SumPartAdd,
        TrustAnchor,
    },
};
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

#[derive(Clone)]
/// A generic store.
pub struct Store<C, M, T>
where
    C: CoordinatorStorage,
    M: ModelStorage,
    T: TrustAnchor,
{
    /// A coordinator store.
    coordinator: C,
    /// A model store.
    model: M,
    /// A trust anchor.
    trust_anchor: T,
}

impl<C, M, T> Store<C, M, T>
where
    C: CoordinatorStorage,
    M: ModelStorage,
    T: TrustAnchor,
{
    pub fn new_with_trust_anchor(coordinator: C, model: M, trust_anchor: T) -> Self {
        Self {
            coordinator,
            model,
            trust_anchor,
        }
    }
}

impl<C, M> Store<C, M, NoOp>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Creates a new [`Store`].
    pub fn new(coordinator: C, model: M) -> Self {
        Self {
            coordinator,
            model,
            trust_anchor: NoOp,
        }
    }
}

#[async_trait]
impl<C, M, T> CoordinatorStorage for Store<C, M, T>
where
    C: CoordinatorStorage,
    M: ModelStorage,
    T: TrustAnchor,
{
    async fn set_coordinator_state(&mut self, state: &CoordinatorState) -> StorageResult<()> {
        self.coordinator.set_coordinator_state(state).await
    }

    async fn coordinator_state(&mut self) -> StorageResult<Option<CoordinatorState>> {
        self.coordinator.coordinator_state().await
    }

    async fn add_sum_participant(
        &mut self,
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
    ) -> StorageResult<SumPartAdd> {
        self.coordinator.add_sum_participant(pk, ephm_pk).await
    }

    async fn sum_dict(&mut self) -> StorageResult<Option<SumDict>> {
        self.coordinator.sum_dict().await
    }

    async fn add_local_seed_dict(
        &mut self,
        update_pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> StorageResult<LocalSeedDictAdd> {
        self.coordinator
            .add_local_seed_dict(update_pk, local_seed_dict)
            .await
    }

    async fn seed_dict(&mut self) -> StorageResult<Option<SeedDict>> {
        self.coordinator.seed_dict().await
    }

    async fn incr_mask_score(
        &mut self,
        pk: &SumParticipantPublicKey,
        mask: &MaskObject,
    ) -> StorageResult<MaskScoreIncr> {
        self.coordinator.incr_mask_score(pk, mask).await
    }

    async fn best_masks(&mut self) -> StorageResult<Option<Vec<(MaskObject, u64)>>> {
        self.coordinator.best_masks().await
    }

    async fn number_of_unique_masks(&mut self) -> StorageResult<u64> {
        self.coordinator.number_of_unique_masks().await
    }

    async fn delete_coordinator_data(&mut self) -> StorageResult<()> {
        self.coordinator.delete_coordinator_data().await
    }

    async fn delete_dicts(&mut self) -> StorageResult<()> {
        self.coordinator.delete_dicts().await
    }

    async fn set_latest_global_model_id(&mut self, id: &str) -> StorageResult<()> {
        self.coordinator.set_latest_global_model_id(id).await
    }

    async fn latest_global_model_id(&mut self) -> StorageResult<Option<String>> {
        self.coordinator.latest_global_model_id().await
    }

    async fn is_ready(&mut self) -> StorageResult<()> {
        self.coordinator.is_ready().await
    }
}

#[async_trait]
impl<C, M, T> ModelStorage for Store<C, M, T>
where
    C: CoordinatorStorage,
    M: ModelStorage,
    T: TrustAnchor,
{
    async fn set_global_model(
        &mut self,
        round_id: u64,
        round_seed: &RoundSeed,
        global_model: &Model,
    ) -> StorageResult<String> {
        self.model
            .set_global_model(round_id, round_seed, global_model)
            .await
    }

    async fn global_model(&mut self, id: &str) -> StorageResult<Option<Model>> {
        self.model.global_model(id).await
    }

    async fn is_ready(&mut self) -> StorageResult<()> {
        self.model.is_ready().await
    }
}

#[async_trait]
impl<C, M, T> TrustAnchor for Store<C, M, T>
where
    C: CoordinatorStorage,
    M: ModelStorage,
    T: TrustAnchor,
{
    async fn publish_proof(&mut self, global_model: &Model) -> StorageResult<()> {
        self.trust_anchor.publish_proof(global_model).await
    }

    async fn is_ready(&mut self) -> StorageResult<()> {
        self.trust_anchor.is_ready().await
    }
}

#[async_trait]
impl<C, M, T> Storage for Store<C, M, T>
where
    C: CoordinatorStorage,
    M: ModelStorage,
    T: TrustAnchor,
{
    async fn is_ready(&mut self) -> StorageResult<()> {
        tokio::try_join!(
            self.coordinator.is_ready(),
            self.model.is_ready(),
            self.trust_anchor.is_ready()
        )
        .map(|_| ())
    }
}
