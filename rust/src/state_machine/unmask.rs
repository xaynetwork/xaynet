use super::{
    idle::Idle,
    CoordinatorState,
    MaskDict,
    PhaseState,
    Request,
    StateError,
    StateMachine,
};

use crate::{
    coordinator::RoundFailed,
    mask::{Aggregation, MaskObject, Model},
};
use std::cmp::Ordering;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Unmask {
    aggregation: Option<Aggregation>,

    /// Dictionary built during the sum2 phase.
    mask_dict: MaskDict,
}

impl PhaseState<Unmask> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
        aggregation: Aggregation,
        mask_dict: MaskDict,
    ) -> Self {
        info!("state transition");
        Self {
            inner: Unmask {
                aggregation: Some(aggregation),
                mask_dict,
            },
            coordinator_state,
            request_rx,
        }
    }

    pub async fn next(mut self) -> Option<StateMachine> {
        let next_state = match self.run_phase().await {
            Ok(_) => PhaseState::<Idle>::new(self.coordinator_state, self.request_rx).into(),
            Err(err) => {
                PhaseState::<StateError>::new(self.coordinator_state, self.request_rx, err).into()
            }
        };
        Some(next_state)
    }

    async fn run_phase(&mut self) -> Result<(), StateError> {
        let _global_model = self.end_round()?;
        Ok(())
    }

    /// Freeze the mask dictionary.
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

        // safe unwrap: State::<Unmask>::new always creates Some(aggregation)
        let aggregation = self.inner.aggregation.take().unwrap();

        aggregation
            .validate_unmasking(&global_mask)
            .map_err(RoundFailed::from)?;
        Ok(aggregation.unmask(global_mask))
    }
}
