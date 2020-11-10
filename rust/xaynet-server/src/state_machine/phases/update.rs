use std::sync::Arc;

use async_trait::async_trait;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

#[cfg(feature = "metrics")]
use crate::metrics;
use crate::state_machine::{
    events::{DictionaryUpdate, MaskLengthUpdate},
    phases::{Handler, Phase, PhaseName, PhaseState, PhaseStateError, Shared, Sum2},
    requests::{StateMachineRequest, UpdateRequest},
    RequestError, StateMachine,
};
use crate::storage::CoordinatorStorage;
use xaynet_core::{
    mask::{Aggregation, MaskObject},
    LocalSeedDict, UpdateParticipantPublicKey,
};
use xaynet_macros::metrics;

/// Update state
#[derive(Debug)]
pub struct Update {
    /// The aggregator for masked models.
    model_agg: Aggregation,

    /// The number of Update messages successfully processed.
    update_count: u64,
}

#[cfg(test)]
impl Update {
    pub fn aggregation(&self) -> &Aggregation {
        &self.model_agg
    }
}

#[async_trait]
impl Phase for PhaseState<Update>
where
    Self: Handler,
{
    const NAME: PhaseName = PhaseName::Update;

    /// Moves from the update state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn run(&mut self) -> Result<(), PhaseStateError> {
        let min_time = self.shared.state.min_update_time;
        debug!("in update phase for a minimum of {} seconds", min_time);
        self.process_during(Duration::from_secs(min_time)).await?;

        let time_left = self.shared.state.max_update_time - min_time;
        timeout(Duration::from_secs(time_left), self.process_until_enough()).await??;

        info!(
            "{} update messages handled (min {} required)",
            self.inner.update_count, self.shared.state.min_update_count
        );

        info!("broadcasting mask length");
        self.shared
            .io
            .events
            .broadcast_mask_length(MaskLengthUpdate::New(self.inner.model_agg.len()));

        let seed_dict = self
            .shared
            .io
            .redis
            .seed_dict()
            .await?
            .ok_or(PhaseStateError::NoSeedDict)?;

        info!("broadcasting the global seed dictionary");
        self.shared
            .io
            .events
            .broadcast_seed_dict(DictionaryUpdate::New(Arc::new(seed_dict)));

        Ok(())
    }

    fn next(self) -> Option<StateMachine> {
        Some(PhaseState::<Sum2>::new(self.shared, self.inner.model_agg).into())
    }
}

