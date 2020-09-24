use xaynet_core::{
    common::RoundSeed,
    crypto::{ByteObject, EncryptKeyPair, SigningKeySeed},
};

use crate::state_machine::{
    events::{DictionaryUpdate, MaskLengthUpdate},
    phases::{Handler, Phase, PhaseName, PhaseState, Shared, Sum},
    requests::StateMachineRequest,
    StateError,
    StateMachine,
    StateMachineError,
};

#[cfg(feature = "metrics")]
use crate::metrics;

use sodiumoxide::crypto::hash::sha256;

/// Idle state
#[derive(Debug)]
pub struct Idle;

#[async_trait]
impl Handler for PhaseState<Idle> {
    /// Reject the request with a [`StateMachineError::MessageRejected`]
    async fn handle_request(&mut self, _req: StateMachineRequest) -> Result<(), StateMachineError> {
        Err(StateMachineError::MessageRejected)
    }
}

#[async_trait]
impl Phase for PhaseState<Idle> {
    const NAME: PhaseName = PhaseName::Idle;

    /// Moves from the idle state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn run(&mut self) -> Result<(), StateError> {
        info!("updating the keys");
        self.gen_round_keypair();

        info!("updating round thresholds");
        self.update_round_thresholds();

        info!("updating round seeds");
        self.update_round_seed();

        self.shared
            .io
            .redis
            .connection()
            .await
            .set_coordinator_state(&self.shared.state)
            .await?;

        let events = &mut self.shared.io.events;

        info!("broadcasting new keys");
        events.broadcast_keys(self.shared.state.keys.clone());

        info!("broadcasting invalidation of sum dictionary from previous round");
        events.broadcast_sum_dict(DictionaryUpdate::Invalidate);

        info!("broadcasting invalidation of seed dictionary from previous round");
        events.broadcast_seed_dict(DictionaryUpdate::Invalidate);

        info!("broadcasting invalidation of mask length from previous round");
        events.broadcast_mask_length(MaskLengthUpdate::Invalidate);

        self.shared
            .io
            .redis
            .connection()
            .await
            .flush_dicts()
            .await?;

        info!("broadcasting new round parameters");
        events.broadcast_params(self.shared.state.round_params.clone());

        metrics!(
            self.shared.io.metrics_tx,
            metrics::round::total_number::update(self.shared.state.round_id),
            metrics::round_parameters::sum::update(
                self.shared.state.round_params.sum,
                self.shared.state.round_id,
                Self::NAME
            ),
            metrics::round_parameters::update::update(
                self.shared.state.round_params.update,
                self.shared.state.round_id,
                Self::NAME
            )
        );

        // TODO: add a delay to prolongate the idle phase
        Ok(())
    }

    fn next(self) -> Option<StateMachine> {
        info!("going to sum phase");
        Some(PhaseState::<Sum>::new(self.shared).into())
    }
}

impl PhaseState<Idle> {
    /// Creates a new idle state.
    pub fn new(mut shared: Shared) -> Self {
        // Since some events are emitted very early, the round id must
        // be correct when the idle phase starts. Therefore, we update
        // it here, when instantiating the idle PhaseState.
        shared.set_round_id(shared.round_id() + 1);
        debug!("new round ID = {}", shared.round_id());
        Self {
            inner: Idle,
            shared,
        }
    }

    fn update_round_thresholds(&mut self) {}

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
mod test {
    use super::*;
    use crate::state_machine::{
        events::Event,
        tests::{builder::StateMachineBuilder, utils},
    };
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn integration_round_id_is_updated_when_idle_phase_runs() {
        let (shared, event_subscriber, ..) = utils::init_shared().await;

        let keys = event_subscriber.keys_listener();
        let id = keys.get_latest().round_id;
        assert_eq!(id, 0);

        let mut idle_phase = PhaseState::<Idle>::new(shared);
        idle_phase.run().await.unwrap();

        let id = keys.get_latest().round_id;
        assert_eq!(id, 1);
    }

    #[tokio::test]
    #[serial]
    async fn integration_idle_to_sum() {
        let (state_machine, _request_tx, events, redis) =
            StateMachineBuilder::new().await.with_round_id(2).build();
        assert!(state_machine.is_idle());

        let initial_round_params = events.params_listener().get_latest().event;
        let initial_keys = events.keys_listener().get_latest().event;
        let initial_seed = initial_round_params.seed.clone();

        let state_machine = state_machine.next().await.unwrap();
        assert!(state_machine.is_sum());

        let PhaseState { shared, .. } = state_machine.into_sum_phase_state();

        let sum_dict = redis.connection().await.get_sum_dict().await.unwrap();
        assert!(sum_dict.is_empty());

        let new_round_params = shared.state.round_params.clone();
        let new_keys = shared.state.keys.clone();

        // Make sure the seed and keys have updated
        assert_ne!(initial_seed, new_round_params.seed.clone());
        assert_ne!(initial_keys, new_keys);

        fn expected_event<T>(event: T) -> Event<T> {
            Event { round_id: 2, event }
        }

        // Check all the events that should be emitted during the idle
        // phase
        assert_eq!(
            events.phase_listener().get_latest(),
            expected_event(PhaseName::Idle)
        );

        assert_eq!(
            events.keys_listener().get_latest(),
            expected_event(new_keys),
        );

        assert_eq!(
            events.params_listener().get_latest(),
            expected_event(new_round_params)
        );

        assert_eq!(
            events.sum_dict_listener().get_latest(),
            expected_event(DictionaryUpdate::Invalidate)
        );

        assert_eq!(
            events.seed_dict_listener().get_latest(),
            expected_event(DictionaryUpdate::Invalidate)
        );

        assert_eq!(
            events.mask_length_listener().get_latest(),
            expected_event(MaskLengthUpdate::Invalidate)
        );
    }
}
