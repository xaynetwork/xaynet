use std::collections::HashMap;

use sodiumoxide::{self, crypto::box_, randombytes::randombytes};

use crate::{
    crypto::{ByteObject, KeyPair},
    mask::{MaskConfig, MaskObject},
    settings::{MaskSettings, PetSettings},
    state_machine::events::{EventPublisher, EventSubscriber, PhaseEvent},
    CoordinatorPublicKey,
};

pub type RoundId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundParameters {
    pub id: RoundId,
    pub pk: CoordinatorPublicKey,
    pub sum: f64,
    pub update: f64,
    pub seed: RoundSeed,
}

pub struct CoordinatorState {
    pub keys: KeyPair,
    pub round_params: RoundParameters,
    pub min_sum: usize,
    pub min_update: usize,
    pub expected_participants: usize,
    pub mask_config: MaskConfig,
    pub events: EventPublisher,
}

impl CoordinatorState {
    pub fn new(pet_settings: PetSettings, mask_settings: MaskSettings) -> (Self, EventSubscriber) {
        let keys = KeyPair::generate();
        let round_params = RoundParameters {
            id: 0,
            pk: keys.public,
            sum: pet_settings.sum,
            update: pet_settings.update,
            seed: RoundSeed::zeroed(),
        };
        let phase = PhaseEvent::Idle;

        let (publisher, subscriber) =
            EventPublisher::init(keys.clone(), round_params.clone(), phase);

        let coordinator_state = Self {
            keys,
            round_params,
            events: publisher,
            min_sum: pet_settings.min_sum,
            min_update: pet_settings.min_update,
            expected_participants: pet_settings.expected_participants,
            mask_config: mask_settings.into(),
        };
        (coordinator_state, subscriber)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// A seed for a round.
pub struct RoundSeed(box_::Seed);

impl ByteObject for RoundSeed {
    /// Create a round seed from a slice of bytes. Fails if the length of the input is invalid.
    fn from_slice(bytes: &[u8]) -> Option<Self> {
        box_::Seed::from_slice(bytes).map(Self)
    }

    /// Create a round seed initialized to zero.
    fn zeroed() -> Self {
        Self(box_::Seed([0_u8; Self::LENGTH]))
    }

    /// Get the round seed as a slice.
    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl RoundSeed {
    /// Get the number of bytes of a round seed.
    pub const LENGTH: usize = box_::SEEDBYTES;

    /// Generate a random round seed.
    pub fn generate() -> Self {
        // safe unwrap: length of slice is guaranteed by constants
        Self::from_slice_unchecked(randombytes(Self::LENGTH).as_slice())
    }
}

/// A dictionary created during the sum2 phase of the protocol. It counts the model masks
/// represented by their hashes.
pub type MaskDict = HashMap<MaskObject, usize>;
