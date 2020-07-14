use crate::{
    crypto::{encrypt::EncryptKeyPair, sign::SigningKeyPair, ByteObject},
    mask::{
        config::{BoundType, DataType, GroupType, MaskConfig, ModelType},
        object::MaskObject,
        seed::{EncryptedMaskSeed, MaskSeed},
    },
    state_machine::requests::{Request, Sum2Request, SumRequest, UpdateRequest},
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
