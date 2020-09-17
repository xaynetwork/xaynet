use std::sync::Arc;

use xaynet_core::{
    mask::{Aggregation, MaskObject},
    LocalSeedDict,
    UpdateParticipantPublicKey,
};

use crate::state_machine::{
    events::{DictionaryUpdate, MaskLengthUpdate},
    phases::{Handler, Phase, PhaseName, PhaseState, Shared, StateError, Sum2},
    requests::{StateMachineRequest, UpdateRequest},
    StateMachine,
    StateMachineError,
};

#[cfg(feature = "metrics")]
use crate::metrics;

use tokio::time::{timeout, Duration};

/// Update state
#[derive(Debug)]
pub struct Update {
    update_count: usize,

    /// The aggregator for masked models.
    model_agg: Aggregation,

    /// The aggregator for masked scalars.
    scalar_agg: Aggregation,
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
    async fn run(&mut self) -> Result<(), StateError> {
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
            .connection()
            .await
            .get_seed_dict()
            .await?;

        info!("broadcasting the global seed dictionary");
        self.shared
            .io
            .events
            .broadcast_seed_dict(DictionaryUpdate::New(Arc::new(seed_dict)));

        Ok(())
    }

    fn next(self) -> Option<StateMachine> {
        let PhaseState {
            inner:
                Update {
                    model_agg,
                    scalar_agg,
                    ..
                },
            shared,
        } = self;

        Some(PhaseState::<Sum2>::new(shared, model_agg, scalar_agg).into())
    }
}

