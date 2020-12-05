use std::{cmp::Ordering, sync::Arc};

use async_trait::async_trait;
use tracing::info;

use crate::{
    metric,
    metrics::{GlobalRecorder, Measurement},
    state_machine::{
        events::ModelUpdate,
        phases::{Idle, Phase, PhaseName, PhaseState, PhaseStateError, Shared},
        StateMachine,
    },
    storage::{CoordinatorStorage, ModelStorage, StorageError},
};
use thiserror::Error;
use xaynet_core::mask::{Aggregation, MaskObject, Model, UnmaskingError};

/// Error that occurs during the unmask phase.
#[derive(Error, Debug)]
pub enum UnmaskStateError {
    #[error("ambiguous masks were computed by the sum participants")]
    AmbiguousMasks,
    #[error("no mask found")]
    NoMask,
    #[error("unmasking global model failed: {0}")]
    Unmasking(#[from] UnmaskingError),
    #[error("fetching best masks failed: {0}")]
    FetchBestMasks(#[from] StorageError),
    #[cfg(feature = "model-persistence")]
    #[error("saving the global model failed: {0}")]
    SaveGlobalModel(crate::storage::StorageError),
}

/// Unmask state
#[derive(Debug)]
pub struct Unmask {
    /// The aggregator for masked models.
    model_agg: Option<Aggregation>,
}

#[cfg(test)]
impl Unmask {
    pub fn aggregation(&self) -> Option<&Aggregation> {
        self.model_agg.as_ref()
    }
}

#[async_trait]
impl<C, M> Phase<C, M> for PhaseState<Unmask, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    const NAME: PhaseName = PhaseName::Unmask;

    /// Run the unmasking phase
    async fn run(&mut self) -> Result<(), PhaseStateError> {
        self.emit_number_of_unique_masks_metrics();

        let best_masks = self
            .shared
            .store
            .best_masks()
            .await
            .map_err(UnmaskStateError::FetchBestMasks)?
            .ok_or(UnmaskStateError::NoMask)?;

        let global_model = self.end_round(best_masks).await?;

        #[cfg(feature = "model-persistence")]
        self.save_global_model(&global_model).await?;

        info!("broadcasting the new global model");
        self.shared
            .events
            .broadcast_model(ModelUpdate::New(Arc::new(global_model)));

        Ok(())
    }

    /// Moves from the unmask state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine<C, M>> {
        Some(PhaseState::<Idle, _, _>::new(self.shared).into())
    }
}

impl<C, M> PhaseState<Unmask, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Creates a new unmask state.
    pub fn new(shared: Shared<C, M>, model_agg: Aggregation) -> Self {
        Self {
            private: Unmask {
                model_agg: Some(model_agg),
            },
            shared,
        }
    }

    /// Freezes the mask dictionary.
    async fn freeze_mask_dict(
        &mut self,
        mut best_masks: Vec<(MaskObject, u64)>,
    ) -> Result<MaskObject, UnmaskStateError> {
        let mask = best_masks
            .drain(0..)
            .fold(
                (None, 0),
                |(unique_mask, unique_count), (mask, count)| match unique_count.cmp(&count) {
                    Ordering::Less => (Some(mask), count),
                    Ordering::Greater => (unique_mask, unique_count),
                    Ordering::Equal => (None, unique_count),
                },
            )
            .0
            .ok_or(UnmaskStateError::AmbiguousMasks)?;

        Ok(mask)
    }

    async fn end_round(
        &mut self,
        best_masks: Vec<(MaskObject, u64)>,
    ) -> Result<Model, UnmaskStateError> {
        let mask = self.freeze_mask_dict(best_masks).await?;

        // Safe unwrap: State::<Unmask>::new always creates Some(aggregation)
        let model_agg = self.private.model_agg.take().unwrap();

        model_agg
            .validate_unmasking(&mask)
            .map_err(UnmaskStateError::from)?;

        Ok(model_agg.unmask(mask))
    }

    #[cfg(feature = "model-persistence")]
    async fn save_global_model(&mut self, global_model: &Model) -> Result<(), UnmaskStateError> {
        use tracing::warn;

        let round_seed = &self.shared.state.round_params.seed;
        let global_model_id = self
            .shared
            .store
            .set_global_model(self.shared.state.round_id, &round_seed, global_model)
            .await
            .map_err(UnmaskStateError::SaveGlobalModel)?;
        let _ = self
            .shared
            .store
            .set_latest_global_model_id(&global_model_id)
            .await
            .map_err(|err| warn!("failed to update latest global model id: {}", err));
        Ok(())
    }
}

impl<C, M> PhaseState<Unmask, C, M>
where
    Self: Phase<C, M>,
    C: CoordinatorStorage,
    M: ModelStorage,
{
    fn emit_number_of_unique_masks_metrics(&mut self) {
        use tracing::error;

        if GlobalRecorder::global().is_some() {
            let mut store = self.shared.store.clone();
            let (round_id, phase_name) = (self.shared.state.round_id, Self::NAME);

            tokio::spawn(async move {
                match store.number_of_unique_masks().await {
                    Ok(number_of_masks) => metric!(
                        Measurement::MasksTotalNumber,
                        number_of_masks,
                        ("round_id", round_id),
                        ("phase", phase_name as u8)
                    ),
                    Err(err) => error!("failed to fetch total number of masks: {}", err),
                };
            });
        }
    }
}
