use crate::{
    state_machine::{
        coordinator::CoordinatorState,
        events::EventSubscriber,
        phases::{self, Phase, PhaseState},
        requests::RequestSender,
        tests::utils,
        StateMachine,
    },
    storage::Storage,
};
use xaynet_core::{common::RoundSeed, crypto::EncryptKeyPair, mask::MaskConfig};

pub struct StateMachineBuilder<P, S>
where
    S: Storage,
{
    coordinator_state: CoordinatorState,
    phase_state: P,
    store: S,
}

impl<S> StateMachineBuilder<phases::Idle, S>
where
    S: Storage,
{
    pub fn new(store: S) -> Self {
        let coordinator_state = utils::coordinator_state();
        let phase_state = phases::Idle;
        Self {
            coordinator_state,
            phase_state,
            store,
        }
    }
}

impl<P, S> StateMachineBuilder<P, S>
where
    PhaseState<P, S>: Phase<S>,
    StateMachine<S>: From<PhaseState<P, S>>,
    S: Storage,
{
    pub fn build(self) -> (StateMachine<S>, RequestSender, EventSubscriber) {
        let Self {
            coordinator_state,
            phase_state,
            store,
        } = self;

        let (mut shared, request_tx, event_subscriber) =
            utils::init_shared(coordinator_state, store);

        // Make sure the events that the listeners have are up to date
        let events = &mut shared.events;
        events.broadcast_keys(shared.state.keys.clone());
        events.broadcast_params(shared.state.round_params.clone());
        events.broadcast_phase(<PhaseState<P, _> as Phase<_>>::NAME);
        // Also re-emit the other events in case the round ID changed
        let model = event_subscriber.model_listener().get_latest().event;
        events.broadcast_model(model);
        let sum_dict = event_subscriber.sum_dict_listener().get_latest().event;
        events.broadcast_sum_dict(sum_dict);
        let seed_dict = event_subscriber.seed_dict_listener().get_latest().event;
        events.broadcast_seed_dict(seed_dict);

        let state = PhaseState {
            private: phase_state,
            shared,
        };

        let state_machine = StateMachine::from(state);
        (state_machine, request_tx, event_subscriber)
    }

    #[allow(dead_code)]
    pub fn with_keys(mut self, keys: EncryptKeyPair) -> Self {
        self.coordinator_state.round_params.pk = keys.public;
        self.coordinator_state.keys = keys;
        self
    }

    pub fn with_round_id(mut self, id: u64) -> Self {
        self.coordinator_state.round_id = id;
        self
    }

    pub fn with_sum_probability(mut self, prob: f64) -> Self {
        self.coordinator_state.round_params.sum = prob;
        self
    }

    pub fn with_update_probability(mut self, prob: f64) -> Self {
        self.coordinator_state.round_params.update = prob;
        self
    }

    pub fn with_seed(mut self, seed: RoundSeed) -> Self {
        self.coordinator_state.round_params.seed = seed;
        self
    }

    pub fn with_sum_count_min(mut self, min: u64) -> Self {
        self.coordinator_state.sum.count.min = min;
        self
    }

    pub fn with_sum_count_max(mut self, max: u64) -> Self {
        self.coordinator_state.sum.count.max = max;
        self
    }

    pub fn with_mask_config(mut self, mask_config: MaskConfig) -> Self {
        self.coordinator_state.round_params.mask_config = mask_config.into();
        self
    }

    pub fn with_update_count_min(mut self, min: u64) -> Self {
        self.coordinator_state.update.count.min = min;
        self
    }

    pub fn with_update_count_max(mut self, max: u64) -> Self {
        self.coordinator_state.update.count.max = max;
        self
    }

    pub fn with_sum2_count_min(mut self, min: u64) -> Self {
        self.coordinator_state.sum2.count.min = min;
        self
    }

    pub fn with_sum2_count_max(mut self, max: u64) -> Self {
        self.coordinator_state.sum2.count.max = max;
        self
    }

    pub fn with_model_length(mut self, model_length: usize) -> Self {
        self.coordinator_state.round_params.model_length = model_length;
        self
    }

    pub fn with_sum_time_min(mut self, min: u64) -> Self {
        self.coordinator_state.sum.time.min = min;
        self
    }

    pub fn with_sum_time_max(mut self, max: u64) -> Self {
        self.coordinator_state.sum.time.max = max;
        self
    }

    pub fn with_update_time_min(mut self, min: u64) -> Self {
        self.coordinator_state.update.time.min = min;
        self
    }

    pub fn with_update_time_max(mut self, max: u64) -> Self {
        self.coordinator_state.update.time.max = max;
        self
    }

    pub fn with_sum2_time_min(mut self, min: u64) -> Self {
        self.coordinator_state.sum2.time.min = min;
        self
    }

    pub fn with_sum2_time_max(mut self, max: u64) -> Self {
        self.coordinator_state.sum2.time.max = max;
        self
    }

    pub fn with_phase<State>(self, phase_state: State) -> StateMachineBuilder<State, S> {
        let Self {
            coordinator_state,
            store,
            ..
        } = self;
        StateMachineBuilder {
            coordinator_state,
            phase_state,
            store,
        }
    }
}
