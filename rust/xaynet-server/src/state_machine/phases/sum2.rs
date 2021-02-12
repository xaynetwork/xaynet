use async_trait::async_trait;
use tracing::info;

use crate::{
    state_machine::{
        events::DictionaryUpdate,
        phases::{Handler, Phase, PhaseError, PhaseName, PhaseState, Shared, Unmask},
        requests::{RequestError, StateMachineRequest, Sum2Request},
        StateMachine,
    },
    storage::Storage,
};
use xaynet_core::{
    mask::{Aggregation, MaskObject},
    SumParticipantPublicKey,
};

/// The sum2 state.
#[derive(Debug)]
pub struct Sum2 {
    /// The aggregator for masked models.
    model_agg: Aggregation,
}

#[async_trait]
impl<T> Phase<T> for PhaseState<Sum2, T>
where
    T: Storage,
    Self: Handler,
{
    const NAME: PhaseName = PhaseName::Sum2;

    async fn process(&mut self) -> Result<(), PhaseError> {
        self.process(self.shared.state.sum2).await
    }

    fn broadcast(&mut self) {
        info!("broadcasting invalidation of sum dictionary");
        self.shared
            .events
            .broadcast_sum_dict(DictionaryUpdate::Invalidate);

        info!("broadcasting invalidation of seed dictionary");
        self.shared
            .events
            .broadcast_seed_dict(DictionaryUpdate::Invalidate);
    }

    async fn next(self) -> Option<StateMachine<T>> {
        Some(PhaseState::<Unmask, _>::new(self.shared, self.private.model_agg).into())
    }
}

#[async_trait]
impl<T> Handler for PhaseState<Sum2, T>
where
    T: Storage,
{
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError> {
        if let StateMachineRequest::Sum2(Sum2Request {
            participant_pk,
            model_mask,
        }) = req
        {
            self.update_mask_dict(participant_pk, model_mask).await
        } else {
            Err(RequestError::MessageRejected)
        }
    }
}

impl<T> PhaseState<Sum2, T> {
    /// Creates a new sum2 state.
    pub fn new(shared: Shared<T>, model_agg: Aggregation) -> Self {
        Self {
            private: Sum2 { model_agg },
            shared,
        }
    }
}

