use std::sync::Arc;

use crate::{
    mask::{masking::Aggregation, object::MaskObject},
    state_machine::{
        coordinator::CoordinatorState,
        events::{DictionaryUpdate, MaskLengthUpdate, ScalarUpdate},
        phases::{Handler, Phase, PhaseName, PhaseState, StateError, Sum2},
        requests::{RequestReceiver, StateMachineRequest, UpdateRequest},
        StateMachine,
    },
    LocalSeedDict,
    PetError,
    SeedDict,
    SumDict,
    UpdateParticipantPublicKey,
};

use tokio::time::{timeout, Duration};

/// Update state
#[derive(Debug)]
pub struct Update {
    /// The frozen sum dictionary built during the sum phase.
    frozen_sum_dict: SumDict,

    /// The seed dictionary built during the update phase.
    seed_dict: SeedDict,

    /// The aggregator for masks and masked models.
    aggregation: Aggregation,
}

#[cfg(test)]
impl Update {
    pub fn frozen_sum_dict(&self) -> &SumDict {
        &self.frozen_sum_dict
    }
    pub fn seed_dict(&self) -> &SeedDict {
        &self.seed_dict
    }
    pub fn aggregation(&self) -> &Aggregation {
        &self.aggregation
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
        let scalar = 1_f64
            / (self.coordinator_state.expected_participants as f64
                * self.coordinator_state.round_params.update);
        info!("broadcasting scalar: {}", scalar);
        self.coordinator_state
            .events
            .broadcast_scalar(ScalarUpdate::New(scalar));

        let min_time = self.coordinator_state.min_update_time;
        debug!("in update phase for a minimum of {} seconds", min_time);
        self.process_during(Duration::from_secs(min_time)).await?;

        let time_left = self.coordinator_state.max_update_time - min_time;
        timeout(Duration::from_secs(time_left), self.process_until_enough()).await??;

        info!(
            "{} update messages handled (min {} required)",
            self.updater_count(),
            self.coordinator_state.min_update_count
        );
        Ok(())
    }

    fn next(self) -> Option<StateMachine> {
        let PhaseState {
            inner:
                Update {
                    frozen_sum_dict,
                    seed_dict,
                    aggregation,
                },
            mut coordinator_state,
            request_rx,
        } = self;

        info!("broadcasting mask length");
        coordinator_state
            .events
            .broadcast_mask_length(MaskLengthUpdate::New(aggregation.len()));

        info!("broadcasting the global seed dictionary");
        coordinator_state
            .events
            .broadcast_seed_dict(DictionaryUpdate::New(Arc::new(seed_dict)));

        Some(
            PhaseState::<Sum2>::new(coordinator_state, request_rx, frozen_sum_dict, aggregation)
                .into(),
        )
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
                self.updater_count(),
                self.coordinator_state.min_update_count
            );
            self.process_single().await?;
        }
        Ok(())
    }
}

impl Handler for PhaseState<Update> {
    /// Handles a [`StateMachineRequest`].
    ///
    /// If the request is a [`StateMachineRequest::Sum`] or
    /// [`StateMachineRequest::Sum2`] request, the request sender will
    /// receive a [`PetError::InvalidMessage`].
    fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), PetError> {
        match req {
            StateMachineRequest::Update(update_req) => self.handle_update(update_req),
            _ => Err(PetError::InvalidMessage),
        }
    }
}

