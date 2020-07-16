use crate::{
    client::{Participant, Task},
    crypto::ByteObject,
    mask::config::{BoundType, DataType, GroupType, ModelType},
    settings::MaskSettings,
    state_machine::coordinator::RoundSeed,
};

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
