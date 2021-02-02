use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tokio::time::{timeout, Duration, Instant};
use tracing::{debug, info, warn};

use crate::{
    state_machine::{
        events::DictionaryUpdate,
        phases::{Handler, Phase, PhaseName, PhaseState, PhaseStateError, Shared, Sum2},
        requests::{StateMachineRequest, UpdateRequest},
        RequestError,
        StateMachine,
    },
    storage::{Storage, StorageError},
};
use xaynet_core::{
    mask::{Aggregation, MaskObject},
    LocalSeedDict,
    UpdateParticipantPublicKey,
};

/// Error that occurs during the update phase.
#[derive(Error, Debug)]
pub enum UpdateStateError {
    #[error("seed dictionary does not exists")]
    NoSeedDict,
    #[error("fetching seed dictionary failed: {0}")]
    FetchSeedDict(StorageError),
}

/// The update state.
#[derive(Debug)]
pub struct Update {
    /// The aggregator for masked models.
    model_agg: Aggregation,
    /// The number of update messages successfully processed.
    accepted: u64,
    /// The number of update messages failed to processed.
    rejected: u64,
    /// The number of update messages discarded without being processed.
    discarded: u64,
}

#[async_trait]
impl<S> Phase<S> for PhaseState<Update, S>
where
    Self: Handler,
    S: Storage,
{
    const NAME: PhaseName = PhaseName::Update;

    async fn run(&mut self) -> Result<(), PhaseStateError> {
        let min_time = self.shared.state.update.time.min;
        let max_time = self.shared.state.update.time.max;
        debug!(
            "in update phase for min {} and max {} seconds",
            min_time, max_time,
        );
        self.process_during(Duration::from_secs(min_time)).await?;

        let time_left = max_time - min_time;
        timeout(Duration::from_secs(time_left), self.process_until_enough()).await??;

        info!(
            "in total {} update messages accepted (min {} and max {} required)",
            self.private.accepted,
            self.shared.state.update.count.min,
            self.shared.state.update.count.max,
        );
        info!(
            "in total {} update messages rejected",
            self.private.rejected,
        );
        info!(
            "in total {} update messages discarded",
            self.private.discarded,
        );

        let seed_dict = self
            .shared
            .store
            .seed_dict()
            .await
            .map_err(UpdateStateError::FetchSeedDict)?
            .ok_or(UpdateStateError::NoSeedDict)?;

        info!("broadcasting the global seed dictionary");
        self.shared
            .events
            .broadcast_seed_dict(DictionaryUpdate::New(Arc::new(seed_dict)));

        Ok(())
    }

    fn next(self) -> Option<StateMachine<S>> {
        Some(PhaseState::<Sum2, _>::new(self.shared, self.private.model_agg).into())
    }
}

#[async_trait]
impl<S> Handler for PhaseState<Update, S>
where
    S: Storage,
{
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError> {
        if let StateMachineRequest::Update(UpdateRequest {
            participant_pk,
            local_seed_dict,
            masked_model,
        }) = req
        {
            self.update_seed_dict_and_aggregate_mask(
                &participant_pk,
                &local_seed_dict,
                masked_model,
            )
            .await
        } else {
            Err(RequestError::MessageRejected)
        }
    }

    fn has_enough_messages(&self) -> bool {
        self.private.accepted >= self.shared.state.update.count.min
    }

    fn has_overmuch_messages(&self) -> bool {
        self.private.accepted >= self.shared.state.update.count.max
    }

    fn increment_accepted(&mut self) {
        self.private.accepted += 1;
        debug!(
            "{} update messages accepted (min {} and max {} required)",
            self.private.accepted,
            self.shared.state.update.count.min,
            self.shared.state.update.count.max,
        );
    }

    fn increment_rejected(&mut self) {
        self.private.rejected += 1;
        debug!("{} update messages rejected", self.private.rejected);
    }

    fn increment_discarded(&mut self) {
        self.private.discarded += 1;
        debug!("{} update messages discarded", self.private.discarded);
    }
}

