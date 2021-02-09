use async_trait::async_trait;
use sodiumoxide::crypto::hash::sha256;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::{
    metric,
    metrics::Measurement,
    state_machine::{
        phases::{Phase, PhaseName, PhaseState, PhaseStateError, Shared, Sum},
        StateMachine,
    },
    storage::{Storage, StorageError},
};
use xaynet_core::{
    common::RoundSeed,
    crypto::{ByteObject, EncryptKeyPair, SigningKeySeed},
};

/// Error that occurs during the idle phase.
#[derive(Error, Debug)]
pub enum IdleStateError {
    #[error("setting the coordinator state failed: {0}")]
    SetCoordinatorState(StorageError),
    #[error("deleting the dictionaries failed: {0}")]
    DeleteDictionaries(StorageError),
}

/// Idle state
#[derive(Debug)]
pub struct Idle;

#[async_trait]
impl<T> Phase<T> for PhaseState<Idle, T>
where
    T: Storage,
{
    const NAME: PhaseName = PhaseName::Idle;

    async fn process(&mut self) -> Result<(), PhaseStateError> {
        info!("removing phase dictionaries from previous round");
        self.shared
            .store
            .delete_dicts()
            .await
            .map_err(IdleStateError::DeleteDictionaries)?;

        info!("updating the keys");
        self.gen_round_keypair();

        info!("updating round probabilities");
        self.update_round_probabilities();

        info!("updating round seed");
        self.update_round_seed();

        info!("storing new coordinator state");
        self.shared
            .store
            .set_coordinator_state(&self.shared.state)
            .await
            .map_err(IdleStateError::SetCoordinatorState)?;

        Ok(())
    }

    async fn broadcast(&mut self) -> Result<(), PhaseStateError> {
        info!("broadcasting new keys");
        self.shared
            .events
            .broadcast_keys(self.shared.state.keys.clone());

        info!("broadcasting new round parameters");
        self.shared
            .events
            .broadcast_params(self.shared.state.round_params.clone());

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

        Ok(())
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

    fn update_round_probabilities(&mut self) {
        warn!("round probabilities stay constant, no update strategy implemented yet");
    }

    /// Updates the seed round parameter.
    fn update_round_seed(&mut self) {
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
        self.shared.state.keys = EncryptKeyPair::generate();
        self.shared.state.round_params.pk = self.shared.state.keys.public;
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;
    use crate::{
        state_machine::{
            events::Event,
            tests::{builder::StateMachineBuilder, utils},
        },
        storage::{tests::init_store, CoordinatorStorage},
    };

    #[tokio::test]
    #[serial]
    async fn integration_round_id_is_updated_when_idle_phase_runs() {
        let store = init_store().await;
        let coordinator_state = utils::coordinator_state();
        let (shared, _, event_subscriber) = utils::init_shared(coordinator_state, store);

        let keys = event_subscriber.keys_listener();
        let id = keys.get_latest().round_id;
        assert_eq!(id, 0);

        let mut idle_phase = PhaseState::<Idle, _>::new(shared);
        idle_phase.process().await.unwrap();
        idle_phase.broadcast().await.unwrap();

        let id = keys.get_latest().round_id;
        assert_eq!(id, 1);
    }

    #[tokio::test]
    #[serial]
    async fn integration_idle_to_sum() {
        let mut store = init_store().await;
        let (state_machine, _request_tx, events) = StateMachineBuilder::new(store.clone())
            .with_round_id(2)
            .build();
        assert!(state_machine.is_idle());

        let initial_round_params = events.params_listener().get_latest().event;
        let initial_keys = events.keys_listener().get_latest().event;
        let initial_seed = initial_round_params.seed.clone();

        let state_machine = state_machine.next().await.unwrap();
        assert!(state_machine.is_sum());

        let PhaseState { shared, .. } = state_machine.into_sum_phase_state();

        let sum_dict = store.sum_dict().await.unwrap();
        assert!(sum_dict.is_none());

        let new_round_params = shared.state.round_params.clone();
        let new_keys = shared.state.keys.clone();

        // Make sure the seed and keys have updated
        assert_ne!(initial_seed, new_round_params.seed);
        assert_ne!(initial_keys, new_keys);

        fn expected_event<T>(event: T) -> Event<T> {
            Event { round_id: 2, event }
        }

        // Check all the events that should be emitted during the idle
        // phase
        assert_eq!(
            events.phase_listener().get_latest(),
            expected_event(PhaseName::Idle),
        );

        assert_eq!(
            events.keys_listener().get_latest(),
            expected_event(new_keys),
        );

        assert_eq!(
            events.params_listener().get_latest(),
            expected_event(new_round_params),
        );
    }
}
