use crate::{
    coordinator::{ProtocolEvent, RoundSeed},
    coordinator_async::{error::Error, idle::Idle, sum::Sum, sum2::Sum2, update::Update},
    crypto::ByteObject,
    message::{MessageOpen, MessageOwned},
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    InitError,
    PetError,
};
use std::{collections::VecDeque, default::Default};
use thiserror::Error;
use tokio::sync::mpsc;

pub mod error;
pub mod idle;
pub mod message;
pub mod sum;
pub mod sum2;
pub mod update;

/// Error that occurs when the current round fails
#[derive(Error, Debug)]
pub enum StateError {
    #[error("state failed: timeout")]
    Timeout,
    #[error("state failed: protocol error: {0}")]
    ProtocolError(#[from] PetError),
}

#[derive(Debug)]
pub struct CoordinatorState {
    pk: CoordinatorPublicKey, // 32 bytes
    sk: CoordinatorSecretKey, // 32 bytes

    // round parameters
    sum: f64,
    update: f64,
    seed: RoundSeed,
    min_sum: usize,
    min_update: usize,

    /// Events emitted by the state machine
    events: VecDeque<ProtocolEvent>,
}

impl Default for CoordinatorState {
    fn default() -> Self {
        let pk = CoordinatorPublicKey::zeroed();
        let sk = CoordinatorSecretKey::zeroed();
        let sum = 0.01_f64;
        let update = 0.1_f64;
        let seed = RoundSeed::zeroed();
        let min_sum = 1_usize;
        let min_update = 3_usize;
        let events = VecDeque::new();
        Self {
            pk,
            sk,
            sum,
            update,
            seed,
            min_sum,
            min_update,
            events,
        }
    }
}

#[derive(Debug)]
pub struct State<S> {
    _inner: S,
    // coordinator state
    coordinator_state: CoordinatorState,
    // message rx
    message_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    // aggregator: Option<Aggregator>,
}

// Functions that are available for all states
impl<S> State<S> {
    fn message_opener(&self) -> MessageOpen<'_, '_> {
        MessageOpen {
            recipient_pk: &self.coordinator_state.pk,
            recipient_sk: &self.coordinator_state.sk,
        }
    }

    fn message_open(&self, message: Vec<u8>) -> Result<MessageOwned, PetError> {
        self.message_opener()
            .open(&message)
            .map_err(|_| PetError::InvalidMessage)
    }

    async fn next_message(&mut self) -> Result<MessageOwned, PetError> {
        let message = match self.message_rx.recv().await {
            Some(message) => message,
            None => panic!("all message senders have been dropped!"),
        };
        debug!("New message!");
        self.message_open(message)
    }
}

pub enum StateMachine {
    Idle(State<Idle>),
    Sum(State<Sum>),
    Update(State<Update>),
    Sum2(State<Sum2>),
    Error(State<Error>),
}

impl StateMachine {
    pub async fn next(self) -> Self {
        match self {
            StateMachine::Idle(val) => val.next().await,
            StateMachine::Sum(val) => val.next().await,
            StateMachine::Update(val) => val.next().await,
            StateMachine::Sum2(val) => val.next().await,
            StateMachine::Error(val) => val.next().await,
        }
    }

    pub fn new() -> Result<(mpsc::UnboundedSender<Vec<u8>>, Self), InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;
        let (message_tx, message_rx) = mpsc::unbounded_channel::<Vec<u8>>();

        let coordinator_state = CoordinatorState {
            seed: RoundSeed::generate(),
            ..Default::default()
        };

        Ok((
            message_tx,
            State::<Idle>::new(coordinator_state, message_rx),
        ))
    }
}
