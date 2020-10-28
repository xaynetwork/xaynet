use std::{cmp::Ordering, sync::Arc};

use xaynet_core::mask::{Aggregation, MaskObject, Model};

use crate::state_machine::{
    events::ModelUpdate,
    phases::{Idle, Phase, PhaseName, PhaseState, PhaseStateError, Shared},
    StateMachine,
    UnmaskGlobalModelError,
};

#[cfg(feature = "metrics")]
use crate::metrics;

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
        {
            let redis = self.shared.io.redis.clone();
            let mut metrics_tx = self.shared.io.metrics_tx.clone();
            let (round_id, phase_name) = (self.shared.state.round_id, Self::NAME);

            tokio::spawn(async move {
                match redis.connection().await.get_number_of_unique_masks().await {
                    Ok(number_of_masks) => metrics_tx.send(metrics::masks::total_number::update(
                        number_of_masks,
                        round_id,
                        phase_name,
                    )),
                    Err(err) => error!("failed to fetch total number of masks: {}", err),
                };
            });
        }

        let best_masks = self
            .shared
            .io
            .redis
            .connection()
            .await
            .get_best_masks()
            .await?;

        let global_model = self.end_round(best_masks).await?;

        #[cfg(feature = "model-persistence")]
        {
            // As key for the global model we use the round_id and the seed
            // (format: `roundid_roundseed`) of the round in which the global model was created.
            use xaynet_core::crypto::ByteObject;
            let round_seed = hex::encode(self.shared.state.round_params.seed.as_slice());
            let key = format!("{}_{}", self.shared.state.round_id, round_seed);
            self.shared
                .io
                .s3
                .upload_global_model(&key, &global_model)
                .await
                .map_err(PhaseStateError::SaveGlobalModel)?;
            let _ = self
                .shared
                .io
                .redis
                .connection()
                .await
                .set_latest_global_model_id(&key)
                .await
                .map_err(|err| warn!("failed to update latest global model id: {}", err));
        }

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
        if best_masks.is_empty() {
            return Err(UnmaskGlobalModelError::NoMask);
        }

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
}
