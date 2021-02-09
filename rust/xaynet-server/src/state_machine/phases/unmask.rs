use std::{cmp::Ordering, sync::Arc};

use async_trait::async_trait;
use thiserror::Error;
#[cfg(feature = "model-persistence")]
use tracing::warn;
use tracing::{error, info};

use crate::{
    metric,
    metrics::{GlobalRecorder, Measurement},
    state_machine::{
        events::ModelUpdate,
        phases::{Idle, Phase, PhaseError, PhaseName, PhaseState, Shared},
        StateMachine,
    },
    storage::{Storage, StorageError},
};
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
    #[error("publishing the proof of the global model failed: {0}")]
    PublishProof(crate::storage::StorageError),
}

/// Unmask state
#[derive(Debug)]
pub struct Unmask {
    /// The aggregator for masked models.
    model_agg: Option<Aggregation>,
    /// The global model of the current round.
    global_model: Option<Arc<Model>>,
}

#[async_trait]
impl<T> Phase<T> for PhaseState<Unmask, T>
where
    T: Storage,
{
    const NAME: PhaseName = PhaseName::Unmask;

    async fn process(&mut self) -> Result<(), PhaseError> {
        self.emit_number_of_unique_masks_metrics();
        let best_masks = self.best_masks().await?;
        self.end_round(best_masks).await?;

        #[cfg(feature = "model-persistence")]
        self.save_global_model().await?;
        self.publish_proof().await?;

        Ok(())
    }

    fn broadcast(&mut self) {
        info!("broadcasting the new global model");
        let global_model =
            self.private.global_model.take().expect(
                "unreachable: never fails when `broadcast()` is called after `end_round()`",
            );
        self.shared
            .events
            .broadcast_model(ModelUpdate::New(global_model));
    }

    async fn next(self) -> Option<StateMachine<T>> {
        Some(PhaseState::<Idle, _>::new(self.shared).into())
    }
}

impl<T> PhaseState<Unmask, T> {
    /// Creates a new unmask state.
    pub fn new(shared: Shared<T>, model_agg: Aggregation) -> Self {
        Self {
            private: Unmask {
                model_agg: Some(model_agg),
                global_model: None,
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
    ) -> Result<(), UnmaskStateError> {
        let mask = self.freeze_mask_dict(best_masks).await?;

        // Safe unwrap: State::<Unmask>::new always creates Some(aggregation)
        let model_agg = self.private.model_agg.take().unwrap();

        model_agg
            .validate_unmasking(&mask)
            .map_err(UnmaskStateError::from)?;
        self.private.global_model = Some(Arc::new(model_agg.unmask(mask)));

        Ok(())
    }
}

impl<T> PhaseState<Unmask, T>
where
    T: Storage,
{
    fn emit_number_of_unique_masks_metrics(&mut self) {
        if GlobalRecorder::global().is_none() {
            return;
        }

        let mut store = self.shared.store.clone();
        let (round_id, phase_name) = (self.shared.state.round_id, Self::NAME);

        tokio::spawn(async move {
            match store.number_of_unique_masks().await {
                Ok(number_of_masks) => metric!(
                    Measurement::MasksTotalNumber,
                    number_of_masks,
                    ("round_id", round_id),
                    ("phase", phase_name as u8),
                ),
                Err(err) => error!("failed to fetch total number of masks: {}", err),
            };
        });
    }

    async fn best_masks(&mut self) -> Result<Vec<(MaskObject, u64)>, UnmaskStateError> {
        self.shared
            .store
            .best_masks()
            .await
            .map_err(UnmaskStateError::FetchBestMasks)?
            .ok_or(UnmaskStateError::NoMask)
    }

    #[cfg(feature = "model-persistence")]
    async fn save_global_model(&mut self) -> Result<(), UnmaskStateError> {
        info!("saving global model");
        let global_model = self
            .private
            .global_model
            .as_ref()
            .expect(
                "unreachable: never fails when `save_global_model()` is called after `end_round()`",
            )
            .as_ref();
        let global_model_id = self
            .shared
            .store
            .set_global_model(
                self.shared.state.round_id,
                &self.shared.state.round_params.seed,
                global_model,
            )
            .await
            .map_err(UnmaskStateError::SaveGlobalModel)?;
        if let Err(err) = self
            .shared
            .store
            .set_latest_global_model_id(&global_model_id)
            .await
        {
            warn!("failed to update latest global model id: {}", err);
        }

        Ok(())
    }

    async fn publish_proof(&mut self) -> Result<(), UnmaskStateError> {
        info!("publishing proof of the new global model");
        let global_model = self
            .private
            .global_model
            .as_ref()
            .expect(
                "unreachable: never fails when `save_global_model()` is called after `end_round()`",
            )
            .as_ref();
        self.shared
            .store
            .publish_proof(global_model)
            .await
            .map_err(UnmaskStateError::PublishProof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Unmask {
        pub fn aggregation(&self) -> Option<&Aggregation> {
            self.model_agg.as_ref()
        }
    }
}
