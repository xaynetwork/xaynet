use std::sync::Arc;

use xaynet_core::{LocalSeedDict, PetError, SeedDict, SumDict};

use crate::state_machine::{
    events::DictionaryUpdate,
    phases::{Handler, Phase, PhaseName, PhaseState, Shared, StateError, Update},
    requests::{StateMachineRequest, SumRequest},
    StateMachine,
};

#[cfg(feature = "metrics")]
use crate::metrics;

use tokio::time::{timeout, Duration};

/// Sum state
#[derive(Debug)]
pub struct Sum {
    /// Dictionary built during the sum phase.
    sum_dict: SumDict,
    /// Seed dictionary built at the end of the sum phase.
    seed_dict: Option<SeedDict>,
}

#[cfg(test)]
impl Sum {
    pub fn sum_dict(&self) -> &SumDict {
        &self.sum_dict
    }
}

impl Handler for PhaseState<Sum> {
    /// Handles a [`StateMachineRequest`].
    ///
    /// If the request is a [`StateMachineRequest::Update`] or
    /// [`StateMachineRequest::Sum2`] request, the request sender will receive a
    /// [`PetError::InvalidMessage`].
    fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), PetError> {
        match req {
            StateMachineRequest::Sum(sum_req) => {
                metrics!(
                    self.shared.io.metrics_tx,
                    metrics::message::sum::increment(self.shared.state.round_id, Self::NAME)
                );
                self.handle_sum(sum_req)
            }
            _ => Err(PetError::InvalidMessage),
        }
    }
}

#[async_trait]
impl Phase for PhaseState<Sum>
where
    Self: Handler,
{
    const NAME: PhaseName = PhaseName::Sum;

    /// Run the sum phase.
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn run(&mut self) -> Result<(), StateError> {
        let min_time = self.shared.state.min_sum_time;
        debug!("in sum phase for a minimum of {} seconds", min_time);
        self.process_during(Duration::from_secs(min_time)).await?;

        let time_left = self.shared.state.max_sum_time - min_time;
        timeout(Duration::from_secs(time_left), self.process_until_enough()).await??;

        info!(
            "{} sum messages handled (min {} required)",
            self.inner.sum_dict.len(),
            self.shared.state.min_sum_count
        );
        self.freeze_sum_dict();
        Ok(())
    }

    fn next(self) -> Option<StateMachine> {
        let Self {
            inner: Sum {
                sum_dict,
                seed_dict,
            },
            shared,
        } = self;

        Some(
            PhaseState::<Update>::new(
                shared,
                sum_dict,
                // `next()` is called at the end of the sum phase, at
                // which point a new seed dictionary has been build,
                // so there's something to unwrap here.
                seed_dict.unwrap(),
            )
            .into(),
        )
    }
}

impl PhaseState<Sum>
where
    Self: Handler + Phase,
{
    /// Processes requests until there are enough.
    async fn process_until_enough(&mut self) -> Result<(), StateError> {
        while !self.has_enough_sums() {
            debug!(
                "{} sum messages handled (min {} required)",
                self.inner.sum_dict.len(),
                self.shared.state.min_sum_count,
            );
            self.process_single().await?;
        }
        Ok(())
    }
}

impl PhaseState<Sum> {
    /// Creates a new sum state.
    pub fn new(shared: Shared) -> Self {
        info!("state transition");
        Self {
            inner: Sum {
                sum_dict: SumDict::new(),
                seed_dict: None,
            },
            shared,
        }
    }

    /// Handles a sum request.
    fn handle_sum(&mut self, req: SumRequest) -> Result<(), PetError> {
        let SumRequest {
            participant_pk,
            ephm_pk,
        } = req;
        self.inner.sum_dict.insert(participant_pk, ephm_pk);
        Ok(())
    }

    /// Freezes the sum dictionary.
    fn freeze_sum_dict(&mut self) {
        info!("broadcasting sum dictionary");
        self.shared
            .io
            .events
            .broadcast_sum_dict(DictionaryUpdate::New(Arc::new(self.inner.sum_dict.clone())));

        info!("initializing seed dictionary");
        self.inner.seed_dict = Some(
            self.inner
                .sum_dict
                .keys()
                .map(|pk| (*pk, LocalSeedDict::new()))
                .collect(),
        )
    }

    /// Checks whether enough sum participants submitted their ephemeral keys to start the update
    /// phase.
    fn has_enough_sums(&self) -> bool {
        self.inner.sum_dict.len() >= self.shared.state.min_sum_count
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state_machine::{
        events::Event,
        tests::{builder::StateMachineBuilder, utils},
    };
    use xaynet_core::SumDict;

    #[tokio::test]
    pub async fn sum_to_update() {
        let sum = Sum {
            sum_dict: SumDict::new(),
            seed_dict: None,
        };
        let (state_machine, request_tx, events) = StateMachineBuilder::new()
            .with_phase(sum)
            // Make sure anyone is a sum participant.
            .with_sum_ratio(1.0)
            .with_update_ratio(0.0)
            // Make sure a single participant is enough to go to the
            // update phase
            .with_min_sum(1)
            .with_model_size(4)
            .build();
        assert!(state_machine.is_sum());

        let round_params = events.params_listener().get_latest().event;
        let seed = round_params.seed.clone();
        let keys = events.keys_listener().get_latest().event;

        // Send a sum request and attempt to transition. The
        // coordinator is configured to consider any sum request as
        // eligible, so after processing it, we should go to the
        // update phase
        let mut summer = utils::generate_summer(&seed, 1.0, 0.0);
        let sum_msg = summer.compose_sum_message();
        let request_fut = async { request_tx.msg(&sum_msg).await.unwrap() };
        let transition_fut = async { state_machine.next().await.unwrap() };

        let (_response, state_machine) = tokio::join!(request_fut, transition_fut);
        let PhaseState {
            inner: update_state,
            shared,
            ..
        } = state_machine.into_update_phase_state();

        // Check the initial state of the update phase.
        assert_eq!(update_state.frozen_sum_dict().len(), 1);
        let (pk, ephm_pk) = update_state.frozen_sum_dict().iter().next().unwrap();
        assert_eq!(pk.clone(), summer.pk);
        assert_eq!(ephm_pk.clone(), utils::ephm_pk(&sum_msg));

        assert_eq!(update_state.seed_dict().len(), 1);
        let (pk, dict) = update_state.seed_dict().iter().next().unwrap();
        assert_eq!(pk.clone(), summer.pk);
        assert!(dict.is_empty());

        assert_eq!(update_state.aggregation().len(), 4);

        // Make sure that the round seed and parameters are unchanged
        assert_eq!(seed, shared.state.round_params.seed);
        assert_eq!(round_params, shared.state.round_params);
        assert_eq!(keys, shared.state.keys);

        // Check all the events that should be emitted during the sum
        // phase
        assert_eq!(
            events.phase_listener().get_latest(),
            Event {
                round_id: 0,
                event: PhaseName::Sum,
            }
        );
    }
}
