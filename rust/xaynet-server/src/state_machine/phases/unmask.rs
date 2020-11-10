use std::{cmp::Ordering, sync::Arc};

use async_trait::async_trait;
#[cfg(feature = "metrics")]
use tracing::error;
use tracing::info;
#[cfg(feature = "model-persistence")]
use tracing::warn;

#[cfg(feature = "metrics")]
use crate::metrics;
use crate::state_machine::{
    events::ModelUpdate,
    phases::{Idle, Phase, PhaseName, PhaseState, PhaseStateError, Shared},
    StateMachine, UnmaskGlobalModelError,
};
use crate::storage::CoordinatorStorage;
use xaynet_core::mask::{Aggregation, MaskObject, Model};

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
impl Phase for PhaseState<Unmask> {
    const NAME: PhaseName = PhaseName::Unmask;

    /// Run the unmasking phase
    async fn run(&mut self) -> Result<(), PhaseStateError> {
        #[cfg(feature = "metrics")]
        self.emit_number_of_unique_masks_metrics();

        let best_masks = self
            .shared
            .io
            .redis
            .best_masks()
            .await?
            .ok_or(UnmaskGlobalModelError::NoMask)?;

        let global_model = self.end_round(best_masks).await?;

        #[cfg(feature = "model-persistence")]
        self.save_global_model(&global_model).await?;

        info!("broadcasting the new global model");
        self.shared
            .io
            .events
            .broadcast_model(ModelUpdate::New(Arc::new(global_model)));

        Ok(())
    }

    /// Moves from the unmask state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine> {
        Some(PhaseState::<Idle>::new(self.shared).into())
    }
}

impl PhaseState<Unmask>
where
    Self: Phase,
{
    fn emit_number_of_unique_masks_metrics(&mut self) {
        let mut redis = self.shared.io.redis.clone();
        let mut metrics_tx = self.shared.io.metrics_tx.clone();
        let (round_id, phase_name) = (self.shared.state.round_id, Self::NAME);

        tokio::spawn(async move {
            match redis.number_of_unique_masks().await {
                Ok(number_of_masks) => metrics_tx.send(metrics::masks::total_number::update(
                    number_of_masks,
                    round_id,
                    phase_name,
                )),
                Err(err) => error!("failed to fetch total number of masks: {}", err),
            };
        });
    }
}

impl PhaseState<Unmask> {
    /// Creates a new unmask state.
    pub fn new(shared: Shared, model_agg: Aggregation) -> Self {
        Self {
            inner: Unmask {
                model_agg: Some(model_agg),
            },
            shared,
        }
    }

    /// Freezes the mask dictionary.
    async fn freeze_mask_dict(
        &mut self,
        mut best_masks: Vec<(MaskObject, u64)>,
    ) -> Result<MaskObject, UnmaskGlobalModelError> {
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
            .ok_or(UnmaskGlobalModelError::AmbiguousMasks)?;

        Ok(mask)
    }

    async fn end_round(
        &mut self,
        best_masks: Vec<(MaskObject, u64)>,
    ) -> Result<Model, UnmaskGlobalModelError> {
        let mask = self.freeze_mask_dict(best_masks).await?;

        // Safe unwrap: State::<Unmask>::new always creates Some(aggregation)
        let model_agg = self.inner.model_agg.take().unwrap();

        model_agg
            .validate_unmasking(&mask)
            .map_err(UnmaskGlobalModelError::from)?;

        Ok(model_agg.unmask(mask))
    }

    #[cfg(feature = "model-persistence")]
    async fn save_global_model(&mut self, global_model: &Model) -> Result<(), PhaseStateError> {
        use crate::storage::ModelStorage;

        let round_seed = &self.shared.state.round_params.seed;
        let global_model_id = self
            .shared
            .io
            .s3
            .set_global_model(self.shared.state.round_id, &round_seed, global_model)
            .await
            .map_err(PhaseStateError::SaveGlobalModel)?;
        let _ = self
            .shared
            .io
            .redis
            .set_latest_global_model_id(&global_model_id)
            .await
            .map_err(|err| warn!("failed to update latest global model id: {}", err));
        Ok(())
    }
}
