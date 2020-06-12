use std::{cmp::Ordering, sync::Arc};

use crate::{
    mask::{Aggregation, MaskObject, Model},
    state_machine::{
        coordinator::{CoordinatorState, MaskDict},
        events::{ModelUpdate, PhaseEvent},
        phases::{Idle, Phase, PhaseState, StateError},
        requests::RequestReceiver,
        RoundFailed,
        StateMachine,
    },
};

#[derive(Debug)]
pub struct Unmask {
    aggregation: Option<Aggregation>,

    /// Dictionary built during the sum2 phase.
    mask_dict: MaskDict,
}

#[async_trait]
impl<R> Phase<R> for PhaseState<R, Unmask>
where
    R: Send,
{
    async fn next(mut self) -> Option<StateMachine<R>> {
        info!("starting unmasking phase");

        info!("broadcasting unmasking phase event");
        self.coordinator_state
            .events
            .broadcast_phase(self.coordinator_state.round_params.id, PhaseEvent::Unmask);

        let next_state = match self.run_phase().await {
            Ok(_) => {
                info!("unmasking phased completed successfully, going back to idle phase");
                PhaseState::<R, Idle>::new(self.coordinator_state, self.request_rx).into()
            }
            Err(err) => {
                error!("unmasking phase failed: {}", err);
                PhaseState::<R, StateError>::new(self.coordinator_state, self.request_rx, err)
                    .into()
            }
        };
        Some(next_state)
    }
}

impl<R> PhaseState<R, Unmask> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: RequestReceiver<R>,
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

    async fn run_phase(&mut self) -> Result<(), StateError> {
        let global_model = self.end_round()?;
        info!("broadcasting the new global model");
        self.coordinator_state.events.broadcast_model(
            self.coordinator_state.round_params.id,
            ModelUpdate::New(Arc::new(global_model)),
        );

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
