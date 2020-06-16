use super::{sum::Sum, CoordinatorState, Request, State, StateMachine};
use crate::{
    coordinator::RoundSeed,
    crypto::{ByteObject, SigningKeySeed},
};

use sodiumoxide::crypto::hash::sha256;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Idle;

impl State<Idle> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
    ) -> StateMachine {
        info!("state transition");
        StateMachine::Idle(Self {
            inner: Idle,
            coordinator_state,
            request_rx,
        })
    }

    pub async fn next(mut self) -> StateMachine {
        self.update_round_thresholds();
        self.update_round_seed();
        State::<Sum>::new(self.coordinator_state, self.request_rx)
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
}
