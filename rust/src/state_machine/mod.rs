pub mod coordinator;
pub mod events;
pub mod phases;
pub mod requests;

use crate::{
    mask::masking::UnmaskingError,
    settings::{MaskSettings, PetSettings},
    state_machine::{
        coordinator::CoordinatorState,
        events::EventSubscriber,
        phases::{Idle, Phase, PhaseState, Shutdown, StateError, Sum, Sum2, Unmask, Update},
        requests::{Request, RequestReceiver, RequestSender},
    },
    utils::trace::Traced,
    InitError,
};

use derive_more::From;
use thiserror::Error;

/// Error that occurs when unmasking of the global model fails
#[derive(Error, Debug, Eq, PartialEq)]
pub enum RoundFailed {
    #[error("ambiguous masks were computed by the sum participants")]
    AmbiguousMasks,
    #[error("no mask found")]
    NoMask,
    #[error("unmasking error: {0}")]
    Unmasking(#[from] UnmaskingError),
}

#[derive(From)]
pub enum StateMachine<R> {
    Idle(PhaseState<R, Idle>),
    Sum(PhaseState<R, Sum>),
    Update(PhaseState<R, Update>),
    Sum2(PhaseState<R, Sum2>),
    Unmask(PhaseState<R, Unmask>),
    Error(PhaseState<R, StateError>),
    Shutdown(PhaseState<R, Shutdown>),
}

/// A [`StateMachine`] that processes `Traced<Request>`.
pub type TracingStateMachine = StateMachine<Traced<Request>>;

impl<R> StateMachine<R>
where
    PhaseState<R, Idle>: Phase<R>,
    PhaseState<R, Sum>: Phase<R>,
    PhaseState<R, Update>: Phase<R>,
    PhaseState<R, Sum2>: Phase<R>,
    PhaseState<R, Unmask>: Phase<R>,
    PhaseState<R, StateError>: Phase<R>,
    PhaseState<R, Shutdown>: Phase<R>,
{
    /// Create a new state machine with the initial state `Idle`.
    /// Fails if there is insufficient system entropy to generate secrets.
    pub fn new(
        pet_settings: PetSettings,
        mask_settings: MaskSettings,
    ) -> Result<(Self, RequestSender<R>, EventSubscriber), InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;
        let (coordinator_state, event_subscriber) =
            CoordinatorState::new(pet_settings, mask_settings);

        let (req_receiver, handle) = RequestReceiver::<R>::new();
        let state_machine =
            StateMachine::from(PhaseState::<R, Idle>::new(coordinator_state, req_receiver));
        Ok((state_machine, handle, event_subscriber))
    }

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

    /// Run the state machine until it shuts down.
    pub async fn run(mut self) -> Option<()> {
        loop {
            self = self.next().await?;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        crypto::{encrypt::EncryptKeyPair, sign::SigningKeyPair, ByteObject},
        mask::{
            config::{BoundType, DataType, GroupType, MaskConfig, ModelType},
            object::MaskObject,
            seed::{EncryptedMaskSeed, MaskSeed},
        },
        settings::{MaskSettings, PetSettings},
        state_machine::{
            requests::{Request, Sum2Request, SumRequest, UpdateRequest},
            StateMachine,
        },
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
        Request,
        SumParticipantPublicKey,
        oneshot::Receiver<Result<(), PetError>>,
    ) {
        let (response_tx, response_rx) = oneshot::channel::<Result<(), PetError>>();
        let SigningKeyPair {
            public: participant_pk,
            ..
        } = SigningKeyPair::generate();
        let EncryptKeyPair {
            public: ephm_pk, ..
        } = EncryptKeyPair::generate();
        let req = Request::Sum((
            SumRequest {
                participant_pk,
                ephm_pk,
            },
            response_tx,
        ));
        (req, participant_pk, response_rx)
    }

    fn gen_update_request(
        sum_pk: SumParticipantPublicKey,
    ) -> (Request, oneshot::Receiver<Result<(), PetError>>) {
        let (response_tx, response_rx) = oneshot::channel::<Result<(), PetError>>();
        let SigningKeyPair {
            public: participant_pk,
            ..
        } = SigningKeyPair::generate();
        let mut local_seed_dict = LocalSeedDict::new();
        local_seed_dict.insert(sum_pk, EncryptedMaskSeed::zeroed());
        let masked_model = gen_mask();
        let req = Request::Update((
            UpdateRequest {
                participant_pk,
                local_seed_dict,
                masked_model,
            },
            response_tx,
        ));

        (req, response_rx)
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
    ) -> (Request, oneshot::Receiver<Result<(), PetError>>) {
        let (response_tx, response_rx) = oneshot::channel::<Result<(), PetError>>();
        let mask = gen_mask();
        let req = Request::Sum2((
            Sum2Request {
                participant_pk: sum_pk,
                mask,
            },
            response_tx,
        ));
        (req, response_rx)
    }

    fn is_update(state_machine: &StateMachine<Request>) -> bool {
        match state_machine {
            StateMachine::Update(_) => true,
            _ => false,
        }
    }

    fn is_sum(state_machine: &StateMachine<Request>) -> bool {
        match state_machine {
            StateMachine::Sum(_) => true,
            _ => false,
        }
    }

    fn is_sum2(state_machine: &StateMachine<Request>) -> bool {
        match state_machine {
            StateMachine::Sum2(_) => true,
            _ => false,
        }
    }

    fn is_idle(state_machine: &StateMachine<Request>) -> bool {
        match state_machine {
            StateMachine::Idle(_) => true,
            _ => false,
        }
    }

    fn is_unmask(state_machine: &StateMachine<Request>) -> bool {
        match state_machine {
            StateMachine::Unmask(_) => true,
            _ => false,
        }
    }

    fn is_error(state_machine: &StateMachine<Request>) -> bool {
        match state_machine {
            StateMachine::Error(_) => true,
            _ => false,
        }
    }

    fn is_shutdown(state_machine: &StateMachine<Request>) -> bool {
        match state_machine {
            StateMachine::Shutdown(_) => true,
            _ => false,
        }
    }

    #[tokio::test]
    async fn test_state_machine() {
        enable_logging();
        let pet_settings = PetSettings {
            sum: 0.4,
            update: 0.5,
            min_sum: 1,
            min_update: 3,
            expected_participants: 10,
        };
        let mask_settings = MaskSettings {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };

        let (mut state_machine, request_tx, _events_subscriber) =
            StateMachine::new(pet_settings, mask_settings).unwrap();
        assert!(is_idle(&state_machine));

        state_machine = state_machine.next().await.unwrap(); // transition from init to sum state
        assert!(is_sum(&state_machine));

        let (sum_req, sum_pk, response_rx) = gen_sum_request();
        let _ = request_tx.send(sum_req);

        state_machine = state_machine.next().await.unwrap(); // transition from sum to update state
        assert!(is_update(&state_machine));
        assert!(response_rx.await.is_ok());

        for _ in 0..3 {
            let (req, _) = gen_update_request(sum_pk.clone());
            let _ = request_tx.send(req);
        }
        state_machine = state_machine.next().await.unwrap(); // transition from update to sum state
        assert!(is_sum2(&state_machine));

        let (req, response_rx) = gen_sum2_request(sum_pk.clone());
        let _ = request_tx.send(req);
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
