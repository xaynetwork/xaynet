//! Coordinator state and round parameter types.

use serde::{Deserialize, Serialize};

use crate::settings::{
    MaskSettings,
    ModelSettings,
    PetSettings,
    PetSettingsCount,
    PetSettingsSum,
    PetSettingsSum2,
    PetSettingsTime,
    PetSettingsUpdate,
};
use xaynet_core::{
    common::{RoundParameters, RoundSeed},
    crypto::{ByteObject, EncryptKeyPair},
    mask::MaskConfig,
};

/// The phase count parameters.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CountParameters {
    /// The minimal number of required messages.
    pub min: u64,
    /// The maximal number of accepted messages.
    pub max: u64,
}

impl From<PetSettingsCount> for CountParameters {
    fn from(count: PetSettingsCount) -> Self {
        let PetSettingsCount { min, max } = count;
        Self { min, max }
    }
}

/// The phase time parameters.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeParameters {
    /// The minimal amount of time (in seconds) reserved for processing messages.
    pub min: u64,
    /// The maximal amount of time (in seconds) permitted for processing messages.
    pub max: u64,
}

impl From<PetSettingsTime> for TimeParameters {
    fn from(time: PetSettingsTime) -> Self {
        let PetSettingsTime { min, max } = time;
        Self { min, max }
    }
}

/// The phase parameters.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhaseParameters {
    /// The number of messages.
    pub count: CountParameters,
    /// The amount of time for processing messages.
    pub time: TimeParameters,
}

impl From<PetSettingsSum> for PhaseParameters {
    fn from(sum: PetSettingsSum) -> Self {
        let PetSettingsSum { count, time, .. } = sum;
        Self {
            count: count.into(),
            time: time.into(),
        }
    }
}

impl From<PetSettingsUpdate> for PhaseParameters {
    fn from(update: PetSettingsUpdate) -> Self {
        let PetSettingsUpdate { count, time, .. } = update;
        Self {
            count: count.into(),
            time: time.into(),
        }
    }
}

impl From<PetSettingsSum2> for PhaseParameters {
    fn from(sum2: PetSettingsSum2) -> Self {
        let PetSettingsSum2 { count, time } = sum2;
        Self {
            count: count.into(),
            time: time.into(),
        }
    }
}

/// The coordinator state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoordinatorState {
    /// The credentials of the coordinator.
    pub keys: EncryptKeyPair,
    /// Internal ID used to identify a round
    pub round_id: u64,
    /// The round parameters.
    pub round_params: RoundParameters,
    /// The sum phase parameters.
    pub sum: PhaseParameters,
    /// The update phase parameters.
    pub update: PhaseParameters,
    /// The sum2 phase parameters.
    pub sum2: PhaseParameters,
}

impl CoordinatorState {
    pub fn new(
        pet_settings: PetSettings,
        mask_settings: MaskSettings,
        model_settings: ModelSettings,
    ) -> Self {
        let keys = EncryptKeyPair::generate();
        let round_params = RoundParameters {
            pk: keys.public,
            sum: pet_settings.sum.prob,
            update: pet_settings.update.prob,
            seed: RoundSeed::zeroed(),
            mask_config: MaskConfig::from(mask_settings).into(),
            model_length: model_settings.length,
        };
        let round_id = 0;
        Self {
            keys,
            round_params,
            round_id,
            sum: pet_settings.sum.into(),
            update: pet_settings.update.into(),
            sum2: pet_settings.sum2.into(),
        }
    }
}
