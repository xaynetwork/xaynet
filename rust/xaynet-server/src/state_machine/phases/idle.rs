use async_trait::async_trait;
use displaydoc::Display;
use sodiumoxide::crypto::hash::sha256;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::{
    metric,
    metrics::Measurement,
    state_machine::{
        phases::{Phase, PhaseError, PhaseName, PhaseState, Shared, Sum},
        StateMachine,
    },
    storage::{Storage, StorageError},
};
use xaynet_core::{
    common::RoundSeed,
    crypto::{ByteObject, EncryptKeyPair, SigningKeySeed},
};

/// Errors which can occur during the idle phase.
#[derive(Debug, Display, Error)]
pub enum IdleError {
    /// Setting the coordinator state failed: {0}.
    SetCoordinatorState(StorageError),
    /// Deleting the dictionaries failed: {0}.
    DeleteDictionaries(StorageError),
}

/// The idle state.
#[derive(Debug)]
pub struct Idle;

#[async_trait]
impl<T> Phase<T> for PhaseState<Idle, T>
where
    T: Storage,
{
    const NAME: PhaseName = PhaseName::Idle;

    async fn process(&mut self) -> Result<(), PhaseError> {
        self.delete_dicts().await?;

        self.gen_round_keypair();
        self.update_round_probabilities();
        self.update_round_seed();

        self.set_coordinator_state().await?;

        Ok(())
    }

    fn broadcast(&mut self) {
        self.broadcast_keys();
        self.broadcast_params();
        self.broadcast_metrics();
    }

    async fn next(self) -> Option<StateMachine<T>> {
        Some(PhaseState::<Sum, _>::new(self.shared).into())
    }
}

impl<T> PhaseState<Idle, T> {
    /// Creates a new idle state.
    pub fn new(mut shared: Shared<T>) -> Self {
        // Since some events are emitted very early, the round id must
        // be correct when the idle phase starts. Therefore, we update
        // it here, when instantiating the idle PhaseState.
        shared.set_round_id(shared.round_id() + 1);
        debug!("new round ID = {}", shared.round_id());
        Self {
            private: Idle,
            shared,
        }
    }

    /// Updates the participant probabilities round parameters.
    fn update_round_probabilities(&mut self) {
        info!("updating round probabilities");
        warn!("round probabilities stay constant, no update strategy implemented yet");
    }

    /// Updates the seed round parameter.
    fn update_round_seed(&mut self) {
        info!("updating round seed");
        // Safe unwrap: `sk` and `seed` have same number of bytes
        let (_, sk) =
            SigningKeySeed::from_slice_unchecked(self.shared.state.keys.secret.as_slice())
                .derive_signing_key_pair();
        let signature = sk.sign_detached(
            &[
                self.shared.state.round_params.seed.as_slice(),
                &self.shared.state.round_params.sum.to_le_bytes(),
                &self.shared.state.round_params.update.to_le_bytes(),
            ]
            .concat(),
        );
        // Safe unwrap: the length of the hash is 32 bytes
        self.shared.state.round_params.seed =
            RoundSeed::from_slice_unchecked(sha256::hash(signature.as_slice()).as_ref());
    }

    /// Generates fresh round credentials.
    fn gen_round_keypair(&mut self) {
        info!("updating the keys");
        self.shared.state.keys = EncryptKeyPair::generate();
        self.shared.state.round_params.pk = self.shared.state.keys.public;
    }

    /// Broadcasts the keys.
    fn broadcast_keys(&mut self) {
        info!("broadcasting new keys");
        self.shared
            .events
            .broadcast_keys(self.shared.state.keys.clone());
    }

    /// Broadcasts the round parameters.
    fn broadcast_params(&mut self) {
        info!("broadcasting new round parameters");
        self.shared
            .events
            .broadcast_params(self.shared.state.round_params.clone());
    }
}

impl<T> PhaseState<Idle, T>
where
    T: Storage,
{
    /// Deletes the dicts from the store.
    async fn delete_dicts(&mut self) -> Result<(), IdleError> {
        info!("removing phase dictionaries from previous round");
        self.shared
            .store
            .delete_dicts()
            .await
            .map_err(IdleError::DeleteDictionaries)
    }

    /// Persists the coordinator state to the store.
    async fn set_coordinator_state(&mut self) -> Result<(), IdleError> {
        info!("storing new coordinator state");
        self.shared
            .store
            .set_coordinator_state(&self.shared.state)
            .await
            .map_err(IdleError::SetCoordinatorState)
    }
}

