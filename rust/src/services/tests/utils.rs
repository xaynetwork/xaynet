use crate::{
    crypto::{ByteObject, EncryptKeyPair, PublicEncryptKey, SecretEncryptKey},
    state_machine::{
        coordinator::{RoundParameters, RoundSeed},
        events::{EventPublisher, EventSubscriber, PhaseEvent},
    },
    CoordinatorPublicKey,
};

pub fn new_event_channels() -> (EventPublisher, EventSubscriber) {
    let keys = EncryptKeyPair {
        public: PublicEncryptKey::zeroed(),
        secret: SecretEncryptKey::zeroed(),
    };
    let params = RoundParameters {
        pk: CoordinatorPublicKey::zeroed(),
        sum: 0.0,
        update: 0.0,
        seed: RoundSeed::zeroed(),
    };
    let phase = PhaseEvent::Idle;
    EventPublisher::init(keys, params, phase)
}
