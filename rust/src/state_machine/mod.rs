use crate::{
    coordinator::{MaskDict, RoundFailed, RoundSeed},
    crypto::ByteObject,
    mask::{Aggregation, BoundType, DataType, GroupType, MaskConfig, ModelType},
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

    // round dictionaries
    /// Dictionary built during the sum phase.
    sum_dict: SumDict,
    /// Dictionary built during the update phase.
    seed_dict: SeedDict,
    /// Dictionary built during the sum2 phase.
    mask_dict: MaskDict,

    /// The masking configuration
    mask_config: MaskConfig,

    /// The aggregated masked model being built in the current round.
    aggregation: Aggregation,
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
        let aggregation = Aggregation::new(mask_config);
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
            aggregation,
        }
    }
}

#[derive(Error, Debug)]
pub enum StateError {
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
    /// Move to the next state and consume the old one.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        crypto::{generate_encrypt_key_pair, generate_signing_key_pair},
        mask::{EncryptedMaskSeed, MaskObject, MaskSeed},
        state_machine::requests::{Sum2Request, SumRequest, UpdateRequest},
        LocalSeedDict,
        PetError,
        SumParticipantPublicKey,
    };
    use tokio::sync::oneshot;
    use tracing_subscriber::*;

    fn enable_logging() {
        let _fmt_subscriber = FmtSubscriber::builder()
            .with_env_filter(EnvFilter::from_default_env())
            .with_ansi(true)
            .init();
    }

    fn gen_sum_request() -> (
        SumRequest,
        SumParticipantPublicKey,
        oneshot::Receiver<Result<(), PetError>>,
    ) {
        let (response_tx, response_rx) = oneshot::channel::<Result<(), PetError>>();
        let (participant_pk, _) = generate_signing_key_pair();
        let (ephm_pk, _) = generate_encrypt_key_pair();
        (
            SumRequest {
                participant_pk,
                ephm_pk,
                response_tx,
            },
            participant_pk,
            response_rx,
        )
    }

    fn gen_update_request(
        sum_pk: SumParticipantPublicKey,
    ) -> (UpdateRequest, oneshot::Receiver<Result<(), PetError>>) {
        let (response_tx, response_rx) = oneshot::channel::<Result<(), PetError>>();
        let (participant_pk, _) = generate_signing_key_pair();
        let mut local_seed_dict = LocalSeedDict::new();
        local_seed_dict.insert(sum_pk, EncryptedMaskSeed::zeroed());
        let masked_model = gen_mask();
        (
            UpdateRequest {
                participant_pk,
                local_seed_dict,
                masked_model,
                response_tx,
            },
            response_rx,
        )
    }

    fn gen_mask() -> MaskObject {
        let seed = MaskSeed::generate();
        let mask = seed.derive_mask(
            10,
            MaskConfig {
                group_type: GroupType::Prime,
                data_type: DataType::F32,
                bound_type: BoundType::B0,
                model_type: ModelType::M3,
            },
        );
        mask
    }

    fn gen_sum2_request(
        sum_pk: SumParticipantPublicKey,
    ) -> (Sum2Request, oneshot::Receiver<Result<(), PetError>>) {
        let (response_tx, response_rx) = oneshot::channel::<Result<(), PetError>>();
        let mask = gen_mask();
        (
            Sum2Request {
                participant_pk: sum_pk,
                mask,
                response_tx,
            },
            response_rx,
        )
    }

    fn is_update(state_machine: &StateMachine) -> bool {
        match state_machine {
            StateMachine::Update(_) => true,
            _ => false,
        }
    }

    fn is_sum(state_machine: &StateMachine) -> bool {
        match state_machine {
            StateMachine::Sum(_) => true,
            _ => false,
        }
    }

    fn is_sum2(state_machine: &StateMachine) -> bool {
        match state_machine {
            StateMachine::Sum2(_) => true,
            _ => false,
        }
    }

    fn is_idle(state_machine: &StateMachine) -> bool {
        match state_machine {
            StateMachine::Idle(_) => true,
            _ => false,
        }
    }

    #[tokio::test]
    async fn test_state_machine() {
        enable_logging();
        let (request_tx, mut state_machine) = StateMachine::new().unwrap();
        assert!(is_idle(&state_machine));

        state_machine = state_machine.next().await; // transition from init to sum state
        assert!(is_sum(&state_machine));

        let (sum_req, sum_pk, response_rx) = gen_sum_request();
        let _ = request_tx.send(Request::Sum(sum_req));

        state_machine = state_machine.next().await; // transition from sum to update state
        assert!(is_update(&state_machine));
        assert!(response_rx.await.is_ok());

        for _ in 0..3 {
            let (gen_update_request, _) = gen_update_request(sum_pk.clone());
            let _ = request_tx.send(Request::Update(gen_update_request));
        }
        state_machine = state_machine.next().await; // transition from update to sum state
        assert!(is_sum2(&state_machine));

        let (sum2_req, response_rx) = gen_sum2_request(sum_pk.clone());
        let _ = request_tx.send(Request::Sum2(sum2_req));
        state_machine = state_machine.next().await; // transition from sum2 to idle state
        assert!(response_rx.await.is_ok());
        assert!(is_idle(&state_machine));

        state_machine = state_machine.next().await; // transition from idle to sum state
        assert!(is_sum(&state_machine));
    }
}
