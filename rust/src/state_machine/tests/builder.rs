use crate::{
    crypto::encrypt::EncryptKeyPair,
    mask::config::MaskConfig,
    state_machine::{
        coordinator::{CoordinatorState, RoundSeed},
        events::EventSubscriber,
        phases::{self, Handler, Phase, PhaseState},
        requests::{RequestReceiver, RequestSender},
        tests::utils,
        StateMachine,
    },
};

#[derive(Debug)]
pub struct StateMachineBuilder<P> {
    coordinator_state: CoordinatorState,
    event_subscriber: EventSubscriber,
    phase_state: P,
}

impl StateMachineBuilder<phases::Idle> {
    pub fn new() -> Self {
        let (coordinator_state, event_subscriber) = CoordinatorState::new(
            utils::pet_settings(),
            utils::mask_settings(),
            utils::model_settings(),
        );
        let phase_state = phases::Idle;
        StateMachineBuilder {
            coordinator_state,
            event_subscriber,
            phase_state,
        }
    }
}

impl<P> StateMachineBuilder<P>
where
    PhaseState<P>: Handler + Phase,
    StateMachine: From<PhaseState<P>>,
{
    pub fn build(self) -> (StateMachine, RequestSender, EventSubscriber) {
        let Self {
            mut coordinator_state,
            event_subscriber,
            phase_state,
        } = self;

        let (request_rx, request_tx) = RequestReceiver::new();

        // Make sure the events that the listeners have are up to date
        let events = &mut coordinator_state.events;
        events.broadcast_keys(coordinator_state.keys.clone());
        events.broadcast_params(coordinator_state.round_params.clone());
        events.broadcast_phase(<PhaseState<P> as Phase>::NAME);
        // Also re-emit the other events in case the round ID changed
        let scalar = event_subscriber.scalar_listener().get_latest().event;
        events.broadcast_scalar(scalar);
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
            coordinator_state,
            request_rx,
        };

        let state_machine = StateMachine::from(state);

        (state_machine, request_tx, event_subscriber)
    }

    #[allow(dead_code)]
    pub fn with_keys(mut self, keys: EncryptKeyPair) -> Self {
        self.coordinator_state.round_params.pk = keys.public.clone();
        self.coordinator_state.keys = keys.clone();
        self
    }

    pub fn with_round_id(mut self, id: u64) -> Self {
        self.coordinator_state.set_round_id(id);
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

    pub fn with_expected_participants(mut self, expected_participants: usize) -> Self {
        self.coordinator_state.expected_participants = expected_participants;
        self
    }

    pub fn with_seed(mut self, seed: RoundSeed) -> Self {
        self.coordinator_state.round_params.seed = seed;
        self
    }

    pub fn with_min_sum(mut self, min_sum: usize) -> Self {
        self.coordinator_state.min_sum_count = min_sum;
        self
    }

    pub fn with_mask_config(mut self, mask_config: MaskConfig) -> Self {
        self.coordinator_state.mask_config = mask_config;
        self
    }

    pub fn with_min_update(mut self, min_update: usize) -> Self {
        self.coordinator_state.min_update_count = min_update;
        self
    }

    pub fn with_model_size(mut self, model_size: usize) -> Self {
        self.coordinator_state.model_size = model_size;
        self
    }

    pub fn with_phase<S>(self, phase_state: S) -> StateMachineBuilder<S> {
        let Self {
            coordinator_state,
            event_subscriber,
            ..
        } = self;
        StateMachineBuilder {
            coordinator_state,
            event_subscriber,
            phase_state,
        }
    }
}