impl PhaseState<Update>
where
    Self: Handler + Phase,
{
    /// Processes requests until there are enough.
    async fn process_until_enough(&mut self) -> Result<(), StateError> {
        while !self.has_enough_updates() {
            debug!(
                "{} update messages handled (min {} required)",
                self.inner.update_count, self.shared.state.min_update_count
            );
            self.process_single().await?;
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
    /// receive a [`StateMachineError::MessageRejected`].
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), StateMachineError> {
        match req {
            StateMachineRequest::Update(update_req) => {
                metrics!(
                    self.shared.io.metrics_tx,
                    metrics::message::update::increment(self.shared.state.round_id, Self::NAME)
                );
                self.handle_update(update_req).await
            }
            _ => Err(StateMachineError::MessageRejected),
        }
    }
}

impl PhaseState<Update> {
    /// Creates a new update state.
    pub fn new(shared: Shared) -> Self {
        info!("state transition");
        Self {
            inner: Update {
                update_count: 0,
                model_agg: Aggregation::new(shared.state.mask_config, shared.state.model_size),
                // TODO separate config for scalars
                scalar_agg: Aggregation::new(shared.state.mask_config, 1),
            },
            shared,
        }
    }

    /// Handles an update request.
    /// If the handling of the update message fails, an error is returned to the request sender.
    async fn handle_update(&mut self, req: UpdateRequest) -> Result<(), StateMachineError> {
        let UpdateRequest {
            participant_pk,
            local_seed_dict,
            masked_model,
            masked_scalar,
        } = req;
        self.update_seed_dict_and_aggregate_mask(
            &participant_pk,
            &local_seed_dict,
            masked_model,
            masked_scalar,
        )
        .await
    }

    /// Updates the local seed dict and aggregates the masked model.
    async fn update_seed_dict_and_aggregate_mask(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
        masked_model: MaskObject,
        masked_scalar: MaskObject,
    ) -> Result<(), StateMachineError> {
        // Check if aggregation can be performed. It is important to
        // do that _before_ updating the seed dictionary, because we
        // don't want to add the local seed dict if the corresponding
        // masked model is invalid
        debug!("checking whether the masked model can be aggregated");
        self.inner
            .model_agg
            .validate_aggregation(&masked_model)
            .map_err(|e| {
                warn!("model aggregation error: {}", e);
                StateMachineError::AggregationFailed
            })?;

        debug!("checking whether the masked scalar can be aggregated");
        self.inner
            .scalar_agg
            .validate_aggregation(&masked_scalar)
            .map_err(|e| {
                warn!("scalar aggregation error: {}", e);
                StateMachineError::AggregationFailed
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
        self.inner.model_agg.aggregate(masked_model);
        self.inner.scalar_agg.aggregate(masked_scalar);
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
    ) -> Result<(), StateMachineError> {
        self.shared
            .io
            .redis
            .connection()
            .await
            .update_seed_dict(pk, local_seed_dict)
            .await?;

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
        tests::{builder::StateMachineBuilder, utils},
    };
    use serial_test::serial;
    use xaynet_core::{
        common::RoundSeed,
        crypto::{ByteObject, EncryptKeyPair},
        mask::{FromPrimitives, MaskObject, Model},
        SeedDict,
        SumDict,
        UpdateSeedDict,
    };

    #[tokio::test]
    #[serial]
    pub async fn integration_update_to_sum2() {
        utils::enable_logging();
        let n_updaters = 1;
        let n_summers = 1;
        let seed = RoundSeed::generate();
        let sum_ratio = 0.5;
        let update_ratio = 1.0;
        let coord_keys = EncryptKeyPair::generate();
        let model_size = 4;

        // Find a sum participant and an update participant for the
        // given seed and ratios.
        let mut summer = utils::generate_summer(&seed, sum_ratio, update_ratio);
        let updater = utils::generate_updater(&seed, sum_ratio, update_ratio);

        // Initialize the update phase state
        let sum_msg = summer.compose_sum_message(coord_keys.public);
        let summer_ephm_pk = utils::ephm_pk(&sum_msg);

        let mut frozen_sum_dict = SumDict::new();
        frozen_sum_dict.insert(summer.pk, summer_ephm_pk);

        let model_agg = Aggregation::new(utils::mask_settings().into(), model_size);
        let scalar_agg = Aggregation::new(utils::mask_settings().into(), 1);
        let update = Update {
            update_count: 0,
            model_agg,
            scalar_agg,
        };

        // Create the state machine
        let (state_machine, request_tx, events, redis) = StateMachineBuilder::new()
            .await
            .with_seed(seed.clone())
            .with_phase(update)
            .with_sum_ratio(sum_ratio)
            .with_update_ratio(update_ratio)
            .with_min_sum(n_summers)
            .with_min_update(n_updaters)
            .with_min_update_time(1)
            .with_max_update_time(2)
            .with_mask_config(utils::mask_settings().into())
            .build();

        // We need to add the sum participant to the sum_dict because the sum_pks are used
        // to compose the seed_dict when fetching the seed_dict from redis.
        redis
            .connection()
            .await
            .add_sum_participant(&summer.pk, &summer_ephm_pk)
            .await
            .unwrap();

        assert!(state_machine.is_update());

        // Create an update request.
        let scalar = 1.0 / (n_updaters as f64 * update_ratio);
        let model = Model::from_primitives(vec![0; model_size].into_iter()).unwrap();
        let update_msg = updater.compose_update_message(
            coord_keys.public,
            &frozen_sum_dict,
            scalar,
            model.clone(),
        );
        let masked_model = utils::masked_model(&update_msg);
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
        let sum_dict = redis.connection().await.get_sum_dict().await.unwrap();
        assert_eq!(sum_dict, frozen_sum_dict);
        // We have only one updater, so the aggregation should contain
        // the masked model from that updater
        assert_eq!(
            <Aggregation as Into<MaskObject>>::into(sum2_state.aggregation().clone().into()),
            masked_model
        );
        assert!(sum2_state.mask_dict().is_empty());

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
        let encrypted_mask_seed = utils::local_seed_dict(&update_msg)
            .values()
            .next()
            .unwrap()
            .clone();
        entry.insert(updater.pk, encrypted_mask_seed);
        global_seed_dict.insert(summer.pk, entry);
        assert_eq!(
            events.seed_dict_listener().get_latest(),
            Event {
                round_id: 0,
                event: DictionaryUpdate::New(Arc::new(global_seed_dict)),
            }
        );
    }
}
