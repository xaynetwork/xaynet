use crate::{
    crypto::encrypt::EncryptKeyPair,
    mask::config::MaskConfig,
    settings::PetSettings,
    state_machine::{
        coordinator::{CoordinatorState, RoundSeed},
        events::EventSubscriber,
        phases::{self, Handler, PhaseState},
        requests::{Request, RequestReceiver, RequestSender},
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
        let pet_settings = PetSettings {
            sum: 0.4,
            update: 0.5,
            min_sum: 1,
            min_update: 3,
            expected_participants: 10,
        };
        let mask_settings = utils::mask_settings();
        let (coordinator_state, event_subscriber) =
            CoordinatorState::new(pet_settings, mask_settings);
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
    PhaseState<Request, P>: Handler<Request>,
    StateMachine<Request>: From<PhaseState<Request, P>>,
{
    pub fn build(
        self,
    ) -> (
        StateMachine<Request>,
        RequestSender<Request>,
        EventSubscriber,
    ) {
        let Self {
            coordinator_state,
            event_subscriber,
            phase_state,
        } = self;

        let (request_rx, request_tx) = RequestReceiver::<Request>::new();

        let state = PhaseState {
            inner: phase_state,
            coordinator_state,
            request_rx,
        };

        let state_machine = StateMachine::from(state);
        (state_machine, request_tx, event_subscriber)
    }

    fn broadcast_round_params(&mut self) {
        let params = self.coordinator_state.round_params.clone();
        self.coordinator_state.events.broadcast_params(params);
    }

    #[allow(dead_code)]
    pub fn with_keys(mut self, keys: EncryptKeyPair) -> Self {
        self.coordinator_state.round_params.pk = keys.public.clone();
        self.coordinator_state.keys = keys.clone();
        let round_id = self.coordinator_state.round_params.seed.clone();
        self.coordinator_state.events.broadcast_keys(round_id, keys);
        self.broadcast_round_params();
        self
    }

    pub fn with_sum_ratio(mut self, sum_ratio: f64) -> Self {
        self.coordinator_state.round_params.sum = sum_ratio;
        self.broadcast_round_params();
        self
    }

    pub fn with_update_ratio(mut self, update_ratio: f64) -> Self {
        self.coordinator_state.round_params.update = update_ratio;
        self.broadcast_round_params();
        self
    }

    pub fn with_expected_participants(mut self, expected_participants: usize) -> Self {
        self.coordinator_state.expected_participants = expected_participants;
        self
    }

    pub fn with_seed(mut self, seed: RoundSeed) -> Self {
        self.coordinator_state.round_params.seed = seed;
        self.broadcast_round_params();
        self
    }

    pub fn with_min_sum(mut self, min_sum: usize) -> Self {
        self.coordinator_state.min_sum = min_sum;
        self
    }

    pub fn with_mask_config(mut self, mask_config: MaskConfig) -> Self {
        self.coordinator_state.mask_config = mask_config;
        self
    }

    pub fn with_min_update(mut self, min_update: usize) -> Self {
        self.coordinator_state.min_update = min_update;
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
