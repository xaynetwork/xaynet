use std::{cmp::Ordering, sync::Arc};

use xaynet_core::mask::{Aggregation, MaskObject, Model};

use crate::state_machine::{
    coordinator::MaskDict,
    events::ModelUpdate,
    phases::{Idle, Phase, PhaseName, PhaseState, Shared, StateError},
    RoundFailed,
    StateMachine,
};

#[cfg(feature = "metrics")]
use crate::metrics;

/// Unmask state
#[derive(Debug)]
pub struct Unmask {
    /// The aggregator for masked models.
    model_agg: Option<Aggregation>,

    /// The model mask dictionary built during the sum2 phase.
    model_mask_dict: MaskDict,
}

#[cfg(test)]
impl Unmask {
    pub fn aggregation(&self) -> Option<&Aggregation> {
        self.model_agg.as_ref()
    }
    pub fn mask_dict(&self) -> &MaskDict {
        &self.model_mask_dict
    }
}

#[async_trait]
impl Phase for PhaseState<Unmask> {
    const NAME: PhaseName = PhaseName::Unmask;

    /// Run the unmasking phase
    async fn run(&mut self) -> Result<(), StateError> {
        metrics!(
            self.shared.io.metrics_tx,
            metrics::masks::total_number::update(
                self.inner.model_mask_dict.len(),
                self.shared.state.round_id,
                Self::NAME
            )
        );

        let global_model = self.end_round()?;

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
        info!("going back to idle phase");
        Some(PhaseState::<Idle>::new(self.shared).into())
    }
}

impl PhaseState<Unmask> {
    /// Creates a new unmask state.
    pub fn new(shared: Shared, model_agg: Aggregation, model_mask_dict: MaskDict) -> Self {
        info!("state transition");
        Self {
            inner: Unmask {
                model_agg: Some(model_agg),
                model_mask_dict,
            },
            shared,
        }
    }

    /// Freezes the mask dictionary.
    fn freeze_mask_dict(&mut self) -> Result<MaskObject, RoundFailed> {
        if self.inner.model_mask_dict.is_empty() {
            return Err(RoundFailed::NoMask);
        }

        let mask = self
            .inner
            .model_mask_dict
            .drain()
            .fold(
                (None, 0_usize),
                |(unique_mask, unique_count), (mask, count)| match unique_count.cmp(&count) {
                    Ordering::Less => (Some(mask), count),
                    Ordering::Greater => (unique_mask, unique_count),
                    Ordering::Equal => (None, unique_count),
                },
            )
            .0
            .ok_or(RoundFailed::AmbiguousMasks)?;

        Ok(mask)
    }

    fn end_round(&mut self) -> Result<Model, RoundFailed> {
        let mask = self.freeze_mask_dict()?;

        // Safe unwrap: State::<Unmask>::new always creates Some(aggregation)
        let model_agg = self.inner.model_agg.take().unwrap();

        model_agg
            .validate_unmasking(&mask)
            .map_err(RoundFailed::from)?;

        Ok(model_agg.unmask(mask))
    }
}
