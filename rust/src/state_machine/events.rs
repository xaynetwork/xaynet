use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::Stream;
use tokio::sync::watch;

use crate::{
    crypto::EncryptKeyPair,
    mask::Model,
    state_machine::coordinator::{RoundId, RoundParameters},
    SeedDict,
    SumDict,
};

/// An event emitted by the coordinator.
#[derive(Debug, Clone)]
pub struct Event<E> {
    /// Metadata that associate this event to the round in which it is
    /// emitted.
    pub round: RoundId,
    /// The event itself
    pub event: E,
}

/// Event that is emitted when the state machine transition to a new
/// phase.
#[derive(Debug, Clone, Copy)]
pub enum PhaseEvent {
    Idle,
    Sum,
    Update,
    Sum2,
    Unmask,
    Error,
    Shutdown,
}

// FIXME: should we simply use `Option`s here?
#[derive(Debug, Clone)]
pub enum ModelUpdate {
    Invalidate,
    New(Arc<Model>),
}

#[derive(Debug, Clone)]
pub enum ScalarUpdate {
    Invalidate,
    New(f64),
}

#[derive(Debug, Clone)]
pub enum MaskLengthUpdate {
    Invalidate,
    New(usize),
}

#[derive(Debug, Clone)]
pub enum DictionaryUpdate<D> {
    Invalidate,
    New(Arc<D>),
}

/// A convenience type to emit any coordinator event.
pub struct EventPublisher {
    keys_tx: EventBroadcaster<EncryptKeyPair>,
    params_tx: EventBroadcaster<RoundParameters>,
    phase_tx: EventBroadcaster<PhaseEvent>,
    scalar_tx: EventBroadcaster<ScalarUpdate>,
    model_tx: EventBroadcaster<ModelUpdate>,
    mask_length_tx: EventBroadcaster<MaskLengthUpdate>,
    sum_dict_tx: EventBroadcaster<DictionaryUpdate<SumDict>>,
    seed_dict_tx: EventBroadcaster<DictionaryUpdate<SeedDict>>,
}

/// The `EventSubscriber` hands out `EventListener`s for any
/// coordinator event.
pub struct EventSubscriber {
    keys_rx: EventListener<EncryptKeyPair>,
    params_rx: EventListener<RoundParameters>,
    phase_rx: EventListener<PhaseEvent>,
    scalar_rx: EventListener<ScalarUpdate>,
    model_rx: EventListener<ModelUpdate>,
    mask_length_rx: EventListener<MaskLengthUpdate>,
    sum_dict_rx: EventListener<DictionaryUpdate<SumDict>>,
    seed_dict_rx: EventListener<DictionaryUpdate<SeedDict>>,
}

impl EventPublisher {
    /// Initialize a new event publisher with the given initial events.
    pub fn init(
        keys: EncryptKeyPair,
        params: RoundParameters,
        phase: PhaseEvent,
    ) -> (Self, EventSubscriber) {
        let round = params.id;
        let (keys_tx, keys_rx) =
            watch::channel::<Event<EncryptKeyPair>>(Event { round, event: keys });

        let (phase_tx, phase_rx) = watch::channel::<Event<PhaseEvent>>(Event {
            round,
            event: phase,
        });

        let (scalar_tx, scalar_rx) = watch::channel::<Event<ScalarUpdate>>(Event {
            round,
            event: ScalarUpdate::Invalidate,
        });

        let (model_tx, model_rx) = watch::channel::<Event<ModelUpdate>>(Event {
            round,
            event: ModelUpdate::Invalidate,
        });

        let (mask_length_tx, mask_length_rx) = watch::channel::<Event<MaskLengthUpdate>>(Event {
            round,
            event: MaskLengthUpdate::Invalidate,
        });

        let (params_tx, params_rx) = watch::channel::<Event<RoundParameters>>(Event {
            round,
            event: params,
        });

        let (sum_dict_tx, sum_dict_rx) =
            watch::channel::<Event<DictionaryUpdate<SumDict>>>(Event {
                round,
                event: DictionaryUpdate::Invalidate,
            });

        let (seed_dict_tx, seed_dict_rx) =
            watch::channel::<Event<DictionaryUpdate<SeedDict>>>(Event {
                round,
                event: DictionaryUpdate::Invalidate,
            });

        let publisher = EventPublisher {
            keys_tx: keys_tx.into(),
            params_tx: params_tx.into(),
            phase_tx: phase_tx.into(),
            scalar_tx: scalar_tx.into(),
            model_tx: model_tx.into(),
            mask_length_tx: mask_length_tx.into(),
            sum_dict_tx: sum_dict_tx.into(),
            seed_dict_tx: seed_dict_tx.into(),
        };

        let subscriber = EventSubscriber {
            keys_rx: keys_rx.into(),
            params_rx: params_rx.into(),
            phase_rx: phase_rx.into(),
            scalar_rx: scalar_rx.into(),
            model_rx: model_rx.into(),
            mask_length_rx: mask_length_rx.into(),
            sum_dict_rx: sum_dict_rx.into(),
            seed_dict_rx: seed_dict_rx.into(),
        };

        (publisher, subscriber)
    }

