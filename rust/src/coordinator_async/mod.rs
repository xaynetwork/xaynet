use crate::{
    coordinator::{ProtocolEvent, RoundParameters, RoundSeed},
    coordinator_async::{
        error::Error,
        idle::Idle,
        store::client::RedisStore,
        sum::Sum,
        sum2::Sum2,
        update::Update,
    },
    crypto::ByteObject,
    message::{MessageOpen, MessageOwned},
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    InitError,
    PetError,
};
use redis::RedisError;
use std::default::Default;
use thiserror::Error;
use tokio::sync::mpsc;

pub mod error;
pub mod idle;
pub mod message;
pub mod store;
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
    #[error("state failed: external service failed: {0}")]
    ExternalServiceFailed(#[from] RedisError),
    #[error("state failed: channel error: {0}")]
    ChannelError(&'static str),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoordinatorState {
    pk: CoordinatorPublicKey, // 32 bytes
    sk: CoordinatorSecretKey, // 32 bytes

    // round parameters
    sum: f64,
    update: f64,
    seed: RoundSeed,
    min_sum: usize,
    min_update: usize,
}

impl Default for CoordinatorState {
    fn default() -> Self {
        let pk = CoordinatorPublicKey::zeroed();
        let sk = CoordinatorSecretKey::zeroed();
        let sum = 0.4_f64;
        let update = 0.5_f64;
        let seed = RoundSeed::zeroed();
        let min_sum = 1_usize;
        let min_update = 3_usize;
        Self {
            pk,
            sk,
            sum,
            update,
            seed,
            min_sum,
            min_update,
        }
    }
}

pub struct State<S> {
    _inner: S,
    // Coordinator state
    coordinator_state: CoordinatorState,
    // message rx
    message_rx: mpsc::UnboundedReceiver<Vec<u8>>,

    // Redis store
    redis: RedisStore,

    /// Events emitted by the state machine
    events_rx: mpsc::UnboundedSender<ProtocolEvent>,
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

    fn open_message(&self, encr_message: Vec<u8>) -> Result<MessageOwned, PetError> {
        self.message_opener()
            .open(&encr_message)
            .map_err(|_| PetError::InvalidMessage)
    }

    async fn next_message(&mut self) -> Result<MessageOwned, StateError> {
        let encr_message = self
            .message_rx
            .recv()
            .await
            .ok_or(StateError::ChannelError(
                "all message senders have been dropped!",
            ))?;

        debug!("received new message");
        self.open_message(encr_message).map_err(From::from)
    }

    pub fn round_parameters(&self) -> RoundParameters {
        RoundParameters {
            pk: self.coordinator_state.pk,
            sum: self.coordinator_state.sum,
            update: self.coordinator_state.update,
            seed: self.coordinator_state.seed.clone(),
        }
    }

    /// Write the coordinator state.
    async fn set_coordinator_state(&self) {
        let _ = self
            .redis
            .clone()
            .connection()
            .await
            .set_coordinator_state(&self.coordinator_state)
            .await;
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

    pub fn new(
        redis: RedisStore,
    ) -> Result<
        (
            mpsc::UnboundedSender<Vec<u8>>,
            mpsc::UnboundedReceiver<ProtocolEvent>,
            Self,
        ),
        InitError,
    > {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;
        let (message_tx, message_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (events_tx, events_rx) = mpsc::unbounded_channel::<ProtocolEvent>();

        let coordinator_state = CoordinatorState {
            seed: RoundSeed::generate(),
            ..Default::default()
        };

        Ok((
            message_tx,
            events_rx,
            State::<Idle>::new(coordinator_state, message_rx, redis, events_tx),
        ))
    }
}
