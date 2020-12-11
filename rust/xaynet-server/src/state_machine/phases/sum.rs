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
        RequestError,
        StateMachine,
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
#[derive(Debug)]
pub struct Sum {
    /// The number of Sum messages successfully processed.
    sum_count: u64,
}

#[async_trait]
impl<C, M> Handler for PhaseState<Sum, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Handles a [`StateMachineRequest`].
    ///
    /// If the request is a [`StateMachineRequest::Update`] or
    /// [`StateMachineRequest::Sum2`] request, the request sender will receive a
    /// [`RequestError::MessageRejected`].
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError> {
        match req {
            StateMachineRequest::Sum(sum_req) => self.handle_sum(sum_req).await.map(|res| {
                self.increment_message_metric();
                res
            }),
            _ => Err(RequestError::MessageRejected),
        }
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
        debug!("in sum phase for a minimum of {} seconds", min_time);
        self.process_during(Duration::from_secs(min_time)).await?;

        let time_left = self.shared.state.max_sum_time - min_time;
        timeout(Duration::from_secs(time_left), self.process_until_enough()).await??;

        info!(
            "{} sum messages handled (min {} required)",
            self.private.sum_count, self.shared.state.min_sum_count
        );

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
                "{} sum messages handled (min {} required)",
                self.private.sum_count, self.shared.state.min_sum_count,
            );
            self.process_next().await?;
        }
        Ok(())
    }

    fn increment_message_metric(&self) {
        metric!(
            Measurement::MessageSum,
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
            private: Sum { sum_count: 0 },
            shared,
        }
    }

    /// Handles a sum request.
    ///
    /// # Error
    ///
    /// Fails if the sum participant cannot be added due to a PET or [`StorageError`].
    async fn handle_sum(&mut self, req: SumRequest) -> Result<(), RequestError> {
        let SumRequest {
            participant_pk,
            ephm_pk,
        } = req;

        self.shared
            .store
            .add_sum_participant(&participant_pk, &ephm_pk)
            .await?
            .into_inner()?;

        self.private.sum_count += 1;
        Ok(())
    }

    /// Checks whether enough sum participants submitted their ephemeral keys to start the update
    /// phase.
    fn has_enough_sums(&self) -> bool {
        self.private.sum_count >= self.shared.state.min_sum_count
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

        let sum = Sum { sum_count: 0 };
        let (state_machine, request_tx, events) = StateMachineBuilder::new(store.clone())
            .with_phase(sum)
            // Make sure anyone is a sum participant.
            .with_sum_ratio(1.0)
            .with_update_ratio(0.0)
            // Make sure a single participant is enough to go to the
            // update phase
            .with_min_sum(1)
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