    /// Emit a keys event
    pub fn broadcast_keys(&mut self, round: RoundId, keys: EncryptKeyPair) {
        let _ = self.keys_tx.broadcast(Event { round, event: keys });
    }

    /// Emit a round parameters event
    pub fn broadcast_params(&mut self, params: RoundParameters) {
        let _ = self.params_tx.broadcast(Event {
            round: params.id,
            event: params,
        });
    }

    /// Emit a phase event
    pub fn broadcast_phase(&mut self, round: RoundId, phase: PhaseEvent) {
        let _ = self.phase_tx.broadcast(Event {
            round,
            event: phase,
        });
    }

    /// Emit a scalar event
    pub fn broadcast_scalar(&mut self, round: RoundId, update: ScalarUpdate) {
        let _ = self.scalar_tx.broadcast(Event {
            round,
            event: update,
        });
    }

    /// Emit a model event
    pub fn broadcast_model(&mut self, round: RoundId, update: ModelUpdate) {
        let _ = self.model_tx.broadcast(Event {
            round,
            event: update,
        });
    }

    /// Emit a mask_length event
    pub fn broadcast_mask_length(&mut self, round: RoundId, update: MaskLengthUpdate) {
        let _ = self.mask_length_tx.broadcast(Event {
            round,
            event: update,
        });
    }

    /// Emit a sum dictionary update
    pub fn broadcast_sum_dict(&mut self, round: RoundId, update: DictionaryUpdate<SumDict>) {
        let _ = self.sum_dict_tx.broadcast(Event {
            round,
            event: update,
        });
    }

    /// Emit a seed dictionary update
    pub fn broadcast_seed_dict(&mut self, round: RoundId, update: DictionaryUpdate<SeedDict>) {
        let _ = self.seed_dict_tx.broadcast(Event {
            round,
            event: update,
        });
    }
}

impl EventSubscriber {
    /// Get a listener for keys events. Callers must be careful not to
    /// leak the secret key they receive, since that would compromise
    /// the security of the coordinator.
    pub fn keys_listener(&self) -> EventListener<EncryptKeyPair> {
        self.keys_rx.clone()
    }
    /// Get a listener for round parameters events
    pub fn params_listener(&self) -> EventListener<RoundParameters> {
        self.params_rx.clone()
    }

    /// Get a listener for new phase events
    pub fn phase_listener(&self) -> EventListener<PhaseEvent> {
        self.phase_rx.clone()
    }

    /// Get a listener for new scalar events
    pub fn scalar_listener(&self) -> EventListener<ScalarUpdate> {
        self.scalar_rx.clone()
    }

    /// Get a listener for new model events
    pub fn model_listener(&self) -> EventListener<ModelUpdate> {
        self.model_rx.clone()
    }

    /// Get a listener for new mask_length events
    pub fn mask_length_listener(&self) -> EventListener<MaskLengthUpdate> {
        self.mask_length_rx.clone()
    }

    /// Get a listener for sum dictionary updates
    pub fn sum_dict_listener(&self) -> EventListener<DictionaryUpdate<SumDict>> {
        self.sum_dict_rx.clone()
    }

    /// Get a listener for seed dictionary updates
    pub fn seed_dict_listener(&self) -> EventListener<DictionaryUpdate<SeedDict>> {
        self.seed_dict_rx.clone()
    }
}

/// A listener for coordinator events. It can be used to either
/// retrieve the latest `Event<E>` emitted by the coordinator (with
/// `EventListener::get_latest`) or to wait for events (since
/// `EventListener<E>` implements `Stream<Item=Event<E>`.
#[derive(Clone)]
pub struct EventListener<E>(watch::Receiver<Event<E>>);

impl<E> From<watch::Receiver<Event<E>>> for EventListener<E> {
    fn from(receiver: watch::Receiver<Event<E>>) -> Self {
        EventListener(receiver)
    }
}

impl<E> EventListener<E>
where
    E: Clone,
{
    pub fn get_latest(&self) -> Event<E> {
        self.0.borrow().clone()
    }
}

impl<E: Clone> Stream for EventListener<E> {
    type Item = Event<E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.0).poll_next(cx)
    }
}

/// A channel to send `Event<E>` to all the `EventListener<E>`.
#[derive(Debug)]
pub struct EventBroadcaster<E>(watch::Sender<Event<E>>);

impl<E> EventBroadcaster<E> {
    /// Send `event` to all the `EventListener<E>`
    fn broadcast(&self, event: Event<E>) {
        // We don't care whether there's a listener or not
        let _ = self.0.broadcast(event);
    }
}

impl<E> From<watch::Sender<Event<E>>> for EventBroadcaster<E> {
    fn from(sender: watch::Sender<Event<E>>) -> Self {
        Self(sender)
    }
}
