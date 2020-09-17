use xaynet_core::{
    common::RoundSeed,
    crypto::ByteObject,
    mask::{BoundType, DataType, GroupType, MaskObject, ModelType},
    message::{Message, Payload, Sum, Update},
    LocalSeedDict,
    SumParticipantEphemeralPublicKey,
};

use crate::{
    settings::{MaskSettings, ModelSettings, PetSettings},
    state_machine::{
        coordinator::CoordinatorState,
        events::{EventPublisher, EventSubscriber},
        phases::{PhaseName, Shared},
        requests::{RequestReceiver, RequestSender},
    },
    storage::redis,
};
use xaynet_client::{Participant, Task};

#[cfg(feature = "metrics")]
use crate::metrics::MetricsSender;

use tracing_subscriber::*;

pub fn enable_logging() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .try_init();
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

pub fn pet_settings() -> PetSettings {
    PetSettings {
        sum: 0.4,
        update: 0.5,
        min_sum_count: 1,
        min_update_count: 3,
        min_sum_time: 1,
        max_sum_time: 2,
        min_update_time: 1,
        max_update_time: 2,
        ..Default::default()
    }
}

pub fn model_settings() -> ModelSettings {
    ModelSettings { size: 1 }
}

pub async fn init_shared() -> (Shared, EventSubscriber, RequestSender, redis::Client) {
    let redis = redis::Client::new("redis://127.0.0.1/", 10).await.unwrap();
    redis.connection().await.flush_db().await.unwrap();

    let coordinator_state =
        CoordinatorState::new(pet_settings(), mask_settings(), model_settings());

    let (event_publisher, event_subscriber) = EventPublisher::init(
        coordinator_state.round_id,
        coordinator_state.keys.clone(),
        coordinator_state.round_params.clone(),
        PhaseName::Idle,
    );

    let (request_rx, request_tx) = RequestReceiver::new();
    (
        Shared::new(
            coordinator_state,
            event_publisher,
            request_rx,
            redis.clone(),
            #[cfg(feature = "metrics")]
            MetricsSender(),
        ),
        event_subscriber,
        request_tx,
        redis,
    )
}

/// Extract the ephemeral public key from a sum message.
///
/// # Panic
///
/// Panic if this message is not a sum message
pub fn ephm_pk(msg: &Message) -> SumParticipantEphemeralPublicKey {
    if let Payload::Sum(Sum { ephm_pk, .. }) = &msg.payload {
        *ephm_pk
    } else {
        panic!("not a sum message");
    }
}

/// Extract the masked model from an update message
///
/// # Panic
///
/// Panic if this message is not an update message
pub fn masked_model(msg: &Message) -> MaskObject {
    if let Payload::Update(Update { masked_model, .. }) = &msg.payload {
        masked_model.clone()
    } else {
        panic!("not an update message");
    }
}

/// Extract the masked scalar from an update message
///
/// # Panic
///
/// Panic if this message is not an update message
pub fn masked_scalar(msg: &Message) -> MaskObject {
    if let Payload::Update(Update { masked_scalar, .. }) = &msg.payload {
        masked_scalar.clone()
    } else {
        panic!("not an update message");
    }
}

/// Extract the local seed dictioanry from an update message
///
/// # Panic
///
/// Panic if this message is not an update message
pub fn local_seed_dict(msg: &Message) -> LocalSeedDict {
    if let Payload::Update(Update {
        local_seed_dict, ..
    }) = &msg.payload
    {
        local_seed_dict.clone()
    } else {
        panic!("not an update message");
    }
}
