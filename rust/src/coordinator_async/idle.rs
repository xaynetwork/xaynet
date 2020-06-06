use super::{CoordinatorState, RedisStore, State, StateMachine};
use crate::{
    coordinator::{ProtocolEvent, RoundSeed},
    coordinator_async::sum::Sum,
    crypto::{ByteObject, SigningKeySeed},
};
use sodiumoxide::crypto::hash::sha256;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Idle;

impl State<Idle> {
    pub fn new(
        coordinator_state: CoordinatorState,
        message_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        redis: RedisStore,
        events_rx: mpsc::UnboundedSender<ProtocolEvent>,
    ) -> StateMachine {
        StateMachine::Idle(Self {
            _inner: Idle,
            coordinator_state,
            message_rx,
            redis,
            events_rx,
        })
    }

    pub async fn next(mut self) -> StateMachine {
        info!("Idle phase!");
        self.start_new_round().await;
        let _ = self.emit_round_parameters();

        State::<Sum>::new(
            self.coordinator_state,
            self.message_rx,
            self.redis,
            self.events_rx,
        )
    }

    async fn start_new_round(&mut self) {
        self.clear_round_dicts().await;

        self.update_round_thresholds();
        self.update_round_seed();

        self.set_coordinator_state().await;
        // clear Aggregator
    }

    fn update_round_thresholds(&mut self) {}

    /// Update the seed round parameter.
    fn update_round_seed(&mut self) {
        // safe unwrap: `sk` and `seed` have same number of bytes
        let (_, sk) = SigningKeySeed::from_slice_unchecked(self.coordinator_state.sk.as_slice())
            .derive_signing_key_pair();
        let signature = sk.sign_detached(
            &[
                self.coordinator_state.seed.as_slice(),
                &self.coordinator_state.sum.to_le_bytes(),
                &self.coordinator_state.update.to_le_bytes(),
            ]
            .concat(),
        );
        // Safe unwrap: the length of the hash is 32 bytes
        self.coordinator_state.seed =
            RoundSeed::from_slice_unchecked(sha256::hash(signature.as_slice()).as_ref());
    }

    /// Clear the round dictionaries.
    async fn clear_round_dicts(&self) {
        let _ = self.redis.clone().connection().await.flushdb().await;
    }

    fn emit_round_parameters(&self) {
        let _ = self
            .events_rx
            .send(ProtocolEvent::StartSum(self.round_parameters()));
    }
}