impl<S> PhaseState<Update, S>
where
    S: Storage,
{
    /// Creates a new update state.
    pub fn new(shared: Shared<S>) -> Self {
        Self {
            private: Update {
                model_agg: Aggregation::new(
                    shared.state.round_params.mask_config,
                    shared.state.round_params.model_length,
                ),
                accepted: 0,
                rejected: 0,
                discarded: 0,
            },
            shared,
        }
    }

    /// Updates the local seed dict and aggregates the masked model.
    async fn update_seed_dict_and_aggregate_mask(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
        mask_object: MaskObject,
    ) -> Result<(), RequestError> {
        // Check if aggregation can be performed. It is important to
        // do that _before_ updating the seed dictionary, because we
        // don't want to add the local seed dict if the corresponding
        // masked model is invalid
        debug!("checking whether the masked model can be aggregated");
        self.private
            .model_agg
            .validate_aggregation(&mask_object)
            .map_err(|e| {
                warn!("model aggregation error: {}", e);
                RequestError::AggregationFailed
            })?;

        // Try to update local seed dict first. If this fail, we do
        // not want to aggregate the model.
        info!("updating the global seed dictionary");
        self.add_local_seed_dict(pk, local_seed_dict)
            .await
            .map_err(|err| {
                warn!("invalid local seed dictionary, ignoring update message");
                err
            })?;

        info!("aggregating the masked model and scalar");
        let now = Instant::now();
        self.private.model_agg.aggregate(mask_object);
        info!("model aggregation took {} millis", now.elapsed().as_millis());
        Ok(())
    }

    /// Adds a local seed dictionary to the seed dictionary.
    ///
    /// # Error
    ///
    /// Fails if the local seed dict cannot be added due to a PET or [`StorageError`].
    async fn add_local_seed_dict(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> Result<(), RequestError> {
        self.shared
            .store
            .add_local_seed_dict(pk, local_seed_dict)
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
            tests::{
                builder::StateMachineBuilder,
                utils::{self, Participant},
            },
        },
        storage::{tests::init_store, CoordinatorStorage},
    };
    use xaynet_core::{
        common::{RoundParameters, RoundSeed},
        crypto::{ByteObject, EncryptKeyPair},
        mask::{FromPrimitives, Model},
        SeedDict,
        SumDict,
        UpdateSeedDict,
    };

    impl Update {
        pub fn aggregation(&self) -> &Aggregation {
            &self.model_agg
        }
    }

    #[tokio::test]
    #[serial]
    pub async fn integration_update_to_sum2() {
        utils::enable_logging();
        let model_length = 4;
        let round_params = RoundParameters {
            pk: EncryptKeyPair::generate().public,
            sum: 0.5,
            update: 1.0,
            seed: RoundSeed::generate(),
            mask_config: utils::mask_config(),
            model_length,
        };
        let n_updaters = 1;
        let n_summers = 1;

        // Find a sum participant and an update participant for the
        // given seed and ratios.
        let summer = utils::generate_summer(round_params.clone());
        let updater = utils::generate_updater(round_params.clone());

        // Initialize the update phase state
        let mut frozen_sum_dict = SumDict::new();
        frozen_sum_dict.insert(summer.keys.public, summer.ephm_keys.public);

        let aggregation = Aggregation::new(utils::mask_config(), model_length);

        let mut store = init_store().await;
        // Create the state machine
        let (state_machine, request_tx, events) = StateMachineBuilder::new(store.clone())
            .with_seed(round_params.seed.clone())
            .with_phase(Update {
                model_agg: aggregation.clone(),
                accepted: 0,
                rejected: 0,
                discarded: 0,
            })
            .with_sum_probability(round_params.sum)
            .with_update_probability(round_params.update)
            .with_sum_count_min(n_summers)
            .with_sum_count_max(n_summers + 10)
            .with_update_count_min(n_updaters)
            .with_update_count_max(n_updaters + 10)
            .with_update_time_min(1)
            .with_update_time_max(2)
            .with_mask_config(utils::mask_settings().into())
            .build();

        // We need to add the sum participant to follow the pet protocol
        store
            .add_sum_participant(&summer.keys.public, &summer.ephm_keys.public)
            .await
            .unwrap();

        assert!(state_machine.is_update());

        // Create an update request.
        let scalar = 1.0 / (n_updaters as f64 * round_params.update);
        let model = Model::from_primitives(vec![0; model_length].into_iter()).unwrap();
        let (mask_seed, masked_model) = updater.compute_masked_model(&model, scalar);
        let local_seed_dict = Participant::build_seed_dict(&frozen_sum_dict, &mask_seed);
        let update_msg =
            updater.compose_update_message(masked_model.clone(), local_seed_dict.clone());
        let request_fut = async { request_tx.msg(&update_msg).await.unwrap() };

        // Have the state machine process the request
        let transition_fut = async { state_machine.next().await.unwrap() };
        let (_response, state_machine) = tokio::join!(request_fut, transition_fut);

        // Extract state of the state machine
        let PhaseState {
            private: sum2_state,
            ..
        } = state_machine.into_sum2_phase_state();

        // Check the initial state of the sum2 phase.

        // The sum dict should be unchanged
        let sum_dict = store.sum_dict().await.unwrap().unwrap();
        assert_eq!(sum_dict, frozen_sum_dict);
        // We have only one updater, so the aggregation should contain
        // the masked model from that updater
        assert_eq!(
            <Aggregation as Into<MaskObject>>::into(sum2_state.aggregation().clone()),
            masked_model
        );
        let best_masks = store.best_masks().await.unwrap();
        assert!(best_masks.is_none());

        // Check all the events that should be emitted during the update
        // phase
        assert_eq!(
            events.phase_listener().get_latest(),
            Event {
                round_id: 0,
                event: PhaseName::Update,
            }
        );

        // Compute the global seed dictionary that we expect to be
        // broadcasted. It has a single entry for our sum
        // participant. That entry is an UpdateSeedDictionary that
        // contains the encrypted mask seed from our update
        // participant.
        let mut global_seed_dict = SeedDict::new();
        let mut entry = UpdateSeedDict::new();
        let encrypted_mask_seed = local_seed_dict.values().next().unwrap().clone();
        entry.insert(updater.keys.public, encrypted_mask_seed);
        global_seed_dict.insert(summer.keys.public, entry);
        assert_eq!(
            events.seed_dict_listener().get_latest(),
            Event {
                round_id: 0,
                event: DictionaryUpdate::New(Arc::new(global_seed_dict)),
            }
        );
    }
}
