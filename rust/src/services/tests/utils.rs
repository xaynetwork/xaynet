use crate::{
    crypto::{ByteObject, EncryptKeyPair},
    state_machine::{
        coordinator::{RoundParameters, RoundSeed},
        events::{EventPublisher, EventSubscriber, PhaseEvent},
    },
};

pub fn new_event_channels() -> (EventPublisher, EventSubscriber) {
    let keys = EncryptKeyPair::generate();
    let params = RoundParameters {
        pk: keys.public.clone(),
        sum: 0.0,
        update: 0.0,
        seed: RoundSeed::generate(),
    };
    let phase = PhaseEvent::Idle;
    EventPublisher::init(keys, params, phase)
}