impl PhaseState<Update> {
    /// Creates a new update state.
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: RequestReceiver,
        frozen_sum_dict: SumDict,
        seed_dict: SeedDict,
    ) -> Self {
        info!("state transition");
        Self {
            inner: Update {
                frozen_sum_dict,
                seed_dict,
                aggregation: Aggregation::new(
                    coordinator_state.mask_config,
                    coordinator_state.model_size,
                ),
            },
            coordinator_state,
            request_rx,
        }
    }

    /// Handles an update request.
    /// If the handling of the update message fails, an error is returned to the request sender.
    fn handle_update(&mut self, req: UpdateRequest) -> Result<(), PetError> {
        let UpdateRequest {
            participant_pk,
            local_seed_dict,
            masked_model,
        } = req;
        self.update_seed_dict_and_aggregate_mask(&participant_pk, &local_seed_dict, masked_model)
    }

    /// Updates the local seed dict and aggregates the masked model.
    fn update_seed_dict_and_aggregate_mask(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
        masked_model: MaskObject,
    ) -> Result<(), PetError> {
        // Check if aggregation can be performed. It is important to
        // do that _before_ updating the seed dictionary, because we
        // don't want to add the local seed dict if the corresponding
        // masked model is invalid
        debug!("checking whether the masked model can be aggregated");
        self.inner
            .aggregation
            .validate_aggregation(&masked_model)
            .map_err(|e| {
                warn!("aggregation error: {}", e);
                PetError::InvalidMessage
            })?;

        // Try to update local seed dict first. If this fail, we do
        // not want to aggregate the model.
        info!("updating the global seed dictionary");
        self.add_local_seed_dict(pk, local_seed_dict)
            .map_err(|err| {
                warn!("invalid local seed dictionary, ignoring update message");
                err
            })?;

        info!("aggregating the masked model");
        self.inner.aggregation.aggregate(masked_model);
        Ok(())
    }

    /// Adds a local seed dictionary to the seed dictionary.
    ///
    /// # Error
    /// Fails if it contains invalid keys or it is a repetition.
    fn add_local_seed_dict(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> Result<(), PetError> {
        if local_seed_dict.keys().len() == self.inner.frozen_sum_dict.keys().len()
            && local_seed_dict
                .keys()
                .all(|pk| self.inner.frozen_sum_dict.contains_key(pk))
            && self
                .inner
                .seed_dict
                .values()
                .next()
                .map_or(true, |dict| !dict.contains_key(pk))
        {
            debug!("adding local seed dictionary");
            for (sum_pk, seed) in local_seed_dict {
                self.inner
                    .seed_dict
                    .get_mut(sum_pk)
                    .ok_or(PetError::InvalidMessage)?
                    .insert(*pk, seed.clone());
            }
            Ok(())
        } else {
            warn!("invalid seed dictionary");
            Err(PetError::InvalidMessage)
        }
    }

    /// Returns the number of update participants that sent a valid update message.
    fn updater_count(&self) -> usize {
        self.inner
            .seed_dict
            .values()
            .next()
            .map(|dict| dict.len())
            .unwrap_or(0)
    }

    fn has_enough_updates(&self) -> bool {
        self.updater_count() >= self.coordinator_state.min_update_count
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        crypto::{ByteObject, EncryptKeyPair},
        mask::{FromPrimitives, MaskObject, Model},
        state_machine::{
            coordinator::RoundSeed,
            events::Event,
            tests::{
                builder::StateMachineBuilder,
                utils::{generate_summer, generate_updater, mask_settings},
            },
        },
        SumDict,
        UpdateSeedDict,
    };

    #[tokio::test]
    pub async fn update_to_sum2() {
        let n_updaters = 1;
        let n_summers = 1;
        let seed = RoundSeed::generate();
        let sum_ratio = 0.5;
        let update_ratio = 1.0;
        let coord_keys = EncryptKeyPair::generate();
        let model_size = 4;

        // Find a sum participant and an update participant for the
        // given seed and ratios.
        let mut summer = generate_summer(&seed, sum_ratio, update_ratio);
        let updater = generate_updater(&seed, sum_ratio, update_ratio);

        // Initialize the update phase state
        let sum_msg = summer.compose_sum_message(&coord_keys.public);
        let summer_ephm_pk = sum_msg.ephm_pk();

        let mut frozen_sum_dict = SumDict::new();
        frozen_sum_dict.insert(summer.pk, summer_ephm_pk);

        let mut seed_dict = SeedDict::new();
        seed_dict.insert(summer.pk, HashMap::new());
        let aggregation = Aggregation::new(mask_settings().into(), model_size);
        let update = Update {
            frozen_sum_dict: frozen_sum_dict.clone(),
            seed_dict: seed_dict.clone(),
            aggregation: aggregation.clone(),
        };

        // Create the state machine
        let (state_machine, request_tx, events) = StateMachineBuilder::new()
            .with_seed(seed.clone())
            .with_phase(update)
            .with_sum_ratio(sum_ratio)
            .with_update_ratio(update_ratio)
            .with_min_sum(n_summers)
            .with_min_update(n_updaters)
            .with_expected_participants(n_updaters + n_summers)
            .with_mask_config(mask_settings().into())
            .build();

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
        let masked_model = update_msg.masked_model();
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
        assert_eq!(sum2_state.sum_dict(), &frozen_sum_dict);
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
        let encrypted_mask_seed = update_msg
            .local_seed_dict()
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
