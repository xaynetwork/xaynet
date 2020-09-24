use std::sync::Arc;

use crate::{
    state_machine::{
        events::DictionaryUpdate,
        phases::{Handler, Phase, PhaseName, PhaseState, Shared, StateError, Update},
        requests::{StateMachineRequest, SumRequest},
        StateMachine,
        StateMachineError,
    },
    storage::AddSumParticipant,
};

#[cfg(feature = "metrics")]
use crate::metrics;

use tokio::time::{timeout, Duration};

/// Sum state
#[derive(Debug)]
pub struct Sum {
    sum_count: usize,
}

#[async_trait]
impl Handler for PhaseState<Sum> {
    /// Handles a [`StateMachineRequest`].
    ///
    /// If the request is a [`StateMachineRequest::Update`] or
    /// [`StateMachineRequest::Sum2`] request, the request sender will receive a
    /// [`StateMachineError::MessageRejected`].
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), StateMachineError> {
        match req {
            StateMachineRequest::Sum(sum_req) => {
                metrics!(
                    self.shared.io.metrics_tx,
                    metrics::message::sum::increment(self.shared.state.round_id, Self::NAME)
                );
                self.handle_sum(sum_req).await
            }
            _ => Err(StateMachineError::MessageRejected),
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
            self.inner.sum_count, self.shared.state.min_sum_count
        );

        let sum_dict = self
            .shared
            .io
            .redis
            .connection()
            .await
            .get_sum_dict()
            .await?;

        info!("broadcasting sum dictionary");
        self.shared
            .io
            .events
            .broadcast_sum_dict(DictionaryUpdate::New(Arc::new(sum_dict)));
        Ok(())
    }

    fn next(self) -> Option<StateMachine> {
        Some(PhaseState::<Update>::new(self.shared).into())
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
                self.inner.sum_count, self.shared.state.min_sum_count,
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
            inner: Sum { sum_count: 0 },
            shared,
        }
    }

    /// Handles a sum request.
    ///
    /// # Error
    /// Fails if the sum participant cannot be added due to a Redis error.
    async fn handle_sum(&mut self, req: SumRequest) -> Result<(), StateMachineError> {
        let SumRequest {
            participant_pk,
            ephm_pk,
        } = req;

        let result = self
            .shared
            .io
            .redis
            .connection()
            .await
            .add_sum_participant(&participant_pk, &ephm_pk)
            .await?;

        if let AddSumParticipant::Ok = result {
            self.inner.sum_count += 1;
        };

        Ok(())
    }

    /// Checks whether enough sum participants submitted their ephemeral keys to start the update
    /// phase.
    fn has_enough_sums(&self) -> bool {
        self.inner.sum_count >= self.shared.state.min_sum_count
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state_machine::{
        events::Event,
        tests::{builder::StateMachineBuilder, utils},
    };
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    pub async fn integration_sum_to_update() {
        utils::enable_logging();
        let sum = Sum { sum_count: 0 };
        let (state_machine, request_tx, events, redis) = StateMachineBuilder::new()
            .await
            .with_phase(sum)
            // Make sure anyone is a sum participant.
            .with_sum_ratio(1.0)
            .with_update_ratio(0.0)
            // Make sure a single participant is enough to go to the
            // update phase
            .with_min_sum(1)
            .with_model_size(4)
            .with_min_sum_time(1)
            .with_max_sum_time(2)
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
        let sum_msg = summer.compose_sum_message(round_params.pk);
        let request_fut = async { request_tx.msg(&sum_msg).await.unwrap() };
        let transition_fut = async { state_machine.next().await.unwrap() };

        let (_response, state_machine) = tokio::join!(request_fut, transition_fut);
        let PhaseState {
            inner: update_state,
            shared,
            ..
        } = state_machine.into_update_phase_state();

        // Check the initial state of the update phase.
        let frozen_sum_dict = redis.connection().await.get_sum_dict().await.unwrap();
        assert_eq!(frozen_sum_dict.len(), 1);
        let (pk, ephm_pk) = frozen_sum_dict.iter().next().unwrap();
        assert_eq!(pk.clone(), summer.pk);
        assert_eq!(ephm_pk.clone(), utils::ephm_pk(&sum_msg));

        let seed_dict = redis.connection().await.get_seed_dict().await.unwrap();
        assert_eq!(seed_dict.len(), 1);
        let (pk, dict) = seed_dict.iter().next().unwrap();
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
