use xaynet_core::{SeedDict, SumDict};

use crate::state_machine::{
    coordinator::CoordinatorState,
    events::{DictionaryUpdate, EventPublisher, EventSubscriber, ModelUpdate},
    phases::PhaseName,
};

use super::{utils::EventSnapshot, CoordinatorStateBuilder, WARNING};

pub struct EventBusBuilder {
    event_publisher: EventPublisher,
    event_subscriber: EventSubscriber,
}

impl EventBusBuilder {
    pub fn new(state: &CoordinatorState) -> Self {
        let (event_publisher, event_subscriber) = EventPublisher::init(
            state.round_id,
            state.keys.clone(),
            state.round_params.clone(),
            PhaseName::Idle,
            ModelUpdate::Invalidate,
        );

        Self {
            event_publisher,
            event_subscriber,
        }
    }

    pub fn broadcast_phase(mut self, phase: PhaseName) -> Self {
        self.event_publisher.broadcast_phase(phase);
        self
    }

    pub fn broadcast_model(mut self, update: ModelUpdate) -> Self {
        self.event_publisher.broadcast_model(update);
        self
    }

    pub fn broadcast_sum_dict(mut self, update: DictionaryUpdate<SumDict>) -> Self {
        self.event_publisher.broadcast_sum_dict(update);
        self
    }

    pub fn broadcast_seed_dict(mut self, update: DictionaryUpdate<SeedDict>) -> Self {
        self.event_publisher.broadcast_seed_dict(update);
        self
    }

    pub fn build(self) -> (EventPublisher, EventSubscriber) {
        (self.event_publisher, self.event_subscriber)
    }
}

#[test]
fn test_initial_events() {
    const PANIC_MESSAGE: &str = "the initial events have been changed.";

    let state = CoordinatorStateBuilder::new().build();
    let (_, subscriber) = EventBusBuilder::new(&state).build();
    let events = EventSnapshot::from(&subscriber);

    assert_eq!(
        events.phase.event,
        PhaseName::Idle,
        "{} {}",
        PANIC_MESSAGE,
        WARNING
    );
    assert_eq!(
        events.model.event,
        ModelUpdate::Invalidate,
        "{} {}",
        PANIC_MESSAGE,
        WARNING
    );
    assert_eq!(
        events.sum_dict.event,
        DictionaryUpdate::Invalidate,
        "{} {}",
        PANIC_MESSAGE,
        WARNING
    );
    assert_eq!(
        events.seed_dict.event,
        DictionaryUpdate::Invalidate,
        "{} {}",
        PANIC_MESSAGE,
        WARNING
    );
}