impl<T> PhaseState<Sum2, T>
where
    T: Storage,
{
    /// Updates the mask dict with a sum2 participant request.
    async fn update_mask_dict(
        &mut self,
        participant_pk: SumParticipantPublicKey,
        model_mask: MaskObject,
    ) -> Result<(), RequestError> {
        self.shared
            .store
            .incr_mask_score(&participant_pk, &model_mask)
            .await?
            .into_inner()
            .map_err(RequestError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use xaynet_core::{SeedDict, SumDict};

    use crate::{
        state_machine::{
            coordinator::CoordinatorState,
            events::{DictionaryUpdate, EventPublisher, EventSubscriber, ModelUpdate},
            tests::{
                utils::{
                    assert_event_updated,
                    enable_logging,
                    init_shared,
                    send_sum2_messages,
                    EventSnapshot,
                },
                CoordinatorStateBuilder,
                EventBusBuilder,
            },
        },
        storage::{
            tests::{utils::create_global_model, MockCoordinatorStore, MockModelStore},
            MaskScoreIncr,
            MaskScoreIncrError,
            Store,
        },
    };

    fn events_from_update_phase(state: &CoordinatorState) -> (EventPublisher, EventSubscriber) {
        EventBusBuilder::new(state)
            .broadcast_phase(PhaseName::Update)
            .broadcast_sum_dict(DictionaryUpdate::New(Arc::new(SumDict::new())))
            .broadcast_seed_dict(DictionaryUpdate::New(Arc::new(SeedDict::new())))
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
        assert_event_updated(&events_after.sum_dict, &events_before.sum_dict);
        assert_event_updated(&events_after.seed_dict, &events_before.seed_dict);
        assert_eq!(events_after.sum_dict.event, DictionaryUpdate::Invalidate);
        assert_eq!(events_after.seed_dict.event, DictionaryUpdate::Invalidate);
        assert_eq!(events_after.keys, events_before.keys);
        assert_eq!(events_after.params, events_before.params);
        assert_eq!(events_after.phase.event, PhaseName::Sum2);
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
        assert_eq!(events_after.phase.event, PhaseName::Sum2);
        assert_eq!(events_after.sum_dict, events_before.sum_dict);
        assert_eq!(events_after.seed_dict, events_before.seed_dict);
        assert_eq!(events_after.model, events_before.model);
    }

    #[tokio::test]
    async fn test_sum2_to_unmask_phase() {
        // No Storage errors
        // lets pretend we come from the update phase
        //
        // What should happen:
        // 1. broadcast Sum2 phase
        // 2. accept 10 sum2 messages
        // 3. broadcast invalidation of sum and seed dict
        // 4. move into unmask phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - events have been broadcasted (except phase event and invalidation
        //   event of sum and seed dict)
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_incr_mask_score()
            .times(10)
            .returning(move |_, _| Ok(MaskScoreIncr(Ok(()))));

        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_sum2_count_min(10)
            .with_sum2_count_max(10)
            .with_sum2_time_min(1)
            .build();

        let (event_publisher, event_subscriber) = events_from_update_phase(&state);
        let events_before_sum2 = EventSnapshot::from(&event_subscriber);
        let state_before_sum2 = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let agg = Aggregation::new(
            state_before_sum2.round_params.mask_config,
            state_before_sum2.round_params.model_length,
        );
        let state_machine = StateMachine::from(PhaseState::<Sum2, _>::new(shared, agg));
        assert!(state_machine.is_sum2());

        send_sum2_messages(10, request_tx.clone());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum2 = state_machine.as_ref().clone();
        let events_after_sum2 = EventSnapshot::from(&event_subscriber);
        assert_after_phase_success(
            &state_before_sum2,
            &events_before_sum2,
            &state_after_sum2,
            &events_after_sum2,
        );

        assert!(state_machine.is_unmask());
    }

    #[tokio::test]
    async fn test_rejected_messages_pet_error() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Sum2 phase
        // 2. reject 3 sum2 messages (pet error MaskScoreIncrError::UnknownSumPk)
        // 3. phase should timeout
        // 4. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated
        // - the sum dict has been invalidated
        // - the seed dict has been invalidated
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_incr_mask_score()
            .times(3)
            .returning(move |_, _| Ok(MaskScoreIncr(Err(MaskScoreIncrError::UnknownSumPk))));
        let store = Store::new(cs, MockModelStore::new());
        let state = CoordinatorStateBuilder::new()
            .with_round_id(1)
            .with_sum2_count_min(3)
            .with_sum2_count_max(3)
            .with_sum2_time_min(0)
            .with_sum2_time_max(2)
            .build();

        let (event_publisher, event_subscriber) = events_from_update_phase(&state);
        let events_before_sum2 = EventSnapshot::from(&event_subscriber);
        let state_before_sum2 = state.clone();

        let (shared, request_tx) = init_shared(state, store, event_publisher);
        let agg = Aggregation::new(
            state_before_sum2.round_params.mask_config,
            state_before_sum2.round_params.model_length,
        );
        let state_machine = StateMachine::from(PhaseState::<Sum2, _>::new(shared, agg));
        assert!(state_machine.is_sum2());

        send_sum2_messages(3, request_tx.clone());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum2 = state_machine.as_ref().clone();
        let events_after_sum2 = EventSnapshot::from(&event_subscriber);
        assert_after_phase_failure(
            &state_before_sum2,
            &events_before_sum2,
            &state_after_sum2,
            &events_after_sum2,
        );

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::PhaseTimeout(_)
        ))
    }
}
