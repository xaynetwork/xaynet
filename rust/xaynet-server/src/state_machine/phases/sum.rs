use std::sync::Arc;

use async_trait::async_trait;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

use crate::{
    metric,
    metrics::Measurement,
    state_machine::{
        events::DictionaryUpdate,
        phases::{Handler, Phase, PhaseName, PhaseState, PhaseStateError, Shared, Update},
        requests::{StateMachineRequest, SumRequest},
        RequestError, StateMachine,
    },
    storage::{CoordinatorStorage, ModelStorage, StorageError},
};
use thiserror::Error;

/// Error that occurs during the sum phase.
#[derive(Error, Debug)]
pub enum SumStateError {
    #[error("sum dictionary does not exists")]
    NoSumDict,
    #[error("fetching sum dictionary failed: {0}")]
    FetchSumDict(StorageError),
}

/// Sum state
#[derive(Debug, Default)]
pub struct Sum {
    /// The number of Sum messages successfully processed.
    accepted: u64,
    /// The number of Sum messages failed to processed.
    rejected: u64,
    /// The number of Sum messages discarded without being processed.
    discarded: u64,
}

#[async_trait]
impl<C, M> Handler for PhaseState<Sum, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Handles a sum request.
    ///
    /// # Errors
    /// Fails if the sum participant cannot be added due to a PET or [`StorageError`].
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError> {
        // discard if `max_sum_count` is reached
        if self.has_overmuch_sums() {
            self.private.discarded += 1;
            self.increment_message_metric(Measurement::MessageDiscarded);
            return Ok(());
        }

        // reject if not a `sum` message
        let SumRequest {
            participant_pk,
            ephm_pk,
        } = if let StateMachineRequest::Sum(req) = req {
            req
        } else {
            self.private.rejected += 1;
            self.increment_message_metric(Measurement::MessageRejected);
            return Err(RequestError::MessageRejected);
        };

        // accept if processed successfully, otherwise reject
        self.shared
            .store
            .add_sum_participant(&participant_pk, &ephm_pk)
            .await?
            .into_inner()
            .map(|ok| {
                self.private.accepted += 1;
                self.increment_message_metric(Measurement::MessageSum);
                ok
            })
            .map_err(|error| {
                self.private.rejected += 1;
                self.increment_message_metric(Measurement::MessageRejected);
                error.into()
            })
    }
}

#[async_trait]
impl<C, M> Phase<C, M> for PhaseState<Sum, C, M>
where
    Self: Handler,
    C: CoordinatorStorage,
    M: ModelStorage,
{
    const NAME: PhaseName = PhaseName::Sum;

    /// Run the sum phase.
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn run(&mut self) -> Result<(), PhaseStateError> {
        let min_time = self.shared.state.min_sum_time;
        let max_time = self.shared.state.max_sum_time;
        debug!(
            "in sum phase for a min {} and max {} seconds",
            min_time, max_time,
        );
        self.process_during(Duration::from_secs(min_time)).await?;

        let time_left = max_time - min_time;
        timeout(Duration::from_secs(time_left), self.process_until_enough()).await??;

        info!(
            "{} sum messages successfully handled (min {} and max {} required)",
            self.private.accepted, self.shared.state.min_sum_count, self.shared.state.max_sum_count,
        );
        info!("{} sum messages rejected", self.private.rejected);
        info!("{} sum messages discarded", self.private.discarded);

        let sum_dict = self
            .shared
            .store
            .sum_dict()
            .await
            .map_err(SumStateError::FetchSumDict)?
            .ok_or(SumStateError::NoSumDict)?;

        info!("broadcasting sum dictionary");
        self.shared
            .events
            .broadcast_sum_dict(DictionaryUpdate::New(Arc::new(sum_dict)));
        Ok(())
    }

    fn next(self) -> Option<StateMachine<C, M>> {
        Some(PhaseState::<Update, _, _>::new(self.shared).into())
    }
}

impl<C, M> PhaseState<Sum, C, M>
where
    Self: Handler + Phase<C, M>,
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Processes requests until there are enough.
    async fn process_until_enough(&mut self) -> Result<(), PhaseStateError> {
        while !self.has_enough_sums() {
            debug!(
                "{} sum messages successfully handled (min {} and max {} required)",
                self.private.accepted,
                self.shared.state.min_sum_count,
                self.shared.state.max_sum_count,
            );
            self.process_next().await?;
        }
        Ok(())
    }

    fn increment_message_metric(&self, meas: Measurement) {
        metric!(
            meas,
            1,
            ("round_id", self.shared.state.round_id),
            ("phase", Self::NAME as u8)
        );
    }
}

impl<C, M> PhaseState<Sum, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Creates a new sum state.
    pub fn new(shared: Shared<C, M>) -> Self {
        Self {
            private: Sum::default(),
            shared,
        }
    }

    /// Checks whether enough sum participants submitted their ephemeral keys to start the update
    /// phase.
    fn has_enough_sums(&self) -> bool {
        self.private.accepted >= self.shared.state.min_sum_count
    }

    /// Checks whether too many sum participants submitted their ephemeral keys to start the update
    /// phase.
    fn has_overmuch_sums(&self) -> bool {
        self.private.accepted >= self.shared.state.max_sum_count
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        state_machine::{
            events::Event,
            tests::{builder::StateMachineBuilder, utils},
        },
        storage::tests::init_store,
    };
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    pub async fn integration_sum_to_update() {
        utils::enable_logging();
        let mut store = init_store().await;

        let sum = Sum::default();
        let (state_machine, request_tx, events) = StateMachineBuilder::new(store.clone())
            .with_phase(sum)
            // Make sure anyone is a sum participant.
            .with_sum_ratio(1.0)
            .with_update_ratio(0.0)
            // Make sure a single participant is enough to go to the
            // update phase
            .with_min_sum_count(1)
            .with_max_sum_count(10)
            .with_model_length(4)
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
        let summer = utils::generate_summer(round_params.clone());
        let sum_msg = summer.compose_sum_message();
        let request_fut = async { request_tx.msg(&sum_msg).await.unwrap() };
        let transition_fut = async { state_machine.next().await.unwrap() };

        let (_response, state_machine) = tokio::join!(request_fut, transition_fut);
        let PhaseState {
            private: update_state,
            shared,
            ..
        } = state_machine.into_update_phase_state();

        // Check the initial state of the update phase.
        let frozen_sum_dict = store.sum_dict().await.unwrap().unwrap();
        assert_eq!(frozen_sum_dict.len(), 1);
        let (pk, ephm_pk) = frozen_sum_dict.iter().next().unwrap();
        assert_eq!(pk.clone(), summer.keys.public);
        assert_eq!(ephm_pk.clone(), utils::ephm_pk(&sum_msg));

        let seed_dict = store.seed_dict().await.unwrap().unwrap();
        assert_eq!(seed_dict.len(), 1);
        let (pk, dict) = seed_dict.iter().next().unwrap();
        assert_eq!(pk.clone(), summer.keys.public);
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
