use super::{CoordinatorState, State, StateMachine};
use crate::{
    coordinator::RoundSeed,
    coordinator_async::sum::Sum,
    crypto::{generate_encrypt_key_pair, ByteObject, SigningKeySeed},
    InitError,
};
use sodiumoxide::crypto::hash::sha256;
use std::default::Default;

#[derive(Debug)]
pub struct Idle;

impl State<Idle> {
    pub fn new() -> Result<(tokio::sync::mpsc::UnboundedSender<()>, StateMachine), InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        Ok((
            tx,
            StateMachine::Idle(State {
                _inner: Idle,
                coordinator_state: CoordinatorState {
                    seed: RoundSeed::generate(),
                    ..Default::default()
                },
                message_rx: rx,
            }),
        ))
    }

    pub async fn next(mut self) -> StateMachine {
        println!("Idle phase!");
        self.run().await;

        StateMachine::Sum(State {
            _inner: Sum {},
            coordinator_state: self.coordinator_state,
            message_rx: self.message_rx,
        })
    }

    async fn run(&mut self) {
        // clear and write round_parameters in Redis
        // clear redis round dicts
        self.update_round_thresholds();
        self.update_round_seed();
        self.gen_round_keypair();
        // clear Aggregator
    }

    /// Generate fresh round credentials.
    fn gen_round_keypair(&mut self) {
        let (pk, sk) = generate_encrypt_key_pair();
        self.coordinator_state.pk = pk;
        self.coordinator_state.sk = sk;
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