impl<T> PhaseState<Idle, T>
where
    T: Storage,
    Self: Phase<T>,
{
    /// Broadcasts idle phase metrics.
    fn broadcast_metrics(&self) {
        metric!(Measurement::RoundTotalNumber, self.shared.state.round_id);
        metric!(
            Measurement::RoundParamSum,
            self.shared.state.round_params.sum,
            ("round_id", self.shared.state.round_id),
            ("phase", Self::NAME as u8),
        );
        metric!(
            Measurement::RoundParamUpdate,
            self.shared.state.round_params.update,
            ("round_id", self.shared.state.round_id),
            ("phase", Self::NAME as u8),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use anyhow::anyhow;
    use xaynet_core::common::RoundParameters;

    use crate::{
        state_machine::{
            coordinator::CoordinatorState,
            events::{DictionaryUpdate, EventPublisher, EventSubscriber, ModelUpdate},
            tests::{
                utils::{assert_event_updated_with_id, enable_logging, init_shared, EventSnapshot},
                CoordinatorStateBuilder,
                EventBusBuilder,
            },
        },
        storage::{
            tests::{utils::create_global_model, MockCoordinatorStore, MockModelStore},
            Store,
        },
    };

    fn state_and_events_from_unmask_phase() -> (CoordinatorState, EventPublisher, EventSubscriber) {
        let state = CoordinatorStateBuilder::new().build();

        let (event_publisher, event_subscriber) = EventBusBuilder::new(&state)
            .broadcast_phase(PhaseName::Unmask)
            .broadcast_sum_dict(DictionaryUpdate::Invalidate)
            .broadcast_seed_dict(DictionaryUpdate::Invalidate)
            .broadcast_model(ModelUpdate::New(Arc::new(create_global_model(10))))
            .build();

        (state, event_publisher, event_subscriber)
    }

    fn assert_params(params1: &RoundParameters, params2: &RoundParameters) {
        assert_ne!(params1.pk, params2.pk);
        assert_ne!(params1.seed, params2.seed);
        assert!((params1.sum - params2.sum).abs() < f64::EPSILON);
        assert!((params1.update - params2.update).abs() < f64::EPSILON);
        assert_eq!(params1.mask_config, params2.mask_config);
        assert_eq!(params1.model_length, params2.model_length);
    }

    fn assert_after_delete_dict_failure(
        state_before: &CoordinatorState,
        events_before: &EventSnapshot,
        state_after: &CoordinatorState,
        events_after: &EventSnapshot,
    ) {
        assert_eq!(state_after.round_params.pk, state_before.round_params.pk);
        assert_eq!(
            state_after.round_params.seed,
            state_before.round_params.seed
        );
        assert!(
            (state_after.round_params.sum - state_before.round_params.sum).abs() < f64::EPSILON
        );
        assert!(
            (state_after.round_params.update - state_before.round_params.update).abs()
                < f64::EPSILON
        );
        assert_eq!(
            state_after.round_params.mask_config,
            state_before.round_params.mask_config
        );
        assert_eq!(
            state_after.round_params.model_length,
            state_before.round_params.model_length
        );

        assert_ne!(state_after.round_id, state_before.round_id);
        assert_eq!(state_after.keys, state_before.keys);
        assert_eq!(state_after.sum, state_before.sum);
        assert_eq!(state_after.update, state_before.update);
        assert_eq!(state_after.sum2, state_before.sum2);
        assert_eq!(state_after.keys.public, state_after.round_params.pk);
        assert_eq!(state_after.round_id, 1);

        assert_event_updated_with_id(&events_after.phase, &events_before.phase);
        assert_eq!(events_after.phase.event, PhaseName::Idle);
        assert_eq!(&events_after.keys, &events_before.keys);
        assert_eq!(&events_after.sum_dict, &events_before.sum_dict);
        assert_eq!(&events_after.seed_dict, &events_before.seed_dict);
        assert_eq!(events_after.params, events_before.params);
        assert_eq!(events_after.model, events_before.model);
    }

    #[tokio::test]
    async fn test_idle_to_sum_phase() {
        // No Storage errors
        // lets pretend we come from the unmask phase
        //
        // What should happen:
        // 1. increase round id by 1
        // 2. broadcast Idle phase
        // 3. delete the sum/seed/mask dict
        // 4. update coordinator keys
        // 5. update round thresholds (not implemented yet)
        // 6. update round seeds
        // 7. save the new coordinator state
        // 8. broadcast updated keys
        // 9. broadcast new round parameters
        // 10. move into sum phase
        //
        // What should not happen:
        // - the global model has been invalidated
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_delete_dicts().return_once(move || Ok(()));
        cs.expect_set_coordinator_state()
            .return_once(move |_| Ok(()));
        let store = Store::new(cs, MockModelStore::new());

        let (state, event_publisher, event_subscriber) = state_and_events_from_unmask_phase();
        let events_before_idle = EventSnapshot::from(&event_subscriber);
        let state_before_idle = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Idle, _>::new(shared));
        assert!(state_machine.is_idle());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_idle = state_machine.shared_state_as_ref().clone();
        assert_params(
            &state_after_idle.round_params,
            &state_before_idle.round_params,
        );
        assert_ne!(state_after_idle.keys, state_before_idle.keys);
        assert_ne!(state_after_idle.round_id, state_before_idle.round_id);
        assert_eq!(state_after_idle.sum, state_before_idle.sum);
        assert_eq!(state_after_idle.update, state_before_idle.update);
        assert_eq!(state_after_idle.sum2, state_before_idle.sum2);
        assert_eq!(
            state_after_idle.keys.public,
            state_after_idle.round_params.pk
        );
        assert_eq!(state_after_idle.round_id, 1);

        let events_after_idle = EventSnapshot::from(&event_subscriber);
        assert_event_updated_with_id(&events_after_idle.keys, &events_before_idle.keys);
        assert_event_updated_with_id(&events_after_idle.params, &events_before_idle.params);
        assert_event_updated_with_id(&events_after_idle.phase, &events_before_idle.phase);
        assert_eq!(events_after_idle.phase.event, PhaseName::Idle);
        assert_eq!(events_after_idle.sum_dict, events_before_idle.sum_dict);
        assert_eq!(events_after_idle.seed_dict, events_before_idle.seed_dict);
        assert_eq!(events_after_idle.model, events_before_idle.model);

        assert!(state_machine.is_sum());
    }

    #[tokio::test]
    async fn test_idle_to_sum_delete_dicts_failed() {
        // Storage:
        // - delete_dicts fails
        //
        // What should happen:
        // 1. increase round id by 1
        // 2. broadcast Idle phase
        // 3. delete the sum/seed/mask dict (fails)
        // 4. move into error phase
        //
        // What should not happen:
        // - new keys have been broadcasted
        // - new round parameters have been broadcasted
        // - the global model has been invalidated
        // - the state machine has moved into sum phase
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_delete_dicts()
            .return_once(move || Err(anyhow!("")));
        let store = Store::new(cs, MockModelStore::new());

        let (state, event_publisher, event_subscriber) = state_and_events_from_unmask_phase();
        let events_before_idle = EventSnapshot::from(&event_subscriber);
        let state_before_idle = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Idle, _>::new(shared));
        assert!(state_machine.is_idle());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_idle = state_machine.shared_state_as_ref().clone();
        let events_after_idle = EventSnapshot::from(&event_subscriber);
        assert_after_delete_dict_failure(
            &state_before_idle,
            &events_before_idle,
            &state_after_idle,
            &events_after_idle,
        );

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::Idle(IdleError::DeleteDictionaries(_))
        ))
    }

    #[tokio::test]
    async fn test_idle_to_sum_save_state_failed() {
        // Storage:
        // - set_coordinator_state fails
        //
        // What should happen:
        // 1. increase round id by 1
        // 2. broadcast Idle phase
        // 3. delete the sum/seed/mask dict
        // 4. update coordinator keys
        // 5. update round thresholds (not implemented yet)
        // 6. update round seeds
        // 7. save the new coordinator state (fails)

        // 6. broadcast updated keys

        // 10. move into error phase
        //
        // What should not happen:
        // - new round parameters have been broadcast
        // - the global model has been invalidated
        // - the state machine has moved into sum phase
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_delete_dicts().return_once(move || Ok(()));
        cs.expect_set_coordinator_state()
            .return_once(move |_| Err(anyhow!("")));
        let store = Store::new(cs, MockModelStore::new());

        let (state, event_publisher, event_subscriber) = state_and_events_from_unmask_phase();
        let events_before_idle = EventSnapshot::from(&event_subscriber);
        let state_before_idle = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Idle, _>::new(shared));
        assert!(state_machine.is_idle());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_idle = state_machine.shared_state_as_ref().clone();
        let events_after_idle = EventSnapshot::from(&event_subscriber);

        assert_params(
            &state_after_idle.round_params,
            &state_before_idle.round_params,
        );
        assert_ne!(state_after_idle.keys, state_before_idle.keys);
        assert_ne!(state_after_idle.round_id, state_before_idle.round_id);
        assert_eq!(state_after_idle.sum, state_before_idle.sum);
        assert_eq!(state_after_idle.update, state_before_idle.update);
        assert_eq!(state_after_idle.sum2, state_before_idle.sum2);
        assert_eq!(
            state_after_idle.keys.public,
            state_after_idle.round_params.pk
        );
        assert_eq!(state_after_idle.round_id, 1);

        assert_event_updated_with_id(&events_after_idle.phase, &events_before_idle.phase);
        assert_eq!(events_after_idle.phase.event, PhaseName::Idle);
        assert_eq!(&events_after_idle.keys, &events_before_idle.keys);
        assert_eq!(&events_after_idle.sum_dict, &events_before_idle.sum_dict);
        assert_eq!(&events_after_idle.seed_dict, &events_before_idle.seed_dict);
        assert_eq!(events_after_idle.params, events_before_idle.params);
        assert_eq!(events_after_idle.model, events_before_idle.model);

        assert!(state_machine.is_failure());
        assert!(matches!(
            state_machine.into_failure_phase_state().private.error,
            PhaseError::Idle(IdleError::SetCoordinatorState(_))
        ))
    }
}
