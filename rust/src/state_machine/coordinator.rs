//! Coordinator state and round parameter types.
use std::collections::HashMap;

use sodiumoxide::{self, crypto::box_};

use crate::{
    crypto::{encrypt::EncryptKeyPair, ByteObject},
    mask::{config::MaskConfig, object::MaskObject},
    settings::{MaskSettings, ModelSettings, PetSettings},
    state_machine::{
        events::{EventPublisher, EventSubscriber},
        phases::PhaseName,
    },
    CoordinatorPublicKey,
};

/// The round parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoundParameters {
    /// The public key of the coordinator used for encryption.
    pub pk: CoordinatorPublicKey,
    /// Fraction of participants to be selected for the sum task.
    pub sum: f64,
    /// Fraction of participants to be selected for the update task.
    pub update: f64,
    /// The random round seed.
    pub seed: RoundSeed,
}

impl Default for RoundParameters {
    fn default() -> Self {
        Self {
            pk: CoordinatorPublicKey::zeroed(),
            sum: 0.0,
            update: 0.0,
            seed: RoundSeed::zeroed(),
        }
    }
}

/// The coordinator state.
#[derive(Debug)]
pub struct CoordinatorState {
    /// The credentials of the coordinator.
    pub keys: EncryptKeyPair,
    /// The round parameters.
    pub round_params: RoundParameters,
    /// The minimum of required sum/sum2 messages.
    pub min_sum_count: usize,
    /// The minimum of required update messages.
    pub min_update_count: usize,
    /// The minimum time (in seconds) reserved for processing sum/sum2 messages.
    pub min_sum_time: u64,
    /// The minimum time (in seconds) reserved for processing update messages.
    pub min_update_time: u64,
    /// The maximum time (in seconds) permitted for processing sum/sum2 messages.
    pub max_sum_time: u64,
    /// The maximum time (in seconds) permitted for processing update messages.
    pub max_update_time: u64,
    /// The number of expected participants.
    pub expected_participants: usize,
    /// The masking configuration.
    pub mask_config: MaskConfig,
    /// The size of the model.
    pub model_size: usize,
    /// The event publisher.
    pub events: EventPublisher,
}

impl CoordinatorState {
    pub fn new(
        pet_settings: PetSettings,
        mask_settings: MaskSettings,
        model_settings: ModelSettings,
    ) -> (Self, EventSubscriber) {
        let keys = EncryptKeyPair::generate();
        let round_params = RoundParameters {
            pk: keys.public,
            sum: pet_settings.sum,
            update: pet_settings.update,
            seed: RoundSeed::zeroed(),
        };
        let phase = PhaseName::Idle;

        let (publisher, subscriber) =
            EventPublisher::init(keys.clone(), round_params.clone(), phase);

        let coordinator_state = Self {
            keys,
            round_params,
            events: publisher,
            min_sum_count: pet_settings.min_sum_count,
            min_update_count: pet_settings.min_update_count,
            min_sum_time: pet_settings.min_sum_time,
            min_update_time: pet_settings.min_update_time,
            max_sum_time: pet_settings.max_sum_time,
            max_update_time: pet_settings.max_update_time,
            expected_participants: pet_settings.expected_participants,
            mask_config: mask_settings.into(),
            model_size: model_settings.size,
        };
        (coordinator_state, subscriber)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// A seed for a round.
pub struct RoundSeed(box_::Seed);

impl ByteObject for RoundSeed {
    const LENGTH: usize = box_::SEEDBYTES;

    /// Creates a round seed from a slice of bytes.
    ///
    /// # Errors
    /// Fails if the length of the input is invalid.
    fn from_slice(bytes: &[u8]) -> Option<Self> {
        box_::Seed::from_slice(bytes).map(Self)
    }

    /// Creates a round seed initialized to zero.
    fn zeroed() -> Self {
        Self(box_::Seed([0_u8; Self::LENGTH]))
    }

    /// Gets the round seed as a slice.
    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// A dictionary created during the sum2 phase of the protocol. It counts the model masks
/// represented by their hashes.
pub type MaskDict = HashMap<MaskObject, usize>;
