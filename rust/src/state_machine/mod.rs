use crate::{
    coordinator::{MaskDict, RoundFailed, RoundSeed},
    crypto::ByteObject,
    mask::{BoundType, DataType, GroupType, MaskConfig, ModelType},
    state_machine::{error::Error, idle::Idle, sum::Sum, sum2::Sum2, update::Update},
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    InitError,
    PetError,
    SeedDict,
    SumDict,
};
use std::default::Default;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

mod error;
mod idle;
pub mod requests;
mod sum;
mod sum2;
mod update;

use requests::Request;

#[derive(Debug, Clone)]
pub struct CoordinatorState {
    pk: CoordinatorPublicKey, // 32 bytes
    sk: CoordinatorSecretKey, // 32 bytes

    // round parameters
    sum: f64,
    update: f64,
    seed: RoundSeed,
    min_sum: usize,
    min_update: usize,

    // round dictionaries
    /// Dictionary built during the sum phase.
    sum_dict: SumDict,
    /// Dictionary built during the update phase.
    seed_dict: SeedDict,
    /// Dictionary built during the sum2 phase.
    mask_dict: MaskDict,

    /// The masking configuration
    mask_config: MaskConfig,
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
        let sum_dict = SumDict::new();
        let seed_dict = SeedDict::new();
        let mask_dict = MaskDict::new();
        let mask_config = MaskConfig {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };
        Self {
            pk,
            sk,
            sum,
            update,
            seed,
            min_sum,
            min_update,
            sum_dict,
            seed_dict,
            mask_dict,
            mask_config,
        }
    }
}

#[derive(Error, Debug)]
pub enum StateError {
    #[error("state failed: protocol error: {0}")]
    ProtocolError(#[from] PetError),
    #[error("state failed: channel error: {0}")]
    ChannelError(&'static str),
    #[error("state failed: round error: {0}")]
    RoundError(#[from] RoundFailed),
}

pub struct State<S> {
    // Inner state
    _inner: S,
    // Coordinator state
    coordinator_state: CoordinatorState,
    // Request receiver
    request_rx: mpsc::UnboundedReceiver<Request>,
}

// Functions that are available to all states
impl<S> State<S> {
    /// Receives the next [`Request`].
    /// Returns [`StateError::ChannelError`] when all senders are dropped.
    async fn next_request(&mut self) -> Result<Request, StateError> {
        debug!("received new message");
        self.request_rx.recv().await.ok_or(StateError::ChannelError(
            "all message senders have been dropped!",
        ))
    }

    // Handle invalid requests.
    fn handle_invalid_message(response_tx: oneshot::Sender<Result<(), PetError>>) {
        debug!("invalid message");
        let _ = response_tx.send(Err(PetError::InvalidMessage));
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
    /// Move to a next state and consume the old state.
    pub async fn next(self) -> Self {
        match self {
            StateMachine::Idle(state) => state.next().await,
            StateMachine::Sum(state) => state.next().await,
            StateMachine::Update(state) => state.next().await,
            StateMachine::Sum2(state) => state.next().await,
            StateMachine::Error(state) => state.next().await,
        }
    }

    /// Create a new state machine with the initial state `Idle`.
    /// Fails if there is insufficient system entropy to generate secrets.
    pub fn new() -> Result<(mpsc::UnboundedSender<Request>, Self), InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;

        let (request_tx, request_rx) = mpsc::unbounded_channel::<Request>();
        let coordinator_state = CoordinatorState {
            seed: RoundSeed::generate(),
            ..Default::default()
        };

        Ok((
            request_tx,
            State::<Idle>::new(coordinator_state, request_rx),
        ))
    }
}
