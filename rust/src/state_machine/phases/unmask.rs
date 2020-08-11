use std::{cmp::Ordering, sync::Arc};

use crate::{
    mask::{masking::Aggregation, model::Model, object::MaskObject},
    state_machine::{
        coordinator::MaskDict,
        events::ModelUpdate,
        phases::{Idle, Phase, PhaseName, PhaseState, Shared, StateError},
        RoundFailed,
        StateMachine,
    },
};

/// Unmask state
#[derive(Debug)]
pub struct Unmask {
    /// The aggregator for masks and masked models.
    aggregation: Option<Aggregation>,

    /// The mask dictionary built during the sum2 phase.
    mask_dict: MaskDict,
}

#[cfg(test)]
impl Unmask {
    pub fn aggregation(&self) -> Option<&Aggregation> {
        self.aggregation.as_ref()
    }
    pub fn mask_dict(&self) -> &MaskDict {
        &self.mask_dict
    }
}

#[async_trait]
impl Phase for PhaseState<Unmask> {
    const NAME: PhaseName = PhaseName::Unmask;

    /// Run the unmasking phase
    async fn run(&mut self) -> Result<(), StateError> {
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
    pub fn new(shared: Shared, aggregation: Aggregation, mask_dict: MaskDict) -> Self {
        info!("state transition");
        Self {
            inner: Unmask {
                aggregation: Some(aggregation),
                mask_dict,
            },
            shared,
        }
    }

    /// Freezes the mask dictionary.
    fn freeze_mask_dict(&mut self) -> Result<MaskObject, RoundFailed> {
        if self.inner.mask_dict.is_empty() {
            return Err(RoundFailed::NoMask);
        }

        self.inner
            .mask_dict
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
            .ok_or(RoundFailed::AmbiguousMasks)
    }

    fn end_round(&mut self) -> Result<Model, RoundFailed> {
        let global_mask = self.freeze_mask_dict()?;

        // Safe unwrap: State::<Unmask>::new always creates Some(aggregation)
        let aggregation = self.inner.aggregation.take().unwrap();

        aggregation
            .validate_unmasking(&global_mask)
            .map_err(RoundFailed::from)?;
        Ok(aggregation.unmask(global_mask))
    }
}
