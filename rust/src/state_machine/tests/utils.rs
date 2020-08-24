use crate::{
    client::{Participant, Task},
    common::RoundSeed,
    crypto::ByteObject,
    mask::config::{BoundType, DataType, GroupType, ModelType},
    settings::{MaskSettings, ModelSettings, PetSettings},
    state_machine::{
        coordinator::CoordinatorState,
        events::{EventPublisher, EventSubscriber},
        phases::{PhaseName, Shared},
        requests::{RequestReceiver, RequestSender},
    },
};

#[cfg(feature = "metrics")]
use crate::metrics::MetricsSender;

use tracing_subscriber::*;

pub fn enable_logging() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();
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
        expected_participants: 10,
        ..Default::default()
    }
}

pub fn model_settings() -> ModelSettings {
    ModelSettings { size: 1 }
}

pub fn init_shared() -> (Shared, EventSubscriber, RequestSender) {
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
            #[cfg(feature = "metrics")]
            MetricsSender(),
        ),
        event_subscriber,
        request_tx,
    )
}
