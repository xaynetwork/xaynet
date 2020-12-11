use tracing_subscriber::{EnvFilter, FmtSubscriber};

use crate::{
    settings::{MaskSettings, ModelSettings, PetSettings},
    state_machine::{
        coordinator::CoordinatorState,
        events::{EventPublisher, EventSubscriber, ModelUpdate},
        phases::{PhaseName, Shared},
        requests::{RequestReceiver, RequestSender},
    },
    storage::{CoordinatorStorage, ModelStorage, Store},
};
use xaynet_core::{
    common::RoundParameters,
    crypto::{ByteObject, EncryptKeyPair, Signature, SigningKeyPair},
    mask::{
        Aggregation,
        BoundType,
        DataType,
        GroupType,
        MaskConfig,
        MaskConfigPair,
        MaskObject,
        MaskSeed,
        Masker,
        Model,
        ModelType,
    },
    message::{Message, Payload, Sum, Sum2, Update},
    LocalSeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    UpdateSeedDict,
};

pub fn enable_logging() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .try_init();
}

pub struct Participant {
    pub keys: SigningKeyPair,
    pub round_params: RoundParameters,
    pub mask_settings: MaskConfigPair,
    // sum participants have an ephemeral key pair for the round they
    // are taking part in
    pub ephm_keys: EncryptKeyPair,
}

impl Participant {
    pub fn new(round_params: RoundParameters, mask_settings: MaskSettings) -> Self {
        let mask_config: MaskConfig = mask_settings.into();
        Participant {
            round_params,
            mask_settings: mask_config.into(),
            keys: SigningKeyPair::generate(),
            ephm_keys: EncryptKeyPair::generate(),
        }
    }

    pub fn sign(&self, data: &[u8]) -> Signature {
        let sk = &self.keys.secret;
        let seed = self.round_params.seed.as_slice();
        sk.sign_detached(&[seed, data].concat())
    }

    pub fn sum_signature(&self) -> Signature {
        self.sign(b"sum")
    }

    pub fn update_signature(&self) -> Signature {
        self.sign(b"update")
    }

    pub fn is_sum_eligible(&self) -> bool {
        let signature = self.sum_signature();
        signature.is_eligible(self.round_params.sum)
    }

    pub fn is_update_eligible(&self) -> bool {
        if self.is_sum_eligible() {
            return false;
        }
        let signature = self.update_signature();
        signature.is_eligible(self.round_params.update)
    }

    // Sum methods
    pub fn compose_sum_message(&self) -> Message {
        let payload = Sum {
            sum_signature: self.sum_signature(),
            ephm_pk: self.ephm_keys.public,
        };
        Message::new_sum(self.keys.public, self.round_params.pk, payload)
    }

    // Update methods
    pub fn compute_masked_model(&self, model: &Model, scalar: f64) -> (MaskSeed, MaskObject) {
        let masker = Masker::new(self.mask_settings);
        masker.mask(scalar, model)
    }

    pub fn build_seed_dict(sum_dict: &SumDict, mask_seed: &MaskSeed) -> LocalSeedDict {
        sum_dict
            .iter()
            .map(|(pk, ephm_pk)| (*pk, mask_seed.encrypt(&ephm_pk)))
            .collect()
    }

    pub fn compose_update_message(
        &self,
        masked_model: MaskObject,
        local_seed_dict: LocalSeedDict,
    ) -> Message {
        let payload = Update {
            sum_signature: self.sum_signature(),
            update_signature: self.update_signature(),
            masked_model,
            local_seed_dict,
        };
        Message::new_update(self.keys.public, self.round_params.pk, payload)
    }

    // Sum2 methods
    pub fn decrypt_seeds(&self, seed_dict: &UpdateSeedDict) -> Vec<MaskSeed> {
        let (pk, sk) = (self.ephm_keys.public, self.ephm_keys.secret.clone());
        seed_dict
            .iter()
            .map(|(_, seed)| seed.decrypt(&pk, &sk).unwrap())
            .collect()
    }

    pub fn aggregate_masks(&self, mask_length: usize, seeds: &[MaskSeed]) -> Aggregation {
        let mut aggregation = Aggregation::new(self.mask_settings, mask_length);
        for seed in seeds {
            let mask = seed.derive_mask(mask_length, self.mask_settings);
            aggregation.validate_aggregation(&mask).unwrap();
            aggregation.aggregate(mask);
        }
        aggregation
    }

    pub fn compose_sum2_message(&self, model_mask: MaskObject) -> Message {
        let payload = Sum2 {
            sum_signature: self.sum_signature(),
            model_mask,
        };
        Message::new_sum2(self.keys.public, self.round_params.pk, payload)
    }
}

pub fn generate_summer(round_params: RoundParameters) -> Participant {
    loop {
        let participant = Participant::new(round_params.clone(), mask_settings());
        if participant.is_sum_eligible() {
            return participant;
        }
    }
}

pub fn generate_updater(round_params: RoundParameters) -> Participant {
    loop {
        let participant = Participant::new(round_params.clone(), mask_settings());
        if participant.is_update_eligible() {
            return participant;
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

pub fn mask_config() -> MaskConfigPair {
    Into::<MaskConfig>::into(mask_settings()).into()
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
    }
}

pub fn model_settings() -> ModelSettings {
    ModelSettings { length: 1 }
}

pub fn init_shared<C, M>(
    coordinator_state: CoordinatorState,
    store: Store<C, M>,
) -> (Shared<C, M>, RequestSender, EventSubscriber)
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    let (event_publisher, event_subscriber) = EventPublisher::init(
        coordinator_state.round_id,
        coordinator_state.keys.clone(),
        coordinator_state.round_params.clone(),
        PhaseName::Idle,
        ModelUpdate::Invalidate,
    );

    let (request_rx, request_tx) = RequestReceiver::new();
    (
        Shared::new(coordinator_state, event_publisher, request_rx, store),
        request_tx,
        event_subscriber,
    )
}

pub fn coordinator_state() -> CoordinatorState {
    CoordinatorState::new(pet_settings(), mask_settings(), model_settings())
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
