use std::{cmp::Ordering, sync::Arc};

use xaynet_core::mask::{Aggregation, MaskMany, Model};

use crate::state_machine::{
    coordinator::MaskDict,
    events::ModelUpdate,
    phases::{Idle, Phase, PhaseName, PhaseState, Shared, StateError},
    RoundFailed, StateMachine,
};

#[cfg(feature = "metrics")]
use crate::metrics;

/// Unmask state
#[derive(Debug)]
pub struct Unmask {
    /// The aggregator for masked models.
    model_agg: Option<Aggregation>,

    /// The aggregator for masked scalars.
    scalar_agg: Option<Aggregation>,

    /// The model mask dictionary built during the sum2 phase.
    model_mask_dict: MaskDict,

    /// The scalar mask dictionary built during the sum2 phase.
    scalar_mask_dict: MaskDict,
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
    pub fn new(
        shared: Shared,
        model_agg: Aggregation,
        scalar_agg: Aggregation,
        model_mask_dict: MaskDict,
        scalar_mask_dict: MaskDict,
    ) -> Self {
        info!("state transition");
        Self {
            inner: Unmask {
                model_agg: Some(model_agg),
                scalar_agg: Some(scalar_agg),
                model_mask_dict,
                scalar_mask_dict,
            },
            shared,
        }
    }

    /// Freezes the mask dictionary.
    fn freeze_mask_dict(&mut self) -> Result<(MaskMany, MaskMany), RoundFailed> {
        if self.inner.model_mask_dict.is_empty() {
            return Err(RoundFailed::NoMask);
        }

        let model_mask = self
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

        // TODO remove duplication
        let scalar_mask = self
            .inner
            .scalar_mask_dict
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

        Ok((model_mask, scalar_mask))
    }

    fn end_round(&mut self) -> Result<Model, RoundFailed> {
        let (model_mask, scalar_mask) = self.freeze_mask_dict()?;

        // Safe unwrap: State::<Unmask>::new always creates Some(aggregation)
        let model_agg = self.inner.model_agg.take().unwrap();
        let scalar_agg = self.inner.scalar_agg.take().unwrap();

        model_agg
            .validate_unmasking(&model_mask)
            .map_err(RoundFailed::from)?;
        scalar_agg
            .validate_unmasking(&scalar_mask)
            .map_err(RoundFailed::from)?;

        let model = model_agg.unmask(model_mask);
        let scalar = scalar_agg.unmask(scalar_mask);

        Ok(Aggregation::correct(model, scalar))
    }
}
