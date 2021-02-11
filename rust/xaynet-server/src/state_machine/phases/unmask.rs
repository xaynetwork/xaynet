use std::{cmp::Ordering, sync::Arc};

use async_trait::async_trait;
use displaydoc::Display;
use thiserror::Error;
#[cfg(feature = "model-persistence")]
use tracing::warn;
use tracing::{error, info};

use crate::{
    metric,
    metrics::{GlobalRecorder, Measurement},
    state_machine::{
        events::ModelUpdate,
        phases::{Idle, Phase, PhaseError, PhaseName, PhaseState, Shared},
        StateMachine,
    },
    storage::{Storage, StorageError},
};
use xaynet_core::mask::{Aggregation, MaskObject, Model, UnmaskingError};

/// Errors which can occur during the unmask phase.
#[derive(Debug, Display, Error)]
pub enum UnmaskError {
    /// Ambiguous masks were computed by the sum participants.
    AmbiguousMasks,
    /// No mask found.
    NoMask,
    /// Unmasking global model failed: {0}.
    Unmasking(#[from] UnmaskingError),
    /// Fetching best masks failed: {0}.
    FetchBestMasks(#[from] StorageError),
    #[cfg(feature = "model-persistence")]
    /// Saving the global model failed: {0}.
    SaveGlobalModel(crate::storage::StorageError),
    /// Publishing the proof of the global model failed: {0}.
    PublishProof(crate::storage::StorageError),
}

/// The unmask state.
#[derive(Debug)]
pub struct Unmask {
    /// The aggregator for masked models.
    model_agg: Option<Aggregation>,
    /// The global model of the current round.
    global_model: Option<Arc<Model>>,
}

#[async_trait]
impl<T> Phase<T> for PhaseState<Unmask, T>
where
    T: Storage,
{
    const NAME: PhaseName = PhaseName::Unmask;

    async fn process(&mut self) -> Result<(), PhaseError> {
        self.emit_number_of_unique_masks_metrics();
        let best_masks = self.best_masks().await?;
        self.end_round(best_masks).await?;

        #[cfg(feature = "model-persistence")]
        self.save_global_model().await?;
        self.publish_proof().await?;

        Ok(())
    }

    fn broadcast(&mut self) {
        info!("broadcasting the new global model");
        let global_model =
            self.private.global_model.take().expect(
                "unreachable: never fails when `broadcast()` is called after `end_round()`",
            );
        self.shared
            .events
            .broadcast_model(ModelUpdate::New(global_model));
    }

    async fn next(self) -> Option<StateMachine<T>> {
        Some(PhaseState::<Idle, _>::new(self.shared).into())
    }
}

impl<T> PhaseState<Unmask, T> {
    /// Creates a new unmask state.
    pub fn new(shared: Shared<T>, model_agg: Aggregation) -> Self {
        Self {
            private: Unmask {
                model_agg: Some(model_agg),
                global_model: None,
            },
            shared,
        }
    }

    /// Freezes the mask dictionary.
    async fn freeze_mask_dict(
        &mut self,
        mut best_masks: Vec<(MaskObject, u64)>,
    ) -> Result<MaskObject, UnmaskError> {
        let mask = best_masks
            .drain(0..)
            .fold(
                (None, 0),
                |(unique_mask, unique_count), (mask, count)| match unique_count.cmp(&count) {
                    Ordering::Less => (Some(mask), count),
                    Ordering::Greater => (unique_mask, unique_count),
                    Ordering::Equal => (None, unique_count),
                },
            )
            .0
            .ok_or(UnmaskError::AmbiguousMasks)?;

        Ok(mask)
    }

    /// Ends the round by unmasking the global model.
    async fn end_round(&mut self, best_masks: Vec<(MaskObject, u64)>) -> Result<(), UnmaskError> {
        let mask = self.freeze_mask_dict(best_masks).await?;

        // Safe unwrap: State::<Unmask>::new always creates Some(aggregation)
        let model_agg = self.private.model_agg.take().unwrap();

        model_agg
            .validate_unmasking(&mask)
            .map_err(UnmaskError::from)?;
        self.private.global_model = Some(Arc::new(model_agg.unmask(mask)));

        Ok(())
    }
}

impl<T> PhaseState<Unmask, T>
where
    T: Storage,
{
    /// Broadcasts mask metrics.
    fn emit_number_of_unique_masks_metrics(&mut self) {
        if GlobalRecorder::global().is_none() {
            return;
        }

        let mut store = self.shared.store.clone();
        let (round_id, phase_name) = (self.shared.state.round_id, Self::NAME);

        tokio::spawn(async move {
            match store.number_of_unique_masks().await {
                Ok(number_of_masks) => metric!(
                    Measurement::MasksTotalNumber,
                    number_of_masks,
                    ("round_id", round_id),
                    ("phase", phase_name as u8),
                ),
                Err(err) => error!("failed to fetch total number of masks: {}", err),
            };
        });
    }

    /// Gets the two masks with the highest score.
    async fn best_masks(&mut self) -> Result<Vec<(MaskObject, u64)>, UnmaskError> {
        self.shared
            .store
            .best_masks()
            .await
            .map_err(UnmaskError::FetchBestMasks)?
            .ok_or(UnmaskError::NoMask)
    }

    /// Persists the global model to the store.
    #[cfg(feature = "model-persistence")]
    async fn save_global_model(&mut self) -> Result<(), UnmaskError> {
        info!("saving global model");
        let global_model = self
            .private
            .global_model
            .as_ref()
            .expect(
                "unreachable: never fails when `save_global_model()` is called after `end_round()`",
            )
            .as_ref();
        let global_model_id = self
            .shared
            .store
            .set_global_model(
                self.shared.state.round_id,
                &self.shared.state.round_params.seed,
                global_model,
            )
            .await
            .map_err(UnmaskError::SaveGlobalModel)?;
        if let Err(err) = self
            .shared
            .store
            .set_latest_global_model_id(&global_model_id)
            .await
        {
            warn!("failed to update latest global model id: {}", err);
        }

        Ok(())
    }

    /// Publishes proof of the global model.
    async fn publish_proof(&mut self) -> Result<(), UnmaskError> {
        info!("publishing proof of the new global model");
        let global_model = self
            .private
            .global_model
            .as_ref()
            .expect(
                "unreachable: never fails when `save_global_model()` is called after `end_round()`",
            )
            .as_ref();
        self.shared
            .store
            .publish_proof(global_model)
            .await
            .map_err(UnmaskError::PublishProof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use anyhow::anyhow;
    use xaynet_core::{SeedDict, SumDict};

    use crate::{
        state_machine::{
            coordinator::CoordinatorState,
            events::{DictionaryUpdate, EventPublisher, EventSubscriber, ModelUpdate},
            tests::{
                utils::{assert_event_updated, enable_logging, init_shared, EventSnapshot},
                CoordinatorStateBuilder,
                EventBusBuilder,
            },
        },
        storage::{
            tests::{
                utils::{create_global_model, create_mask},
                MockCoordinatorStore,
                MockModelStore,
                MockTrustAnchor,
            },
            Store,
        },
    };

    fn events_from_sum_phase(state: &CoordinatorState) -> (EventPublisher, EventSubscriber) {
        EventBusBuilder::new(state)
            .broadcast_phase(PhaseName::Sum2)
            .broadcast_sum_dict(DictionaryUpdate::New(Arc::new(SumDict::new())))
            .broadcast_seed_dict(DictionaryUpdate::New(Arc::new(SeedDict::new())))
            .broadcast_model(ModelUpdate::New(Arc::new(create_global_model(10))))
            .build()
    }

    fn assert_after_phase_success(
        state_before: &CoordinatorState,
        events_before: &EventSnapshot,
        state_after: &CoordinatorState,
        events_after: &EventSnapshot,
    ) {
        assert_ne!(state_after.round_id, state_before.round_id);
        assert_eq!(state_after.round_params, state_before.round_params);
        assert_eq!(state_after.keys, state_before.keys);
        assert_eq!(state_after.sum, state_before.sum);
        assert_eq!(state_after.update, state_before.update);
        assert_eq!(state_after.sum2, state_before.sum2);

        assert_event_updated(&events_after.phase, &events_before.phase);
        assert_event_updated(&events_after.model, &events_before.model);
        assert_eq!(events_after.keys, events_before.keys);
        assert_eq!(events_after.params, events_before.params);
        assert_eq!(events_after.phase.event, PhaseName::Unmask);
        assert_eq!(events_after.sum_dict, events_before.sum_dict);
        assert_eq!(events_after.seed_dict, events_before.seed_dict);
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
        assert_eq!(events_after.phase.event, PhaseName::Unmask);
        assert_eq!(events_after.sum_dict, events_before.sum_dict);
        assert_eq!(events_after.seed_dict, events_before.seed_dict);
        assert_eq!(events_after.model, events_before.model);
    }

    fn init_aggregator(state: &CoordinatorState) -> Aggregation {
        let mut aggregator = Aggregation::new(
            state.round_params.mask_config,
            state.round_params.model_length,
        );
        aggregator.aggregate(create_mask(state.round_params.model_length, 1));
        aggregator
    }

    #[tokio::test]
    async fn test_unmask_to_idle_phase() {
        // No Storage errors
        // lets pretend we come from the sum2 phase
        //
        // What should happen:
        // 1. broadcast Unmask phase
        // 2  fetch best masks (return only one)
        // 3. unmask the masked global model
        // 4. publish proof
        // 5. broadcast unmasked global model
        // 6. move into idle phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - events have been broadcasted (except phase event and global model)
        enable_logging();

        let state = CoordinatorStateBuilder::new().with_round_id(1).build();
        let model_length = state.round_params.model_length;

        let mut cs = MockCoordinatorStore::new();
        cs.expect_best_masks()
            .returning(move || Ok(Some(vec![(create_mask(model_length, 1), 1)])));
        #[cfg(feature = "model-persistence")]
        {
            cs.expect_set_latest_global_model_id()
                .returning(move |_| Ok(()));
        }
        let ms = {
            #[cfg(not(feature = "model-persistence"))]
            {
                MockModelStore::new()
            }
            #[cfg(feature = "model-persistence")]
            {
                let mut ms = MockModelStore::new();
                ms.expect_set_global_model()
                    .returning(move |_, _, _| Ok("id".to_string()));
                ms
            }
        };

        let store = Store::new(cs, ms);

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_sum2 = EventSnapshot::from(&event_subscriber);
        let state_before_sum2 = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let aggregator = init_aggregator(&state_before_sum2);
        let state_machine = StateMachine::from(PhaseState::<Unmask, _>::new(shared, aggregator));
        assert!(state_machine.is_unmask());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum2 = state_machine.shared_state_as_ref().clone();
        let events_after_sum2 = EventSnapshot::from(&event_subscriber);
        assert_after_phase_success(
            &state_before_sum2,
            &events_before_sum2,
            &state_after_sum2,
            &events_after_sum2,
        );

        assert!(state_machine.is_idle());
    }

    #[tokio::test]
    async fn test_unmask_to_idle_phase_best_masks_fails() {
        // Storage:
        // - best_masks fails
        //
        // What should happen:
        // 1. broadcast Unmask phase
        // 2. fetch best masks (fails)
        // 3. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated/changed
        // - the sum dict has been invalidated
        // - the seed dict has been invalidated
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_best_masks().returning(move || Err(anyhow!("")));

        let store = Store::new(cs, MockModelStore::new());

        let state = CoordinatorStateBuilder::new().with_round_id(1).build();
        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_sum2 = EventSnapshot::from(&event_subscriber);
        let state_before_sum2 = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let aggregator = init_aggregator(&state_before_sum2);
        let state_machine = StateMachine::from(PhaseState::<Unmask, _>::new(shared, aggregator));
        assert!(state_machine.is_unmask());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum2 = state_machine.shared_state_as_ref().clone();
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
            PhaseError::Unmask(UnmaskError::FetchBestMasks(_))
        ))
    }

    #[tokio::test]
    async fn test_unmask_to_idle_phase_no_mask() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Unmask phase
        // 2. fetch best masks (no storage error but the mask vec is None)
        // 3. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated/changed
        // - the sum dict has been invalidated
        // - the seed dict has been invalidated
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_best_masks().returning(move || Ok(None));

        let store = Store::new(cs, MockModelStore::new());

        let state = CoordinatorStateBuilder::new().with_round_id(1).build();
        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_sum2 = EventSnapshot::from(&event_subscriber);
        let state_before_sum2 = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let aggregator = init_aggregator(&state_before_sum2);
        let state_machine = StateMachine::from(PhaseState::<Unmask, _>::new(shared, aggregator));
        assert!(state_machine.is_unmask());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum2 = state_machine.shared_state_as_ref().clone();
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
            PhaseError::Unmask(UnmaskError::NoMask)
        ))
    }

    #[tokio::test]
    async fn test_unmask_to_idle_phase_ambiguous_masks() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Unmask phase
        // 2. fetch best masks
        // 3. unmask the masked global model (fails because of ambiguous masks)
        // 4. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated/changed
        // - the sum dict has been invalidated
        // - the seed dict has been invalidated
        enable_logging();

        let state = CoordinatorStateBuilder::new().with_round_id(1).build();
        let model_length = state.round_params.model_length;

        let mut cs = MockCoordinatorStore::new();
        cs.expect_best_masks().returning(move || {
            Ok(Some(vec![
                (create_mask(model_length, 1), 1),
                (create_mask(model_length, 2), 1),
            ]))
        });

        let store = Store::new(cs, MockModelStore::new());

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_sum2 = EventSnapshot::from(&event_subscriber);
        let state_before_sum2 = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let aggregator = init_aggregator(&state_before_sum2);
        let state_machine = StateMachine::from(PhaseState::<Unmask, _>::new(shared, aggregator));
        assert!(state_machine.is_unmask());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum2 = state_machine.shared_state_as_ref().clone();
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
            PhaseError::Unmask(UnmaskError::AmbiguousMasks)
        ))
    }

    #[tokio::test]
    async fn test_unmask_to_idle_phase_validate_unmasking_fails() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Unmask phase
        // 2. fetch best masks
        // 3. unmask the masked global model (fails because of validate unmasking error)
        // 4. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated/changed
        // - the sum dict has been invalidated
        // - the seed dict has been invalidated
        enable_logging();

        let state = CoordinatorStateBuilder::new().with_round_id(1).build();
        let model_length = state.round_params.model_length;

        let mut cs = MockCoordinatorStore::new();
        cs.expect_best_masks()
            .returning(move || Ok(Some(vec![(create_mask(model_length, 1), 1)])));

        let store = Store::new(cs, MockModelStore::new());

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_sum2 = EventSnapshot::from(&event_subscriber);
        let state_before_sum2 = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let aggregator = Aggregation::new(
            state_before_sum2.round_params.mask_config,
            state_before_sum2.round_params.model_length,
        );
        let state_machine = StateMachine::from(PhaseState::<Unmask, _>::new(shared, aggregator));
        assert!(state_machine.is_unmask());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum2 = state_machine.shared_state_as_ref().clone();
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
            PhaseError::Unmask(UnmaskError::Unmasking(UnmaskingError::NoModel))
        ))
    }

    #[tokio::test]
    async fn test_unmask_to_idle_phase_publish_proof_fails() {
        // TODO: we should set the latest_global_model_id only if the
        // the proof was successfully published
        //
        // Why? If the coordinator were to restart after this phase, they would
        // be using a model that has no evidence and therefore cannot be validated
        // by the user.

        // Storage:
        // - publish_proof fails
        //
        // What should happen:
        // 1. broadcast Unmask phase
        // 2. fetch best masks
        // 3. unmask the masked global model
        // 4. save global model and model id (model-persistence feature)
        // 5. publish proof (fails)
        // 6. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated/changed
        // - the sum dict has been invalidated
        // - the seed dict has been invalidated
        enable_logging();

        let state = CoordinatorStateBuilder::new().with_round_id(1).build();
        let model_length = state.round_params.model_length;

        let mut cs = MockCoordinatorStore::new();
        cs.expect_best_masks()
            .returning(move || Ok(Some(vec![(create_mask(model_length, 1), 1)])));
        #[cfg(feature = "model-persistence")]
        {
            cs.expect_set_latest_global_model_id()
                .returning(move |_| Ok(()));
        }
        let ms = {
            #[cfg(not(feature = "model-persistence"))]
            {
                MockModelStore::new()
            }
            #[cfg(feature = "model-persistence")]
            {
                let mut ms = MockModelStore::new();
                ms.expect_set_global_model()
                    .returning(move |_, _, _| Ok("id".to_string()));
                ms
            }
        };
        let mut ta = MockTrustAnchor::new();
        ta.expect_publish_proof()
            .returning(move |_| Err(anyhow!("")));

        let store = Store::new_with_trust_anchor(cs, ms, ta);

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_sum2 = EventSnapshot::from(&event_subscriber);
        let state_before_sum2 = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let aggregator = init_aggregator(&state_before_sum2);
        let state_machine = StateMachine::from(PhaseState::<Unmask, _>::new(shared, aggregator));
        assert!(state_machine.is_unmask());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum2 = state_machine.shared_state_as_ref().clone();
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
            PhaseError::Unmask(UnmaskError::PublishProof(_))
        ))
    }

    #[cfg(feature = "model-persistence")]
    #[tokio::test]
    async fn test_unmask_to_idle_phase_set_global_model_fails() {
        // Storage:
        // - set_global_model fails
        //
        // What should happen:
        // 1. broadcast Unmask phase
        // 2. fetch best masks
        // 3. unmask the masked global model
        // 4. save global model (fails)
        // 5. move into error phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - the global model has been invalidated/changed
        // - the sum dict has been invalidated
        // - the seed dict has been invalidated
        enable_logging();

        let state = CoordinatorStateBuilder::new().with_round_id(1).build();
        let model_length = state.round_params.model_length;

        let mut cs = MockCoordinatorStore::new();
        cs.expect_best_masks()
            .returning(move || Ok(Some(vec![(create_mask(model_length, 1), 1)])));
        cs.expect_set_latest_global_model_id()
            .returning(move |_| Ok(()));

        let mut ms = MockModelStore::new();
        ms.expect_set_global_model()
            .returning(move |_, _, _| Err(anyhow!("")));

        let store = Store::new(cs, ms);

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_sum2 = EventSnapshot::from(&event_subscriber);
        let state_before_sum2 = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let aggregator = init_aggregator(&state_before_sum2);
        let state_machine = StateMachine::from(PhaseState::<Unmask, _>::new(shared, aggregator));
        assert!(state_machine.is_unmask());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum2 = state_machine.shared_state_as_ref().clone();
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
            PhaseError::Unmask(UnmaskError::SaveGlobalModel(_))
        ))
    }

    #[cfg(feature = "model-persistence")]
    #[tokio::test]
    async fn test_unmask_to_idle_phase_set_global_model_id_fails() {
        // Storage:
        // - set_latest_global_model_id fails
        //
        // What should happen:
        // 1. broadcast Unmask phase
        // 2. fetch best masks
        // 3. unmask the masked global model
        // 4. save global model and model id (fails)
        // 5. publish proof
        // 6. broadcast unmasked global model
        // 7. move into idle phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - events have been broadcasted (except phase event and global model)
        enable_logging();

        let state = CoordinatorStateBuilder::new().with_round_id(1).build();
        let model_length = state.round_params.model_length;

        let mut cs = MockCoordinatorStore::new();
        cs.expect_best_masks()
            .returning(move || Ok(Some(vec![(create_mask(model_length, 1), 1)])));
        cs.expect_set_latest_global_model_id()
            .returning(move |_| Err(anyhow!("")));

        let mut ms = MockModelStore::new();
        ms.expect_set_global_model()
            .returning(move |_, _, _| Ok("id".to_string()));

        let store = Store::new(cs, ms);

        let (event_publisher, event_subscriber) = events_from_sum_phase(&state);
        let events_before_sum2 = EventSnapshot::from(&event_subscriber);
        let state_before_sum2 = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let aggregator = init_aggregator(&state_before_sum2);
        let state_machine = StateMachine::from(PhaseState::<Unmask, _>::new(shared, aggregator));
        assert!(state_machine.is_unmask());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_sum2 = state_machine.shared_state_as_ref().clone();
        let events_after_sum2 = EventSnapshot::from(&event_subscriber);
        assert_after_phase_success(
            &state_before_sum2,
            &events_before_sum2,
            &state_after_sum2,
            &events_after_sum2,
        );

        assert!(state_machine.is_idle());
    }
}
