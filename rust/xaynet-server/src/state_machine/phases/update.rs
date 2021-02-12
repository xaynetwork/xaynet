use std::sync::Arc;

use async_trait::async_trait;
use displaydoc::Display;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::{
    state_machine::{
        events::DictionaryUpdate,
        phases::{Handler, Phase, PhaseError, PhaseName, PhaseState, Shared, Sum2},
        requests::{RequestError, StateMachineRequest, UpdateRequest},
        StateMachine,
    },
    storage::{Storage, StorageError},
};
use xaynet_core::{
    mask::{Aggregation, MaskObject},
    LocalSeedDict,
    SeedDict,
    UpdateParticipantPublicKey,
};

/// Errors which can occur during the update phase.
#[derive(Debug, Display, Error)]
pub enum UpdateError {
    /// Seed dictionary does not exists.
    NoSeedDict,
    /// Fetching seed dictionary failed: {0}.
    FetchSeedDict(StorageError),
}

/// The update state.
#[derive(Debug)]
pub struct Update {
    /// The aggregator for masked models.
    model_agg: Aggregation,
    /// The seed dictionary which gets assembled during the update phase.
    seed_dict: Option<SeedDict>,
}

#[async_trait]
impl<T> Phase<T> for PhaseState<Update, T>
where
    T: Storage,
    Self: Handler,
{
    const NAME: PhaseName = PhaseName::Update;

    async fn process(&mut self) -> Result<(), PhaseError> {
        self.process(self.shared.state.update).await?;
        self.seed_dict().await?;

        Ok(())
    }

    fn broadcast(&mut self) {
        info!("broadcasting the global seed dictionary");
        let seed_dict = self
            .private
            .seed_dict
            .take()
            .expect("unreachable: never fails when `broadcast()` is called after `process()`");
        self.shared
            .events
            .broadcast_seed_dict(DictionaryUpdate::New(Arc::new(seed_dict)));
    }

    async fn next(self) -> Option<StateMachine<T>> {
        Some(PhaseState::<Sum2, _>::new(self.shared, self.private.model_agg).into())
    }
}

#[async_trait]
impl<T> Handler for PhaseState<Update, T>
where
    T: Storage,
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
}

impl<T> PhaseState<Update, T> {
    /// Creates a new update state.
    pub fn new(shared: Shared<T>) -> Self {
        let model_agg = Aggregation::new(
            shared.state.round_params.mask_config,
            shared.state.round_params.model_length,
        );
        Self {
            private: Update {
                model_agg,
                seed_dict: None,
            },
            shared,
        }
    }
}

