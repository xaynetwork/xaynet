//! State machine misc test utilities.

use std::fmt::Debug;

use tokio::sync::mpsc;
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use xaynet_core::{
    common::RoundParameters,
    crypto::{ByteObject, EncryptKeyPair, PublicEncryptKey, PublicSigningKey},
    mask::{BoundType, DataType, GroupType, MaskObject, ModelType},
    message::{Message, Sum, Sum2, Update},
    LocalSeedDict,
    ParticipantTaskSignature,
    SeedDict,
    SumDict,
};

use crate::{
    settings::{
        MaskSettings,
        ModelSettings,
        PetSettings,
        PetSettingsCount,
        PetSettingsSum,
        PetSettingsSum2,
        PetSettingsTime,
        PetSettingsUpdate,
    },
    state_machine::{
        coordinator::CoordinatorState,
        events::{DictionaryUpdate, Event, EventPublisher, EventSubscriber, ModelUpdate},
        phases::{PhaseName, Shared},
        requests::{RequestReceiver, RequestSender},
    },
    storage::tests::utils::create_mask,
};

use super::WARNING;

pub fn enable_logging() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .try_init();
}

pub fn pet_settings() -> PetSettings {
    PetSettings {
        sum: PetSettingsSum {
            prob: 0.4,
            count: PetSettingsCount { min: 1, max: 100 },
            time: PetSettingsTime { min: 1, max: 2 },
        },
        update: PetSettingsUpdate {
            prob: 0.5,
            count: PetSettingsCount { min: 3, max: 1000 },
            time: PetSettingsTime { min: 1, max: 2 },
        },
        sum2: PetSettingsSum2 {
            count: PetSettingsCount { min: 1, max: 100 },
            time: PetSettingsTime { min: 1, max: 2 },
        },
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

pub fn model_settings() -> ModelSettings {
    ModelSettings { length: 1 }
}

pub fn init_shared<T>(
    coordinator_state: CoordinatorState,
    store: T,
    event_publisher: EventPublisher,
) -> (Shared<T>, RequestSender) {
    let (request_rx, request_tx) = RequestReceiver::new();
    (
        Shared::new(coordinator_state, event_publisher, request_rx, store),
        request_tx,
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventSnapshot {
    pub keys: Event<EncryptKeyPair>,
    pub params: Event<RoundParameters>,
    pub phase: Event<PhaseName>,
    pub model: Event<ModelUpdate>,
    pub sum_dict: Event<DictionaryUpdate<SumDict>>,
    pub seed_dict: Event<DictionaryUpdate<SeedDict>>,
}

impl From<&EventSubscriber> for EventSnapshot {
    fn from(event_subscriber: &EventSubscriber) -> Self {
        Self {
            keys: event_subscriber.keys_listener().get_latest(),
            params: event_subscriber.params_listener().get_latest(),
            phase: event_subscriber.phase_listener().get_latest(),
            model: event_subscriber.model_listener().get_latest(),
            sum_dict: event_subscriber.sum_dict_listener().get_latest(),
            seed_dict: event_subscriber.seed_dict_listener().get_latest(),
        }
    }
}

pub fn assert_event_updated_with_id<T: Debug + PartialEq>(event1: &Event<T>, event2: &Event<T>) {
    assert_ne!(event1.round_id, event2.round_id);
    assert_ne!(event1.event, event2.event);
}

pub fn assert_event_updated<T: Debug + PartialEq>(event1: &Event<T>, event2: &Event<T>) {
    assert_eq!(event1.round_id, event2.round_id);
    assert_ne!(event1.event, event2.event);
}

pub fn compose_sum_message() -> Message {
    let payload = Sum {
        sum_signature: ParticipantTaskSignature::zeroed(),
        ephm_pk: PublicEncryptKey::zeroed(),
    };
    Message::new_sum(
        PublicSigningKey::zeroed(),
        PublicEncryptKey::zeroed(),
        payload,
    )
}

pub fn compose_update_message(masked_model: MaskObject) -> Message {
    let payload = Update {
        sum_signature: ParticipantTaskSignature::zeroed(),
        update_signature: ParticipantTaskSignature::zeroed(),
        masked_model,
        local_seed_dict: LocalSeedDict::new(),
    };
    Message::new_update(
        PublicSigningKey::zeroed(),
        PublicEncryptKey::zeroed(),
        payload,
    )
}

pub fn compose_sum2_message() -> Message {
    let payload = Sum2 {
        sum_signature: ParticipantTaskSignature::zeroed(),
        model_mask: create_mask(1, 1),
    };
    Message::new_sum2(
        PublicSigningKey::zeroed(),
        PublicEncryptKey::zeroed(),
        payload,
    )
}

pub fn send_sum_messages(n: u32, request_tx: RequestSender) {
    for _ in 0..n {
        let request = request_tx.clone();
        tokio::spawn(async move { request.msg(&compose_sum_message()).await });
    }
}

#[allow(dead_code)]
pub fn send_sum_messages_with_latch(n: u32, request_tx: RequestSender, latch: Latch) {
    for _ in 0..n {
        let request = request_tx.clone();
        let l = latch.clone();
        tokio::spawn(async move {
            let _ = request.msg(&compose_sum_message()).await;
            l.release();
        });
    }
}

pub fn send_sum2_messages(n: u32, request_tx: RequestSender) {
    for _ in 0..n {
        let request = request_tx.clone();
        tokio::spawn(async move { request.msg(&compose_sum2_message()).await });
    }
}

pub fn send_update_messages(n: u32, request_tx: RequestSender) {
    let default_model = create_mask(1, 1);
    for _ in 0..n {
        let request = request_tx.clone();
        let masked_model = default_model.clone();
        tokio::spawn(async move { request.msg(&compose_update_message(masked_model)).await });
    }
}

pub fn send_update_messages_with_model(
    n: u32,
    request_tx: RequestSender,
    masked_model: MaskObject,
) {
    for _ in 0..n {
        let request = request_tx.clone();
        let moved_masked_model = masked_model.clone();
        tokio::spawn(async move {
            request
                .msg(&compose_update_message(moved_masked_model))
                .await
        });
    }
}

#[allow(dead_code)]
pub struct Readiness(mpsc::Receiver<()>);

#[allow(dead_code)]
#[derive(Clone)]
pub struct Latch(mpsc::Sender<()>);

#[allow(dead_code)]
impl Readiness {
    pub fn new() -> (Readiness, Latch) {
        let (sender, receiver) = mpsc::channel(1);
        (Readiness(receiver), Latch(sender))
    }

    pub async fn is_ready(&mut self) {
        let _ = self.0.recv().await;
    }
}

impl Latch {
    /// Releases this readiness latch.
    pub fn release(self) {
        drop(self);
    }
}

#[test]
fn test_initial_settings() {
    let pet = PetSettings {
        sum: PetSettingsSum {
            prob: 0.4,
            count: PetSettingsCount { min: 1, max: 100 },
            time: PetSettingsTime { min: 1, max: 2 },
        },
        update: PetSettingsUpdate {
            prob: 0.5,
            count: PetSettingsCount { min: 3, max: 1000 },
            time: PetSettingsTime { min: 1, max: 2 },
        },
        sum2: PetSettingsSum2 {
            count: PetSettingsCount { min: 1, max: 100 },
            time: PetSettingsTime { min: 1, max: 2 },
        },
    };

    assert_eq!(
        pet,
        pet_settings(),
        "the initial PetSettings have been changed. {}",
        WARNING
    );

    let mask = MaskSettings {
        group_type: GroupType::Prime,
        data_type: DataType::F32,
        bound_type: BoundType::B0,
        model_type: ModelType::M3,
    };

    assert_eq!(
        mask,
        mask_settings(),
        "the initial MaskSettings have been changed. {}",
        WARNING
    );

    let model = ModelSettings { length: 1 };

    assert_eq!(
        model,
        model_settings(),
        "the initial ModelSettings have been changed. {}",
        WARNING
    );
}
