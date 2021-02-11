use xaynet_core::{common::RoundSeed, crypto::EncryptKeyPair, mask::MaskConfig};

use crate::state_machine::coordinator::CoordinatorState;

use super::utils::{mask_settings, model_settings, pet_settings};

pub struct CoordinatorStateBuilder {
    state: CoordinatorState,
}

#[allow(dead_code)]
impl CoordinatorStateBuilder {
    pub fn new() -> Self {
        Self {
            state: CoordinatorState::new(pet_settings(), mask_settings(), model_settings()),
        }
    }

    pub fn build(self) -> CoordinatorState {
        self.state
    }

    pub fn with_keys(mut self, keys: EncryptKeyPair) -> Self {
        self.state.round_params.pk = keys.public;
        self.state.keys = keys;
        self
    }

    pub fn with_round_id(mut self, id: u64) -> Self {
        self.state.round_id = id;
        self
    }

    pub fn with_sum_probability(mut self, prob: f64) -> Self {
        self.state.round_params.sum = prob;
        self
    }

    pub fn with_update_probability(mut self, prob: f64) -> Self {
        self.state.round_params.update = prob;
        self
    }

    pub fn with_seed(mut self, seed: RoundSeed) -> Self {
        self.state.round_params.seed = seed;
        self
    }

    pub fn with_sum_count_min(mut self, min: u64) -> Self {
        self.state.sum.count.min = min;
        self
    }

    pub fn with_sum_count_max(mut self, max: u64) -> Self {
        self.state.sum.count.max = max;
        self
    }

    pub fn with_mask_config(mut self, mask_config: MaskConfig) -> Self {
        self.state.round_params.mask_config = mask_config.into();
        self
    }

    pub fn with_update_count_min(mut self, min: u64) -> Self {
        self.state.update.count.min = min;
        self
    }

    pub fn with_update_count_max(mut self, max: u64) -> Self {
        self.state.update.count.max = max;
        self
    }

    pub fn with_sum2_count_min(mut self, min: u64) -> Self {
        self.state.sum2.count.min = min;
        self
    }

    pub fn with_sum2_count_max(mut self, max: u64) -> Self {
        self.state.sum2.count.max = max;
        self
    }

    pub fn with_model_length(mut self, model_length: usize) -> Self {
        self.state.round_params.model_length = model_length;
        self
    }

    pub fn with_sum_time_min(mut self, min: u64) -> Self {
        self.state.sum.time.min = min;
        self
    }

    pub fn with_sum_time_max(mut self, max: u64) -> Self {
        self.state.sum.time.max = max;
        self
    }

    pub fn with_update_time_min(mut self, min: u64) -> Self {
        self.state.update.time.min = min;
        self
    }

    pub fn with_update_time_max(mut self, max: u64) -> Self {
        self.state.update.time.max = max;
        self
    }

    pub fn with_sum2_time_min(mut self, min: u64) -> Self {
        self.state.sum2.time.min = min;
        self
    }

    pub fn with_sum2_time_max(mut self, max: u64) -> Self {
        self.state.sum2.time.max = max;
        self
    }
}
