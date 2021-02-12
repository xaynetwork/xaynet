use std::time::Duration;

use async_trait::async_trait;
use displaydoc::Display;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{error, info};

use crate::{
    event,
    state_machine::{
        events::DictionaryUpdate,
        phases::{
            Idle,
            IdleError,
            Phase,
            PhaseName,
            PhaseState,
            Shared,
            Shutdown,
            SumError,
            UnmaskError,
            UpdateError,
        },
        StateMachine,
    },
    storage::Storage,
};

/// Errors which can occur during the execution of the [`StateMachine`].
#[derive(Debug, Display, Error)]
pub enum PhaseError {
    /// Request channel error: {0}.
    RequestChannel(&'static str),
    /// Phase timeout.
    PhaseTimeout(#[from] tokio::time::error::Elapsed),
    /// Idle phase failed: {0}.
    Idle(#[from] IdleError),
    /// Sum phase failed: {0}.
    Sum(#[from] SumError),
    /// Update phase failed: {0}.
    Update(#[from] UpdateError),
    /// Unmask phase failed: {0}.
    Unmask(#[from] UnmaskError),
}

/// The failure state.
#[derive(Debug)]
pub struct Failure {
    pub(in crate::state_machine) error: PhaseError,
}

#[async_trait]
impl<T> Phase<T> for PhaseState<Failure, T>
where
    T: Storage,
{
    const NAME: PhaseName = PhaseName::Failure;

    async fn process(&mut self) -> Result<(), PhaseError> {
        error!("phase state error: {}", self.private.error);
        event!("Phase error", self.private.error.to_string());

        Ok(())
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

    async fn next(mut self) -> Option<StateMachine<T>> {
        if let PhaseError::RequestChannel(_) = self.private.error {
            Some(PhaseState::<Shutdown, _>::new(self.shared).into())
        } else {
            self.wait_for_store_readiness().await;
            Some(PhaseState::<Idle, _>::new(self.shared).into())
        }
    }
}

impl<T> PhaseState<Failure, T> {
    /// Creates a new error phase.
    pub fn new(shared: Shared<T>, error: PhaseError) -> Self {
        Self {
            private: Failure { error },
            shared,
        }
    }
}

impl<T> PhaseState<Failure, T>
where
    T: Storage,
{
    /// Waits until the [`Store`] is ready.
    ///
    /// [`Store`]: crate::storage::Store
    async fn wait_for_store_readiness(&mut self) {
        while let Err(err) = <T as Storage>::is_ready(&mut self.shared.store).await {
            error!("store not ready: {}", err);
            info!("try again in 5 sec");
            sleep(Duration::from_secs(5)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    use anyhow::anyhow;
    use tokio::time::{timeout, Duration, Instant};
    use xaynet_core::{SeedDict, SumDict};

    use crate::{
        state_machine::{
            coordinator::CoordinatorState,
            events::{EventPublisher, EventSubscriber, ModelUpdate},
            tests::{
                utils::{enable_logging, init_shared, EventSnapshot},
                CoordinatorStateBuilder,
                EventBusBuilder,
            },
        },
        storage::{
            tests::{utils::create_global_model, MockCoordinatorStore, MockModelStore},
            Store,
        },
    };

    fn state_and_events_from_sum2_phase() -> (CoordinatorState, EventPublisher, EventSubscriber) {
        let state = CoordinatorStateBuilder::new().build();

        let (event_publisher, event_subscriber) = EventBusBuilder::new(&state)
            .broadcast_phase(PhaseName::Sum2)
            .broadcast_sum_dict(DictionaryUpdate::New(Arc::new(SumDict::new())))
            .broadcast_seed_dict(DictionaryUpdate::New(Arc::new(SeedDict::new())))
            .broadcast_model(ModelUpdate::New(Arc::new(create_global_model(1))))
            .build();

        (state, event_publisher, event_subscriber)
    }

    #[tokio::test]
    async fn error_to_idle_phase() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Error phase
        // 2. broadcast invalidation of sum and seed dict
        // 3. check if store is ready to process requests
        // 4. move into idle phase
        //
        // What should not happen:
        // - the shared state has been changed
        //   (except for`round_id` when moving into idle phase)
        // - events have been broadcasted (except phase event and invalidation
        //   event of sum and seed dict)
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_is_ready().return_once(move || Ok(()));

        let mut ms = MockModelStore::new();
        ms.expect_is_ready().return_once(move || Ok(()));

        let store = Store::new(cs, ms);

        let (state, event_publisher, event_subscriber) = state_and_events_from_sum2_phase();
        let events_before_error = EventSnapshot::from(&event_subscriber);
        let state_before_error = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Failure, _>::new(
            shared,
            PhaseError::Idle(IdleError::DeleteDictionaries(anyhow!(""))),
        ));
        assert!(state_machine.is_failure());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_error = state_machine.as_ref().clone();

        // round id is updated in idle phase
        assert_ne!(state_after_error.round_id, state_before_error.round_id);
        assert_eq!(
            state_after_error.round_params,
            state_before_error.round_params
        );
        assert_eq!(state_after_error.keys, state_before_error.keys);
        assert_eq!(state_after_error.sum, state_before_error.sum);
        assert_eq!(state_after_error.update, state_before_error.update);
        assert_eq!(state_after_error.sum2, state_before_error.sum2);

        let events_after_error = EventSnapshot::from(&event_subscriber);
        assert_ne!(events_after_error.phase, events_before_error.phase);
        assert_eq!(events_after_error.keys, events_before_error.keys);
        assert_eq!(events_after_error.params, events_before_error.params);
        assert_eq!(
            events_after_error.sum_dict.event,
            DictionaryUpdate::Invalidate
        );
        assert_eq!(
            events_after_error.seed_dict.event,
            DictionaryUpdate::Invalidate
        );
        assert_eq!(events_after_error.model, events_before_error.model);
        assert_eq!(events_after_error.phase.event, PhaseName::Failure);

        assert!(state_machine.is_idle());
    }

    #[tokio::test]
    async fn test_error_to_shutdown_phase() {
        // No Storage errors
        //
        // What should happen:
        // 1. broadcast Error phase
        // 2. broadcast invalidation of sum and seed dict
        // 3. previous phase failed with Failure::RequestChannel
        //    which means that the state machine should be shut down
        // 4. move into shutdown phase
        //
        // What should not happen:
        // - the shared state has been changed
        // - events have been broadcasted (except phase event and invalidation
        //   event of sum and seed dict)
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        cs.expect_is_ready().return_once(move || Ok(()));

        let mut ms = MockModelStore::new();
        ms.expect_is_ready().return_once(move || Ok(()));

        let store = Store::new(cs, ms);

        let (state, event_publisher, event_subscriber) = state_and_events_from_sum2_phase();
        let events_before_error = EventSnapshot::from(&event_subscriber);
        let state_before_error = state.clone();

        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Failure, _>::new(
            shared,
            PhaseError::RequestChannel(""),
        ));
        assert!(state_machine.is_failure());

        let state_machine = state_machine.next().await.unwrap();

        let state_after_error = state_machine.as_ref().clone();

        assert_eq!(state_after_error, state_before_error);

        let events_after_error = EventSnapshot::from(&event_subscriber);
        assert_ne!(events_after_error.phase, events_before_error.phase);
        assert_eq!(events_after_error.keys, events_before_error.keys);
        assert_eq!(events_after_error.params, events_before_error.params);
        assert_eq!(
            events_after_error.sum_dict.event,
            DictionaryUpdate::Invalidate
        );
        assert_eq!(
            events_after_error.seed_dict.event,
            DictionaryUpdate::Invalidate
        );
        assert_eq!(events_after_error.model, events_before_error.model);
        assert_eq!(events_after_error.phase.event, PhaseName::Failure);

        assert!(state_machine.is_shutdown());
    }

    #[tokio::test]
    async fn test_error_to_idle_store_failed() {
        // Storage error:
        // - first call on `is_ready` the coordinator store and model store fails
        // - second call on `is_ready` the coordinator store fails and model store passes
        // - third call on `is_ready` the coordinator store passes and model store fails
        // - forth call on `is_ready` the coordinator store and model store passes
        //
        // What should happen:
        // 1. broadcast Error phase
        // 2. broadcast invalidation of sum and seed dict
        // 3. check if store is ready to process requests
        // 4. wait until store is ready again (15 sec)
        // 5. move into idle phase
        //
        // What should not happen:
        // - the shared state has been changed
        //   (except for`round_id` when moving into idle phase)
        // - events have been broadcasted (except phase event and invalidation
        //   event of sum and seed dict)
        enable_logging();

        let mut cs = MockCoordinatorStore::new();
        let mut cs_counter = 0;
        cs.expect_is_ready().returning(move || {
            let res = match cs_counter {
                0 => Err(anyhow!("")),
                1 => Err(anyhow!("")),
                2 => Ok(()),
                3 => Ok(()),
                _ => panic!(""),
            };
            cs_counter += 1;
            res
        });

        let mut ms = MockModelStore::new();
        let mut ms_counter = 0;
        ms.expect_is_ready().returning(move || {
            let res = match ms_counter {
                // we skip step 1 and 2 because Storage::is_ready does not call
                // MockModelStore::is_ready if MockCoordinatorStore::is_ready
                // has already failed
                0 => Err(anyhow!("")),
                1 => Ok(()),
                _ => panic!(""),
            };
            ms_counter += 1;
            res
        });

        let store = Store::new(cs, ms);

        let state = CoordinatorStateBuilder::new().build();
        let (event_publisher, _event_subscriber) = EventBusBuilder::new(&state).build();
        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Failure, _>::new(
            shared,
            PhaseError::Idle(IdleError::DeleteDictionaries(anyhow!(""))),
        ));

        assert!(state_machine.is_failure());

        let now = Instant::now();

        let state_machine = timeout(Duration::from_secs(20), state_machine.next())
            .await
            .unwrap()
            .unwrap();

        assert!(now.elapsed().as_secs() > 14);

        assert!(state_machine.is_idle());
    }

    #[tokio::test]
    async fn test_error_to_shutdown_skip_store_readiness_check() {
        // Storage error:
        //
        // What should happen:
        // 1. broadcast Error phase
        // 2. broadcast invalidation of sum and seed dict
        // 3. previous phase failed with Failure::RequestChannel
        //    which means that the state machine should be shut down
        // 4. skip store readiness check
        // 5. move into shutdown phase
        //
        // What should not happen:
        // - wait for the store to be ready again
        // - the shared state has been changed
        // - events have been broadcasted (except phase event and invalidation
        //   event of sum and seed dict)
        enable_logging();

        let store = Store::new(MockCoordinatorStore::new(), MockModelStore::new());

        let state = CoordinatorStateBuilder::new().build();
        let (event_publisher, _event_subscriber) = EventBusBuilder::new(&state).build();
        let (shared, _request_tx) = init_shared(state, store, event_publisher);
        let state_machine = StateMachine::from(PhaseState::<Failure, _>::new(
            shared,
            PhaseError::RequestChannel(""),
        ));

        assert!(state_machine.is_failure());

        let state_machine = timeout(Duration::from_secs(5), state_machine.next())
            .await
            .unwrap()
            .unwrap();

        assert!(state_machine.is_shutdown());
    }
}
