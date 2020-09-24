use xaynet_core::{common::RoundSeed, crypto::EncryptKeyPair, mask::MaskConfig};

use crate::{
    state_machine::{
        events::EventSubscriber,
        phases::{self, Handler, Phase, PhaseState, Shared},
        requests::RequestSender,
        tests::utils,
        StateMachine,
    },
    storage::redis,
};

#[derive(Debug)]
pub struct StateMachineBuilder<P> {
    shared: Shared,
    request_tx: RequestSender,
    event_subscriber: EventSubscriber,
    phase_state: P,
    redis: redis::Client,
}

impl StateMachineBuilder<phases::Idle> {
    pub async fn new() -> Self {
        let (shared, event_subscriber, request_tx, redis) = utils::init_shared().await;

        let phase_state = phases::Idle;
        StateMachineBuilder {
            shared,
            request_tx,
            event_subscriber,
            phase_state,
            redis,
        }
    }
}

impl<P> StateMachineBuilder<P>
where
    PhaseState<P>: Handler + Phase,
    StateMachine: From<PhaseState<P>>,
{
    pub fn build(self) -> (StateMachine, RequestSender, EventSubscriber, redis::Client) {
        let Self {
            mut shared,
            request_tx,
            event_subscriber,
            phase_state,
            redis,
        } = self;

        // Make sure the events that the listeners have are up to date
        let events = &mut shared.io.events;
        events.broadcast_keys(shared.state.keys.clone());
        events.broadcast_params(shared.state.round_params.clone());
        events.broadcast_phase(<PhaseState<P> as Phase>::NAME);
        // Also re-emit the other events in case the round ID changed
        let model = event_subscriber.model_listener().get_latest().event;
        events.broadcast_model(model);
        let mask_length = event_subscriber.mask_length_listener().get_latest().event;
        events.broadcast_mask_length(mask_length);
        let sum_dict = event_subscriber.sum_dict_listener().get_latest().event;
        events.broadcast_sum_dict(sum_dict);
        let seed_dict = event_subscriber.seed_dict_listener().get_latest().event;
        events.broadcast_seed_dict(seed_dict);

        let state = PhaseState {
            inner: phase_state,
            shared,
        };

        let state_machine = StateMachine::from(state);

        (state_machine, request_tx, event_subscriber, redis)
    }

    #[allow(dead_code)]
    pub fn with_keys(mut self, keys: EncryptKeyPair) -> Self {
        self.shared.state.round_params.pk = keys.public.clone();
        self.shared.state.keys = keys.clone();
        self
    }

    pub fn with_round_id(mut self, id: u64) -> Self {
        self.shared.set_round_id(id);
        self
    }

    pub fn with_sum_ratio(mut self, sum_ratio: f64) -> Self {
        self.shared.state.round_params.sum = sum_ratio;
        self
    }

    pub fn with_update_ratio(mut self, update_ratio: f64) -> Self {
        self.shared.state.round_params.update = update_ratio;
        self
    }

    pub fn with_seed(mut self, seed: RoundSeed) -> Self {
        self.shared.state.round_params.seed = seed;
        self
    }

    pub fn with_min_sum(mut self, min_sum: usize) -> Self {
        self.shared.state.min_sum_count = min_sum;
        self
    }

    pub fn with_mask_config(mut self, mask_config: MaskConfig) -> Self {
        self.shared.state.mask_config = mask_config;
        self
    }

    pub fn with_min_update(mut self, min_update: usize) -> Self {
        self.shared.state.min_update_count = min_update;
        self
    }

    pub fn with_model_size(mut self, model_size: usize) -> Self {
        self.shared.state.model_size = model_size;
        self
    }

    pub fn with_min_sum_time(mut self, in_secs: u64) -> Self {
        self.shared.state.min_sum_time = in_secs;
        self
    }

    pub fn with_max_sum_time(mut self, in_secs: u64) -> Self {
        self.shared.state.max_sum_time = in_secs;
        self
    }

    pub fn with_min_update_time(mut self, in_secs: u64) -> Self {
        self.shared.state.min_update_time = in_secs;
        self
    }

    pub fn with_max_update_time(mut self, in_secs: u64) -> Self {
        self.shared.state.max_update_time = in_secs;
        self
    }

    pub fn with_phase<S>(self, phase_state: S) -> StateMachineBuilder<S> {
        let Self {
            shared,
            request_tx,
            event_subscriber,
            redis,
            ..
        } = self;
        StateMachineBuilder {
            shared,
            request_tx,
            event_subscriber,
            phase_state,
            redis,
        }
    }
}
