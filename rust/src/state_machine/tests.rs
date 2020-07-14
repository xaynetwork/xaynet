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

impl<T> StateMachine<T> {
    fn is_update(&self) -> bool {
        match self {
            StateMachine::Update(_) => true,
            _ => false,
        }
    }

    fn is_sum(&self) -> bool {
        match self {
            StateMachine::Sum(_) => true,
            _ => false,
        }
    }

    fn is_sum2(&self) -> bool {
        match self {
            StateMachine::Sum2(_) => true,
            _ => false,
        }
    }

    fn is_idle(&self) -> bool {
        match self {
            StateMachine::Idle(_) => true,
            _ => false,
        }
    }

    fn is_unmask(&self) -> bool {
        match self {
            StateMachine::Unmask(_) => true,
            _ => false,
        }
    }

    fn is_error(&self) -> bool {
        match self {
            StateMachine::Error(_) => true,
            _ => false,
        }
    }

    fn is_shutdown(&self) -> bool {
        match self {
            StateMachine::Shutdown(_) => true,
            _ => false,
        }
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
    assert!(state_machine.is_idle());

    state_machine = state_machine.next().await.unwrap(); // transition from init to sum state
    assert!(state_machine.is_sum());

    let (sum_req, sum_pk, response_rx) = gen_sum_request();
    let _ = request_tx.send(sum_req);

    state_machine = state_machine.next().await.unwrap(); // transition from sum to update state
    assert!(state_machine.is_update());
    assert!(response_rx.await.is_ok());

    for _ in 0..3 {
        let (req, _) = gen_update_request(sum_pk.clone());
        let _ = request_tx.send(req);
    }
    state_machine = state_machine.next().await.unwrap(); // transition from update to sum state
    assert!(state_machine.is_sum2());

    let (req, response_rx) = gen_sum2_request(sum_pk.clone());
    let _ = request_tx.send(req);
    state_machine = state_machine.next().await.unwrap(); // transition from sum2 to unmasked state
    assert!(response_rx.await.is_ok());
    assert!(state_machine.is_unmask());

    state_machine = state_machine.next().await.unwrap(); // transition from unmasked to idle state
    assert!(state_machine.is_idle());

    drop(request_tx);
    state_machine = state_machine.next().await.unwrap(); // transition from idle to sum state
    assert!(state_machine.is_sum());

    state_machine = state_machine.next().await.unwrap(); // transition from sum to error state
    assert!(state_machine.is_error());

    state_machine = state_machine.next().await.unwrap(); // transition from error to shutdown state
    assert!(state_machine.is_shutdown());
    assert!(state_machine.next().await.is_none())
}
