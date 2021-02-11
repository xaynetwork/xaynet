use std::sync::Arc;

use async_trait::async_trait;
use displaydoc::Display;
use thiserror::Error;
use tracing::info;

use crate::{
    state_machine::{
        events::DictionaryUpdate,
        phases::{Handler, Phase, PhaseError, PhaseName, PhaseState, Shared, Update},
        requests::{RequestError, StateMachineRequest, SumRequest},
        StateMachine,
    },
    storage::{Storage, StorageError},
};
use xaynet_core::{SumDict, SumParticipantEphemeralPublicKey, SumParticipantPublicKey};

/// Errors which can occur during the sum phase.
#[derive(Debug, Display, Error)]
pub enum SumError {
    /// Sum dictionary does not exists.
    NoSumDict,
    /// Fetching sum dictionary failed: {0}.
    FetchSumDict(StorageError),
}

/// The sum state.
#[derive(Debug)]
pub struct Sum {
    /// The sum dictionary which gets assembled during the sum phase.
    sum_dict: Option<SumDict>,
}

#[async_trait]
impl<T> Phase<T> for PhaseState<Sum, T>
where
    T: Storage,
    Self: Handler,
{
    const NAME: PhaseName = PhaseName::Sum;

    async fn process(&mut self) -> Result<(), PhaseError> {
        self.process(self.shared.state.sum).await?;
        self.sum_dict().await?;

        Ok(())
    }

    fn broadcast(&mut self) {
        info!("broadcasting sum dictionary");
        let sum_dict = self
            .private
            .sum_dict
            .take()
            .expect("unreachable: never fails when `broadcast()` is called after `process()`");
        self.shared
            .events
            .broadcast_sum_dict(DictionaryUpdate::New(Arc::new(sum_dict)));
    }

    async fn next(self) -> Option<StateMachine<T>> {
        Some(PhaseState::<Update, _>::new(self.shared).into())
    }
}

#[async_trait]
impl<T> Handler for PhaseState<Sum, T>
where
    T: Storage,
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
}

impl<T> PhaseState<Sum, T> {
    /// Creates a new sum state.
    pub fn new(shared: Shared<T>) -> Self {
        Self {
            private: Sum { sum_dict: None },
            shared,
        }
    }
}