impl<T> PhaseState<Update, T>
where
    T: Storage,
{
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
        self.private.model_agg.aggregate(mask_object);
        Ok(())
    }

    /// Adds a local seed dictionary to the global seed dictionary.
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

    /// Gets the global seed dict from the store.
    async fn seed_dict(&mut self) -> Result<(), UpdateError> {
        self.private.seed_dict = self
            .shared
            .store
            .seed_dict()
            .await
            .map_err(UpdateError::FetchSeedDict)?
            .ok_or(UpdateError::NoSeedDict)?
            .into();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::anyhow;
    use xaynet_core::{SeedDict, SumDict};

    use crate::{
        state_machine::{
            coordinator::CoordinatorState,
            events::{EventPublisher, EventSubscriber, ModelUpdate},
            tests::{
                utils::{
                    assert_event_updated,
                    enable_logging,
                    init_shared,
                    send_update_messages,
                    send_update_messages_with_model,
                    EventSnapshot,
                },
                CoordinatorStateBuilder,
                EventBusBuilder,
            },
        },
        storage::{
            tests::{
                utils::{create_global_model, create_mask},
                MockCoordinatorStore,
                MockModelStore,
            },
            LocalSeedDictAdd,
            LocalSeedDictAddError,
            Store,
        },
    };

    fn events_from_sum_phase(state: &CoordinatorState) -> (EventPublisher, EventSubscriber) {
        EventBusBuilder::new(state)
            .broadcast_phase(PhaseName::Sum)
            .broadcast_sum_dict(DictionaryUpdate::New(Arc::new(SumDict::new())))
            .broadcast_seed_dict(DictionaryUpdate::Invalidate)
            .broadcast_model(ModelUpdate::New(Arc::new(create_global_model(1))))
            .build()
    }

    fn assert_after_phase_success(
        state_before: &CoordinatorState,
        events_before: &EventSnapshot,
        state_after: &CoordinatorState,
        events_after: &EventSnapshot,
    ) {
        assert_eq!(state_after, state_before);

        assert_event_updated(&events_after.phase, &events_before.phase);
        assert_event_updated(&events_after.seed_dict, &events_before.seed_dict);
        assert_eq!(events_after.keys, events_before.keys);
        assert_eq!(events_after.params, events_before.params);
        assert_eq!(events_after.phase.event, PhaseName::Update);
        assert_eq!(events_after.sum_dict, events_before.sum_dict);
        assert_eq!(events_after.model, events_before.model);
    }

    fn assert_after_phase_failure(
        state_before: &CoordinatorState,
        events_before: &EventSnapshot,
        state_after: &CoordinatorState,
        events_after: &EventSnapshot,
    ) {
        assert_eq!(state_after, state_before);

        assert_event_updated(&events_after.phase, &events_before.phase);
        assert_eq!(events_after.keys, events_before.keys);
        assert_eq!(events_after.params, events_before.params);
        assert_eq!(events_after.phase.event, PhaseName::Update);
        assert_eq!(events_after.sum_dict, events_before.sum_dict);
        assert_eq!(events_after.seed_dict, events_before.seed_dict);
        assert_eq!(events_after.model, events_before.model);
    }

    #[tokio::test]
    async fn test_update_to_sum2_phase() {
        // No Storage errors
        // lets pretend we come from the sum phase
        //
        // What should happen:
        // 1. broadcast Update phase
        // 2. accept 10 update messages
        // 3. fetch seed dict
        // 4. broadcast seed dict
        // 5. move into sum2 phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been invalidated
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_local_seed_dict()
            .times(10)
            .returning(move |_, _| Ok(LocalSeedDictAdd(Ok(()))));
        cs.expect_seed_dict()
            .return_once(move || Ok(Some(SeedDict::new())));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_update_count_min(10)
            .with_update_count_max(10)
            .with_update_time_min(1)
            .build();

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_update = EventSnapshot::from(&event_subscriber);
        let state_before_update = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Update, _>::new(shared));
        assert!(state_machine.is_update());

        send_update_messages(10, request_tx.clone());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_update = state_machine.as_ref().clone();
        let events_after_update = EventSnapshot::from(&event_subscriber);
        assert_after_phase_success(
            &state_before_update,
            &events_before_update,
            &state_after_update,
            &events_after_update,
        );

        assert!(state_machine.is_sum2());
    }

    #[tokio::test]
    async fn test_update_to_sum2_fetch_seed_dict_failed() {
        // Storage errors
        // - seed_dict fails
        //
        // What should happen:
        // 1. broadcast Update phase
        // 2. accept 1 update message
        // 3. fetch seed dict (fails)
        // 4. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been invalidated
        // - the seed dict has been broadcasted
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_local_seed_dict()
            .times(1)
            .returning(move |_, _| Ok(LocalSeedDictAdd(Ok(()))));
        cs.expect_seed_dict().return_once(move || Err(anyhow!("")));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_update_count_min(1)
            .with_update_count_max(1)
            .with_update_time_min(1)
            .with_update_time_max(5)
            .build();

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_update = EventSnapshot::from(&event_subscriber);
        let state_before_update = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Update, _>::new(shared));
        assert!(state_machine.is_update());

        send_update_messages(1, request_tx.clone());
        let state_machine = state_machine.next().await.unwrap();

        let state_after_update = state_machine.as_ref().clone();
        let events_after_update = EventSnapshot::from(&event_subscriber);
        assert_after_phase_failure(
            &state_before_update,
            &events_before_update,
            &state_after_update,
            &events_after_update,
        );

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::Update(UpdateError::FetchSeedDict(_))
        ))
    }

    #[tokio::test]
    async fn test_update_to_sum2_seed_dict_none() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Update phase
        // 2. accept 1 update message
        // 3. fetch seed dict (no storage error but the seed dict is None)
        // 4. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been invalidated
        // - the seed dict has been broadcasted
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_local_seed_dict()
            .times(1)
            .returning(move |_, _| Ok(LocalSeedDictAdd(Ok(()))));
        cs.expect_seed_dict().return_once(move || Ok(None));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_update_count_min(1)
            .with_update_count_max(1)
            .with_update_time_min(1)
            .with_update_time_max(5)
            .build();

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_update = EventSnapshot::from(&event_subscriber);
        let state_before_update = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Update, _>::new(shared));
        assert!(state_machine.is_update());

        send_update_messages(1, request_tx.clone());
        let state_machine = state_machine.next().await.unwrap();

        let state_after_update = state_machine.as_ref().clone();
        let events_after_update = EventSnapshot::from(&event_subscriber);
        assert_after_phase_failure(
            &state_before_update,
            &events_before_update,
            &state_after_update,
            &events_after_update,
        );

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::Update(UpdateError::NoSeedDict)
        ))
    }

    #[tokio::test]
    async fn test_aggregation_error() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Update phase
        // 2. reject 3 update messages (validation of the models fail due to an invalid length)
        // 3. accept 3 update messages
        // 4. fetch seed dict
        // 5. broadcast seed dict
        // 6. move into sum2 phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been invalidated
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_local_seed_dict()
            .times(3)
            .returning(move |_, _| Ok(LocalSeedDictAdd(Ok(()))));
        cs.expect_seed_dict()
            .return_once(move || Ok(Some(SeedDict::new())));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_update_count_min(3)
            .with_update_count_max(3)
            .with_update_time_min(1)
            .build();

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_update = EventSnapshot::from(&event_subscriber);
        let state_before_update = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Update, _>::new(shared));
        assert!(state_machine.is_update());

        send_update_messages_with_model(3, request_tx.clone(), create_mask(2, 1));
        send_update_messages(3, request_tx.clone());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_update = state_machine.as_ref().clone();
        let events_after_update = EventSnapshot::from(&event_subscriber);
        assert_after_phase_success(
            &state_before_update,
            &events_before_update,
            &state_after_update,
            &events_after_update,
        );

        assert!(state_machine.is_sum2());
    }

    #[tokio::test]
    async fn test_rejected_messages_pet_error() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Update phase
        // 2. reject 3 update messages (pet error LocalSeedDictAddError::LengthMisMatch)
        // 3. phase should timeout
        // 4. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been invalidated
        // - the seed dict has been broadcasted
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_local_seed_dict()
            .times(3)
            .returning(move |_, _| {
                Ok(LocalSeedDictAdd(Err(LocalSeedDictAddError::LengthMisMatch)))
            });
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_update_count_min(3)
            .with_update_count_max(3)
            .with_update_time_min(0)
            .with_update_time_max(2)
            .build();

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_update = EventSnapshot::from(&event_subscriber);
        let state_before_update = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Update, _>::new(shared));
        assert!(state_machine.is_update());

        send_update_messages(3, request_tx.clone());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_update = state_machine.as_ref().clone();
        let events_after_update = EventSnapshot::from(&event_subscriber);
        assert_after_phase_failure(
            &state_before_update,
            &events_before_update,
            &state_after_update,
            &events_after_update,
        );

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::PhaseTimeout(_)
        ))
    }
}
