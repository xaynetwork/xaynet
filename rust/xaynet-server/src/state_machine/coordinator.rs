//! Coordinator state and round parameter types.

use serde::{Deserialize, Serialize};

use crate::settings::{MaskSettings, ModelSettings, PetSettings};
use xaynet_core::{
    common::{RoundParameters, RoundSeed},
    crypto::{ByteObject, EncryptKeyPair},
    mask::MaskConfig,
};

/// The coordinator state.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CoordinatorState {
    /// The credentials of the coordinator.
    pub keys: EncryptKeyPair,
    /// Internal ID used to identify a round
    pub round_id: u64,
    /// The round parameters.
    pub round_params: RoundParameters,
    /// The minimum of required sum messages.
    pub min_sum_count: u64,
    /// The minimum of required update messages.
    pub min_update_count: u64,
    /// The minimum of required sum2 messages.
    pub min_sum2_count: u64,
    /// The maximum of accepted sum messages.
    pub max_sum_count: u64,
    /// The maximum of accepted update messages.
    pub max_update_count: u64,
    /// The maximum of accepted sum2 messages.
    pub max_sum2_count: u64,
    /// The minimum time (in seconds) reserved for processing sum messages.
    pub min_sum_time: u64,
    /// The minimum time (in seconds) reserved for processing update messages.
    pub min_update_time: u64,
    /// The minimum time (in seconds) reserved for processing sum2 messages.
    pub min_sum2_time: u64,
    /// The maximum time (in seconds) permitted for processing sum messages.
    pub max_sum_time: u64,
    /// The maximum time (in seconds) permitted for processing update messages.
    pub max_update_time: u64,
    /// The maximum time (in seconds) permitted for processing sum2 messages.
    pub max_sum2_time: u64,
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
            min_sum_count: pet_settings.sum.count.min,
            min_update_count: pet_settings.update.count.min,
            min_sum2_count: pet_settings.sum2.count.min,
            max_sum_count: pet_settings.sum.count.max,
            max_update_count: pet_settings.update.count.max,
            max_sum2_count: pet_settings.sum2.count.max,
            min_sum_time: pet_settings.sum.time.min,
            min_update_time: pet_settings.update.time.min,
            min_sum2_time: pet_settings.sum2.time.min,
            max_sum_time: pet_settings.sum.time.max,
            max_update_time: pet_settings.update.time.max,
            max_sum2_time: pet_settings.sum2.time.max,
        }
    }
}
