use crate::{
    coordinator::{MaskDict, RoundFailed, RoundSeed},
    crypto::ByteObject,
    mask::{BoundType, DataType, GroupType, MaskConfig, ModelType},
    state_machine::{
        idle::Idle,
        shutdown::Shutdown,
        sum::Sum,
        sum2::Sum2,
        unmask::Unmask,
        update::Update,
    },
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    InitError,
    PetError,
    SeedDict,
    SumDict,
};
use derive_more::From;
use std::default::Default;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

mod error;
mod idle;
pub mod requests;
mod shutdown;
mod sum;
mod sum2;
mod unmask;
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
    expected_participants: usize,

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
        let expected_participants = 10_usize;
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
            expected_participants,
            mask_config,
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

pub struct PhaseState<S> {
    // Inner state
    inner: S,
    // Coordinator state
    coordinator_state: CoordinatorState,
    // Request receiver halve
    request_rx: mpsc::UnboundedReceiver<Request>,
}

// Functions that are available to all states
impl<S> PhaseState<S> {
    /// Receives the next [`Request`].
    /// Returns [`StateError::ChannelError`] when all sender halve have been dropped.
    async fn next_request(&mut self) -> Result<Request, StateError> {
        debug!("received new message");
        self.request_rx.recv().await.ok_or(StateError::ChannelError(
            "all message senders have been dropped!",
        ))
    }

    /// Handle an invalid request.
    fn handle_invalid_message(response_tx: oneshot::Sender<Result<(), PetError>>) {
        debug!("invalid message");
        // `send` returns an error if the receiver halve has already been dropped. This means that
        // the receiver is not interested in the response of the request. Therefore the error is
        // ignored.
        let _ = response_tx.send(Err(PetError::InvalidMessage));
    }
}

#[derive(From)]
pub enum StateMachine {
    Idle(PhaseState<Idle>),
    Sum(PhaseState<Sum>),
    Update(PhaseState<Update>),
    Sum2(PhaseState<Sum2>),
    Unmask(PhaseState<Unmask>),
    Error(PhaseState<StateError>),
    Shutdown(PhaseState<Shutdown>),
}

impl StateMachine {
    /// Move to the next state and consume the old one.
    pub async fn next(self) -> Option<Self> {
        match self {
            StateMachine::Idle(state) => state.next().await,
            StateMachine::Sum(state) => state.next().await,
            StateMachine::Update(state) => state.next().await,
            StateMachine::Sum2(state) => state.next().await,
            StateMachine::Unmask(state) => state.next().await,
            StateMachine::Error(state) => state.next().await,
            StateMachine::Shutdown(state) => state.next().await,
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
            PhaseState::<Idle>::new(coordinator_state, request_rx).into(),
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

    fn is_unmask(state_machine: &StateMachine) -> bool {
        match state_machine {
            StateMachine::Unmask(_) => true,
            _ => false,
        }
    }

    fn is_error(state_machine: &StateMachine) -> bool {
        match state_machine {
            StateMachine::Error(_) => true,
            _ => false,
        }
    }

    fn is_shutdown(state_machine: &StateMachine) -> bool {
        match state_machine {
            StateMachine::Shutdown(_) => true,
            _ => false,
        }
    }

    #[tokio::test]
    async fn test_state_machine() {
        enable_logging();
        let (request_tx, mut state_machine) = StateMachine::new().unwrap();
        assert!(is_idle(&state_machine));

        state_machine = state_machine.next().await.unwrap(); // transition from init to sum state
        assert!(is_sum(&state_machine));

        let (sum_req, sum_pk, response_rx) = gen_sum_request();
        let _ = request_tx.send(Request::Sum(sum_req));

        state_machine = state_machine.next().await.unwrap(); // transition from sum to update state
        assert!(is_update(&state_machine));
        assert!(response_rx.await.is_ok());

        for _ in 0..3 {
            let (gen_update_request, _) = gen_update_request(sum_pk.clone());
            let _ = request_tx.send(Request::Update(gen_update_request));
        }
        state_machine = state_machine.next().await.unwrap(); // transition from update to sum state
        assert!(is_sum2(&state_machine));

        let (sum2_req, response_rx) = gen_sum2_request(sum_pk.clone());
        let _ = request_tx.send(Request::Sum2(sum2_req));
        state_machine = state_machine.next().await.unwrap(); // transition from sum2 to unmasked state
        assert!(response_rx.await.is_ok());
        assert!(is_unmask(&state_machine));

        state_machine = state_machine.next().await.unwrap(); // transition from unmasked to idle state
        assert!(is_idle(&state_machine));

        drop(request_tx);
        state_machine = state_machine.next().await.unwrap(); // transition from idle to sum state
        assert!(is_sum(&state_machine));

        state_machine = state_machine.next().await.unwrap(); // transition from sum to error state
        assert!(is_error(&state_machine));

        state_machine = state_machine.next().await.unwrap(); // transition from error to shutdown state
        assert!(is_shutdown(&state_machine));
        assert!(state_machine.next().await.is_none())
    }
}
