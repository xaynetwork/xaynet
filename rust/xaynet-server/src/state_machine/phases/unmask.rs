use std::{cmp::Ordering, sync::Arc};

use xaynet_core::mask::{Aggregation, MaskObject, Model};

use crate::{
    state_machine::{
        events::ModelUpdate,
        phases::{Idle, Phase, PhaseName, PhaseState, PhaseStateError, Shared},
        StateMachine,
        UnmaskGlobalModelError,
    },
    storage::api::{PersistentStorage, VolatileStorage},
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
impl<V, P> Phase<V, P> for PhaseState<Unmask, V, P>
where
    V: VolatileStorage,
    P: PersistentStorage,
{
    const NAME: PhaseName = PhaseName::Unmask;

    /// Run the unmasking phase
    async fn run(&mut self) -> Result<(), PhaseStateError> {
        #[cfg(feature = "metrics")]
        {
            let storage = self.shared.store.clone();
            let mut metrics_tx = self.shared.metrics_tx.clone();
            let (round_id, phase_name) = (self.shared.state.round_id, Self::NAME);

            tokio::spawn(async move {
                match storage.get_number_of_unique_masks().await {
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
            .store
            .get_best_masks()
            .await
            .map_err(PhaseStateError::GetSeedDict)?;

        let global_model = self.end_round(best_masks).await?;

        #[cfg(feature = "model-persistence")]
        {
            let round_seed = self.shared.state.round_params.seed.clone();
            let round_id = self.shared.state.round_id;
            let id = self
                .shared
                .store
                .set_global_model(round_id, &round_seed, &global_model)
                .await
                .map_err(PhaseStateError::SaveGlobalModel)?;
            let _ = self
                .shared
                .store
                .set_latest_global_model_id(&id)
                .await
                .map_err(|err| warn!("cannot set latest model id: {}", err));
        }

        info!("broadcasting the new global model");
        self.shared
            .events
            .broadcast_model(ModelUpdate::New(Arc::new(global_model)));

        Ok(())
    }

    /// Moves from the unmask state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine<V, P>> {
        Some(PhaseState::<Idle, _, _>::new(self.shared).into())
    }
}

impl<V, P> PhaseState<Unmask, V, P>
where
    V: VolatileStorage,
    P: PersistentStorage,
{
    /// Creates a new unmask state.
    pub fn new(shared: Shared<V, P>, model_agg: Aggregation) -> Self {
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