impl PhaseState<Update>
where
    Self: Handler + Phase,
{
    /// Processes requests until there are enough.
    async fn process_until_enough(&mut self) -> Result<(), PhaseStateError> {
        while !self.has_enough_updates() {
            debug!(
                "{} update messages handled (min {} required)",
                self.inner.update_count, self.shared.state.min_update_count
            );
            self.process_next().await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Handler for PhaseState<Update> {
    /// Handles a [`StateMachineRequest`].
    ///
    /// If the request is a [`StateMachineRequest::Sum`] or
    /// [`StateMachineRequest::Sum2`] request, the request sender will
    /// receive a [`RequestError::MessageRejected`].
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError> {
        match req {
            StateMachineRequest::Update(update_req) => {
                metrics!(
                    self.shared.io.metrics_tx,
                    metrics::message::update::increment(self.shared.state.round_id, Self::NAME)
                );
                self.handle_update(update_req).await
            }
            _ => Err(RequestError::MessageRejected),
        }
    }
}

impl PhaseState<Update> {
    /// Creates a new update state.
    pub fn new(shared: Shared) -> Self {
        Self {
            inner: Update {
                update_count: 0,
                model_agg: Aggregation::new(
                    shared.state.mask_config.into(),
                    shared.state.model_size,
                ),
            },
            shared,
        }
    }

    /// Handles an update request.
    /// If the handling of the update message fails, an error is returned to the request sender.
    async fn handle_update(&mut self, req: UpdateRequest) -> Result<(), RequestError> {
        let UpdateRequest {
            participant_pk,
            local_seed_dict,
            masked_model,
        } = req;
        self.update_seed_dict_and_aggregate_mask(&participant_pk, &local_seed_dict, masked_model)
            .await
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
        self.inner
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
        self.inner.model_agg.aggregate(mask_object);
        Ok(())
    }

    /// Adds a local seed dictionary to the seed dictionary.
    ///
    /// # Error
    /// Fails if the local dict cannot be added due to a Redis error.
    async fn add_local_seed_dict(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> Result<(), RequestError> {
        self.shared
            .io
            .redis
            .add_local_seed_dict(pk, local_seed_dict)
            .await?
            .into_inner()?;

        self.inner.update_count += 1;
        Ok(())
    }

    fn has_enough_updates(&self) -> bool {
        self.inner.update_count >= self.shared.state.min_update_count
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state_machine::{
        events::Event,
        tests::{
            builder::StateMachineBuilder,
            utils::{self, Participant},
        },
    };
    use serial_test::serial;
    use xaynet_core::{
        common::{RoundParameters, RoundSeed},
        crypto::{ByteObject, EncryptKeyPair},
        mask::{FromPrimitives, MaskConfig, Model},
        SeedDict, SumDict, UpdateSeedDict,
    };

    #[tokio::test]
    #[serial]
    pub async fn integration_update_to_sum2() {
        utils::enable_logging();
        let round_params = RoundParameters {
            pk: EncryptKeyPair::generate().public,
            sum: 0.5,
            update: 1.0,
            seed: RoundSeed::generate(),
        };
        let n_updaters = 1;
        let n_summers = 1;
        let model_size = 4;

        // Find a sum participant and an update participant for the
        // given seed and ratios.
        let summer = utils::generate_summer(round_params.clone());
        let updater = utils::generate_updater(round_params.clone());

        // Initialize the update phase state
        let mut frozen_sum_dict = SumDict::new();
        frozen_sum_dict.insert(summer.keys.public, summer.ephm_keys.public);

        let config: MaskConfig = utils::mask_settings().into();
        let aggregation = Aggregation::new(config.into(), model_size);

        // Create the state machine
        let (state_machine, request_tx, events, eio) = StateMachineBuilder::new()
            .await
            .with_seed(round_params.seed.clone())
            .with_phase(Update {
                update_count: 0,
                model_agg: aggregation.clone(),
            })
            .with_sum_ratio(round_params.sum)
            .with_update_ratio(round_params.update)
            .with_min_sum(n_summers)
            .with_min_update(n_updaters)
            .with_min_update_time(1)
            .with_max_update_time(2)
            .with_mask_config(utils::mask_settings().into())
            .build();

        // We need to add the sum participant to the sum_dict because the sum_pks are used
        // to compose the seed_dict when fetching the seed_dict from redis.
        eio.redis
            .add_sum_participant(&summer.keys.public, &summer.ephm_keys.public)
            .await
            .unwrap();

        assert!(state_machine.is_update());

        // Create an update request.
        let scalar = 1.0 / (n_updaters as f64 * round_params.update);
        let model = Model::from_primitives(vec![0; model_size].into_iter()).unwrap();
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
            inner: sum2_state, ..
        } = state_machine.into_sum2_phase_state();

        // Check the initial state of the sum2 phase.

        // The sum dict should be unchanged
        let sum_dict = eio.redis.sum_dict().await.unwrap().unwrap();
        assert_eq!(sum_dict, frozen_sum_dict);
        // We have only one updater, so the aggregation should contain
        // the masked model from that updater
        assert_eq!(
            <Aggregation as Into<MaskObject>>::into(sum2_state.aggregation().clone()),
            masked_model
        );
        let best_masks = eio.redis.best_masks().await.unwrap().unwrap();
        assert!(best_masks.is_empty());

        // Check all the events that should be emitted during the update
        // phase
        assert_eq!(
            events.phase_listener().get_latest(),
            Event {
                round_id: 0,
                event: PhaseName::Update,
            }
        );
        assert_eq!(
            events.mask_length_listener().get_latest(),
            Event {
                round_id: 0,
                event: MaskLengthUpdate::New(model.len()),
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