impl<T> PhaseState<Sum, T>
where
    T: Storage,
{
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

    /// Gets the sum dict from the store.
    async fn sum_dict(&mut self) -> Result<(), SumError> {
        self.private.sum_dict = self
            .shared
            .store
            .sum_dict()
            .await
            .map_err(SumError::FetchSumDict)?
            .ok_or(SumError::NoSumDict)?
            .into();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::anyhow;
    use tokio::time::{timeout, Duration};
    use xaynet_core::SumDict;

    use crate::{
        state_machine::{
            coordinator::CoordinatorState,
            events::{EventPublisher, EventSubscriber, ModelUpdate},
            tests::{
                utils::{
                    assert_event_updated,
                    enable_logging,
                    init_shared,
                    send_sum2_messages,
                    send_sum_messages,
                    send_update_messages,
                    EventSnapshot,
                },
                CoordinatorStateBuilder,
                EventBusBuilder,
            },
        },
        storage::{
            tests::{utils::create_global_model, MockCoordinatorStore, MockModelStore},
            Store,
            SumPartAdd,
            SumPartAddError,
        },
    };

    fn events_from_idle_phase(state: &CoordinatorState) -> (EventPublisher, EventSubscriber) {
        EventBusBuilder::new(state)
            .broadcast_phase(PhaseName::Idle)
            .broadcast_sum_dict(DictionaryUpdate::Invalidate)
            .broadcast_seed_dict(DictionaryUpdate::Invalidate)
            .broadcast_model(ModelUpdate::New(Arc::new(create_global_model(10))))
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
        assert_event_updated(&events_after.sum_dict, &events_before.sum_dict);
        assert_eq!(events_after.keys, events_before.keys);
        assert_eq!(events_after.params, events_before.params);
        assert_eq!(events_after.phase.event, PhaseName::Sum);
        assert_eq!(events_after.seed_dict, events_before.seed_dict);
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
        assert_eq!(events_after.phase.event, PhaseName::Sum);
        assert_eq!(events_after.sum_dict, events_before.sum_dict);
        assert_eq!(events_after.seed_dict, events_before.seed_dict);
        assert_eq!(events_after.model, events_before.model);
    }

    #[tokio::test]
    async fn test_sum_to_update_phase() {
        // No Storage errors
        // lets pretend we come from the sum phase
        //
        // What should happen:
        // 1. broadcast Sum phase
        // 2. accept 10 sum messages
        // 3. fetch sum dict
        // 4. broadcast sum dict
        // 5. move into update phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_sum_participant()
            .times(10)
            .returning(move |_, _| Ok(SumPartAdd(Ok(()))));
        cs.expect_sum_dict()
            .return_once(move || Ok(Some(SumDict::new())));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_sum_count_min(10)
            .with_sum_count_max(10)
            .with_sum_time_min(1)
            .build();

        let (event_publisher, event_subscriber) = events_from_idle_phase(&state);
        let events_before_sum = EventSnapshot::from(&event_subscriber);
        let state_before_sum = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Sum, _>::new(shared));
        assert!(state_machine.is_sum());

        send_sum_messages(10, request_tx.clone());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum = state_machine.shared_state_as_ref().clone();
        let events_after_sum = EventSnapshot::from(&event_subscriber);
        assert_after_phase_success(
            &state_before_sum,
            &events_before_sum,
            &state_after_sum,
            &events_after_sum,
        );

        assert!(state_machine.is_update());
    }

    #[tokio::test]
    async fn test_sum_phase_timeout() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Sum phase
        // 2. phase should timeout
        // 3. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been fetched
        // - the sum dict has been broadcasted
        enable_logging();

        let store = Store::new(MockCoordinatorStore::new(), MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_sum_time_min(1)
            .with_sum_time_max(2)
            .build();

        let (event_publisher, event_subscriber) = events_from_idle_phase(&state);
        let events_before_sum = EventSnapshot::from(&event_subscriber);
        let state_before_sum = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Sum, _>::new(shared));
        assert!(state_machine.is_sum());

        let state_machine = timeout(Duration::from_secs(4), state_machine.next())
            .await
            .unwrap()
            .unwrap();

        let state_after_sum = state_machine.shared_state_as_ref().clone();
        let events_after_sum = EventSnapshot::from(&event_subscriber);
        assert_after_phase_failure(
            &state_before_sum,
            &events_before_sum,
            &state_after_sum,
            &events_after_sum,
        );

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::PhaseTimeout(_)
        ))
    }

    #[tokio::test]
    async fn test_rejected_messages() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Sum phase
        // 2. accept 7 sum messages
        // 3. reject 5 update and 2 sum2 messages
        // 4. fetch sum dict
        // 5. broadcast sum dict
        // 6. move into update phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_sum_participant()
            .times(7)
            .returning(move |_, _| Ok(SumPartAdd(Ok(()))));
        cs.expect_sum_dict()
            .return_once(move || Ok(Some(SumDict::new())));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_sum_count_min(7)
            .with_sum_count_max(7)
            .build();

        let (event_publisher, event_subscriber) = events_from_idle_phase(&state);
        let events_before_sum = EventSnapshot::from(&event_subscriber);
        let state_before_sum = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Sum, _>::new(shared));
        assert!(state_machine.is_sum());

        send_update_messages(3, request_tx.clone());
        send_sum2_messages(5, request_tx.clone());
        send_sum_messages(7, request_tx.clone());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum = state_machine.shared_state_as_ref().clone();
        let events_after_sum = EventSnapshot::from(&event_subscriber);
        assert_after_phase_success(
            &state_before_sum,
            &events_before_sum,
            &state_after_sum,
            &events_after_sum,
        );

        assert!(state_machine.is_update());
    }

    #[tokio::test]
    async fn test_discarded_messages() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Sum phase
        // 2. accept 5 sum messages
        // 3. discard 5 sum messages
        // 4. fetch sum dict
        // 5. broadcast sum dict
        // 6. move into update phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_sum_participant()
            .times(5)
            .returning(move |_, _| Ok(SumPartAdd(Ok(()))));
        cs.expect_sum_dict()
            .return_once(move || Ok(Some(SumDict::new())));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_sum_count_min(5)
            .with_sum_count_max(5)
            .with_sum_time_min(5)
            .with_sum_time_max(10)
            .build();

        let (event_publisher, event_subscriber) = events_from_idle_phase(&state);
        let events_before_sum = EventSnapshot::from(&event_subscriber);
        let state_before_sum = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Sum, _>::new(shared));
        assert!(state_machine.is_sum());

        send_sum_messages(10, request_tx.clone());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum = state_machine.shared_state_as_ref().clone();
        let events_after_sum = EventSnapshot::from(&event_subscriber);
        assert_after_phase_success(
            &state_before_sum,
            &events_before_sum,
            &state_after_sum,
            &events_after_sum,
        );

        assert!(state_machine.is_update());
    }

    #[tokio::test]
    async fn test_request_channel_is_dropped() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Sum phase
        // 2. request channel is dropped
        // 3. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been fetched
        // - the sum dict has been broadcasted
        enable_logging();

        let store = Store::new(MockCoordinatorStore::new(), MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_sum_count_min(1)
            .with_sum_count_max(1)
            .with_sum_time_min(1)
            .with_sum_time_max(5)
            .build();

        let (event_publisher, event_subscriber) = events_from_idle_phase(&state);
        let events_before_sum = EventSnapshot::from(&event_subscriber);
        let state_before_sum = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Sum, _>::new(shared));
        assert!(state_machine.is_sum());

        drop(request_tx);
        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum = state_machine.shared_state_as_ref().clone();
        let events_after_sum = EventSnapshot::from(&event_subscriber);
        assert_after_phase_failure(
            &state_before_sum,
            &events_before_sum,
            &state_after_sum,
            &events_after_sum,
        );

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::RequestChannel(_)
        ))
    }

    #[tokio::test]
    async fn test_sum_to_update_fetch_sum_dict_failed() {
        // Storage errors
        // - sum_dict fails
        //
        // What should happen:
        // 1. broadcast Sum phase
        // 2. accept 1 sum message
        // 3. fetch sum dict (fails)
        // 4. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been broadcasted
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_sum_participant()
            .times(1)
            .returning(move |_, _| Ok(SumPartAdd(Ok(()))));
        cs.expect_sum_dict().return_once(move || Err(anyhow!("")));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_sum_count_min(1)
            .with_sum_count_max(1)
            .with_sum_time_min(1)
            .with_sum_time_max(5)
            .build();

        let (event_publisher, event_subscriber) = events_from_idle_phase(&state);
        let events_before_sum = EventSnapshot::from(&event_subscriber);
        let state_before_sum = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Sum, _>::new(shared));
        assert!(state_machine.is_sum());

        send_sum_messages(1, request_tx.clone());
        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum = state_machine.shared_state_as_ref().clone();
        let events_after_sum = EventSnapshot::from(&event_subscriber);
        assert_after_phase_failure(
            &state_before_sum,
            &events_before_sum,
            &state_after_sum,
            &events_after_sum,
        );

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::Sum(SumError::FetchSumDict(_))
        ))
    }

    #[tokio::test]
    async fn test_sum_to_update_sum_dict_none() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Sum phase
        // 2. accept 1 sum message
        // 3. fetch sum dict (no storage error but the sum dict is None)
        // 4. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been broadcasted
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_sum_participant()
            .times(1)
            .returning(move |_, _| Ok(SumPartAdd(Ok(()))));
        cs.expect_sum_dict().return_once(move || Ok(None));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_sum_count_min(1)
            .with_sum_count_max(1)
            .with_sum_time_min(1)
            .with_sum_time_max(5)
            .build();

        let (event_publisher, event_subscriber) = events_from_idle_phase(&state);
        let events_before_sum = EventSnapshot::from(&event_subscriber);
        let state_before_sum = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Sum, _>::new(shared));
        assert!(state_machine.is_sum());

        send_sum_messages(1, request_tx.clone());
        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum = state_machine.shared_state_as_ref().clone();
        let events_after_sum = EventSnapshot::from(&event_subscriber);
        assert_after_phase_failure(
            &state_before_sum,
            &events_before_sum,
            &state_after_sum,
            &events_after_sum,
        );

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::Sum(SumError::NoSumDict)
        ))
    }

    #[tokio::test]
    async fn test_rejected_messages_pet_error() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Sum phase
        // 2. reject 3 sum messages (pet error SumPartAddError::AlreadyExists)
        // 3. phase should timeout
        // 4. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been fetched
        // - the sum dict has been broadcasted
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_add_sum_participant()
            .times(3)
            .returning(move |_, _| Ok(SumPartAdd(Err(SumPartAddError::AlreadyExists))));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_sum_count_min(3)
            .with_sum_count_max(3)
            .with_sum_time_min(0)
            .with_sum_time_max(2)
            .build();

        let (event_publisher, event_subscriber) = events_from_idle_phase(&state);
        let events_before_sum = EventSnapshot::from(&event_subscriber);
        let state_before_sum = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Sum, _>::new(shared));
        assert!(state_machine.is_sum());

        send_sum_messages(3, request_tx.clone());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum = state_machine.shared_state_as_ref().clone();
        let events_after_sum = EventSnapshot::from(&event_subscriber);
        assert_after_phase_failure(
            &state_before_sum,
            &events_before_sum,
            &state_after_sum,
            &events_after_sum,
        );

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::PhaseTimeout(_)
        ))
    }

    // #[tokio::test]
    // async fn test_sum_phase_publish_after_purge() {
    //     // Publish sum dict after purging all remaining messages.
    //     enable_logging();

    //     let mut cs = MockCoordinatorStore::new();
    //     cs.expect_add_sum_participant()
    //         .returning(move |_, _| Ok(SumPartAdd(Ok(()))));
    //     cs.expect_sum_dict()
    //         .return_once(move || Ok(Some(SumDict::new())));

    //     let store = Store::new(cs, MockModelStore::new());
    //     let state = CoordinatorStateBuilder::new()
    //         .with_round_id(1)
    //         .with_sum_count_min(2)
    //         .with_sum_count_max(500)
    //         .with_sum_time_min(0)
    //         .build();

    //     let (event_publisher, event_subscriber) = events_from_idle_phase(&state);

    //     let (shared, request_tx) = init_shared(state, store, event_publisher);
    //     let state_machine = StateMachine::from(PhaseState::<Sum, _>::new(shared));
    //     assert!(state_machine.is_sum());

    //     let (mut ready, latch) = Readiness::new();

    //     send_sum_messages_with_latch(1000, request_tx.clone(), latch);

    //     let mut sum_dict_listener = event_subscriber.sum_dict_listener();
    //     sum_dict_listener.changed().await.unwrap();
    //     tokio::time::sleep(Duration::from_secs(10)).await;
    //     tokio::select! {
    //         // TODO: purge_outdated_requests blocks the current thread (we should fix that)
    //         // and sum_dict_listener.changed() would always be executed after
    //         // state_machine.next(). The test always passes although it shouldn't
    //         // therefore we need to spawn it here to run the state machine on a separate
    //         // thread
    //         //
    //         // Further more we suffer from the https://github.com/tokio-rs/tokio/issues/3350
    //         // issue in request_tx::try_recv(). We fill the request channel with 1000
    //         // before we start the machine. Nevertheless, the message purging stops after
    //         // around 134 messages.
    //         _ = state_machine.next() => {
    //             panic!("state did no run successfully")
    //         }
    //         _ = sum_dict_listener.changed() => {
    //             panic!("sum dict was broadcasted before all requests has been purged")
    //         }
    //         _ = ready.is_ready() => {

    //         }
    //     }
    // }
}
