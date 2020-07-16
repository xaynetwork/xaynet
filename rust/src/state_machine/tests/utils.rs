use crate::{
    client::{Participant, Task},
    crypto::{encrypt::EncryptKeyPair, sign::SigningKeyPair, ByteObject},
    mask::{
        config::{BoundType, DataType, GroupType, ModelType},
        object::MaskObject,
        seed::{EncryptedMaskSeed, MaskSeed},
    },
    settings::MaskSettings,
    state_machine::{
        coordinator::RoundSeed,
        requests::{Request, Sum2Request, SumRequest, UpdateRequest},
    },
    LocalSeedDict,
    PetError,
    SumParticipantPublicKey,
};

use tokio::sync::oneshot;
use tracing_subscriber::*;

pub fn enable_logging() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();
}

pub fn gen_sum_request() -> (
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

pub fn gen_update_request(
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

pub fn gen_mask() -> MaskObject {
    let seed = MaskSeed::generate();
    let mask = seed.derive_mask(10, mask_settings().into());
    mask
}

pub fn gen_sum2_request(
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

pub fn generate_summer(seed: &RoundSeed, sum_ratio: f64, update_ratio: f64) -> Participant {
    loop {
        let mut participant = Participant::new().unwrap();
        participant.compute_signatures(seed.as_slice());
        match participant.check_task(sum_ratio, update_ratio) {
            Task::Sum => return participant,
            _ => {}
        }
    }
}

pub fn generate_updater(seed: &RoundSeed, sum_ratio: f64, update_ratio: f64) -> Participant {
    loop {
        let mut participant = Participant::new().unwrap();
        participant.compute_signatures(seed.as_slice());
        match participant.check_task(sum_ratio, update_ratio) {
            Task::Update => return participant,
            _ => {}
        }
    }
}

pub fn mask_settings() -> MaskSettings {
    MaskSettings {
        group_type: GroupType::Prime,
        data_type: DataType::F32,
        bound_type: BoundType::B0,
        model_type: ModelType::M3,
    }
}
