use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

use crate::{
    state_machine::{
        events::DictionaryUpdate,
        phases::{Handler, Phase, PhaseName, PhaseState, PhaseStateError, Shared, Update},
        requests::{StateMachineRequest, SumRequest},
        RequestError, StateMachine,
    },
    storage::{Storage, StorageError},
};
use xaynet_core::{SumParticipantEphemeralPublicKey, SumParticipantPublicKey};

/// Error that occurs during the sum phase.
#[derive(Error, Debug)]
pub enum SumStateError {
    #[error("sum dictionary does not exists")]
    NoSumDict,
    #[error("fetching sum dictionary failed: {0}")]
    FetchSumDict(StorageError),
}

/// The sum state.
#[derive(Debug)]
pub struct Sum {
    /// The number of sum messages successfully processed.
    accepted: u64,
    /// The number of sum messages failed to processed.
    rejected: u64,
    /// The number of sum messages discarded without being processed.
    discarded: u64,
    //phase parameters
}

#[async_trait]
impl<S> Phase<S> for PhaseState<Sum, S>
where
    Self: Handler,
    S: Storage,
{
    const NAME: PhaseName = PhaseName::Sum;

    async fn run(&mut self) -> Result<(), PhaseStateError> {
        let min_time = self.shared.state.sum.time.min;
        let max_time = self.shared.state.sum.time.max;
        debug!(
            "in sum phase for min {} and max {} seconds",
            min_time, max_time,
        );
        self.process_during(Duration::from_secs(min_time)).await?;

        let time_left = max_time - min_time;
        timeout(Duration::from_secs(time_left), self.process_until_enough()).await??;

        info!(
            "in total {} sum messages accepted (min {} and max {} required)",
            self.private.accepted, self.shared.state.sum.count.min, self.shared.state.sum.count.max,
        );
        info!("in total {} sum messages rejected", self.private.rejected);
        info!("in total {} sum messages discarded", self.private.discarded);

        // purge
        ///////////////////////////////////////////////////////////////////////////////////////////////

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

    fn next(self) -> Option<StateMachine<S>> {
        Some(PhaseState::<Update, _>::new(self.shared).into())
    }
}

#[async_trait]
impl<S> Handler for PhaseState<Sum, S>
where
    S: Storage,
{
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError> {
        if let StateMachineRequest::Sum(SumRequest {
            participant_pk,
            ephm_pk,
        }) = req
        {
            self.update_sum_dict(participant_pk, ephm_pk).await
        } else {
            Err(RequestError::MessageRejected)
        }
    }

    fn has_enough_messages(&self) -> bool {
        self.private.accepted >= self.shared.state.sum.count.min
    }

    fn has_overmuch_messages(&self) -> bool {
        self.private.accepted >= self.shared.state.sum.count.max
    }

    fn increment_accepted(&mut self) {
        self.private.accepted += 1;
        debug!(
            "{} sum messages accepted (min {} and max {} required)",
            self.private.accepted, self.shared.state.sum.count.min, self.shared.state.sum.count.max,
        );
    }

    fn increment_rejected(&mut self) {
        self.private.rejected += 1;
        debug!("{} sum messages rejected", self.private.rejected);
    }

    fn increment_discarded(&mut self) {
        self.private.discarded += 1;
        debug!("{} sum messages discarded", self.private.discarded);
    }
}

impl<S> PhaseState<Sum, S>
where
    S: Storage,
{
    /// Creates a new sum state.
    pub fn new(shared: Shared<S>) -> Self {
        Self {
            private: Sum {
                accepted: 0,
                rejected: 0,
                discarded: 0,
            },
            shared,
        }
    }

    /// Updates the sum dict with a sum participant request.
    async fn update_sum_dict(
        &mut self,
        participant_pk: SumParticipantPublicKey,
        ephm_pk: SumParticipantEphemeralPublicKey,
    ) -> Result<(), RequestError> {
        self.shared
            .store
            .add_sum_participant(&participant_pk, &ephm_pk)
            .await?
            .into_inner()
            .map_err(RequestError::from)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;
    use crate::{
        state_machine::{
            events::Event,
            tests::{builder::StateMachineBuilder, utils},
        },
        storage::{tests::init_store, CoordinatorStorage},
    };

    #[tokio::test]
    #[serial]
    pub async fn integration_sum_to_update() {
        utils::enable_logging();
        let mut store = init_store().await;

        let sum = Sum {
            accepted: 0,
            rejected: 0,
            discarded: 0,
        };
        let (state_machine, request_tx, events) = StateMachineBuilder::new(store.clone())
            .with_phase(sum)
            // Make sure anyone is a sum participant.
            .with_sum_probability(1.0)
            .with_update_probability(0.0)
            // Make sure a single participant is enough to go to the
            // update phase
            .with_sum_count_min(1)
            .with_sum_count_max(10)
            .with_model_length(4)
            .with_sum_time_min(1)
            .with_sum_time_max(2)
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
