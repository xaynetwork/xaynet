use crate::{
    state_machine::{
        coordinator::CoordinatorState,
        events::EventSubscriber,
        phases::{self, Phase, PhaseState},
        requests::RequestSender,
        tests::utils,
        StateMachine,
    },
    storage::{CoordinatorStorage, ModelStorage, Store},
};
use xaynet_core::{common::RoundSeed, crypto::EncryptKeyPair, mask::MaskConfig};

pub struct StateMachineBuilder<P, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    coordinator_state: CoordinatorState,
    phase_state: P,
    store: Store<C, M>,
}

impl<C, M> StateMachineBuilder<phases::Idle, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    pub fn new(store: Store<C, M>) -> Self {
        let coordinator_state = utils::coordinator_state();
        let phase_state = phases::Idle;
        Self {
            coordinator_state,
            phase_state,
            store,
        }
    }
}

impl<P, C, M> StateMachineBuilder<P, C, M>
where
    PhaseState<P, C, M>: Phase<C, M>,
    StateMachine<C, M>: From<PhaseState<P, C, M>>,
    C: CoordinatorStorage,
    M: ModelStorage,
{
    pub fn build(self) -> (StateMachine<C, M>, RequestSender, EventSubscriber) {
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
        events.broadcast_phase(<PhaseState<P, _, _> as Phase<_, _>>::NAME);
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

    pub fn with_sum_ratio(mut self, sum_ratio: f64) -> Self {
        self.coordinator_state.round_params.sum = sum_ratio;
        self
    }

    pub fn with_update_ratio(mut self, update_ratio: f64) -> Self {
        self.coordinator_state.round_params.update = update_ratio;
        self
    }

    pub fn with_seed(mut self, seed: RoundSeed) -> Self {
        self.coordinator_state.round_params.seed = seed;
        self
    }

    pub fn with_min_sum(mut self, min_sum: u64) -> Self {
        self.coordinator_state.min_sum_count = min_sum;
        self
    }

    pub fn with_mask_config(mut self, mask_config: MaskConfig) -> Self {
        self.coordinator_state.mask_config = mask_config;
        self
    }

    pub fn with_min_update(mut self, min_update: u64) -> Self {
        self.coordinator_state.min_update_count = min_update;
        self
    }

    pub fn with_model_length(mut self, model_length: usize) -> Self {
        self.coordinator_state.model_length = model_length;
        self
    }

    pub fn with_min_sum_time(mut self, in_secs: u64) -> Self {
        self.coordinator_state.min_sum_time = in_secs;
        self
    }

    pub fn with_max_sum_time(mut self, in_secs: u64) -> Self {
        self.coordinator_state.max_sum_time = in_secs;
        self
    }

    pub fn with_min_update_time(mut self, in_secs: u64) -> Self {
        self.coordinator_state.min_update_time = in_secs;
        self
    }

    pub fn with_max_update_time(mut self, in_secs: u64) -> Self {
        self.coordinator_state.max_update_time = in_secs;
        self
    }

    pub fn with_phase<S>(self, phase_state: S) -> StateMachineBuilder<S, C, M> {
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
