use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::Stream;
use tokio::sync::watch;

use crate::{
    coordinator::{Phase, RoundId, RoundParameters},
    crypto::KeyPair,
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

#[derive(Debug, Clone)]
pub enum DictionaryUpdate<D> {
    Invalidate,
    New(Arc<D>),
}

/// A convenience type to emit any coordinator event.
pub struct EventPublisher {
    keys_tx: EventBroadcaster<KeyPair>,
    params_tx: EventBroadcaster<RoundParameters>,
    phase_tx: EventBroadcaster<Phase>,
    sum_dict_tx: EventBroadcaster<DictionaryUpdate<SumDict>>,
    seed_dict_tx: EventBroadcaster<DictionaryUpdate<SeedDict>>,
}

/// The `EventSubscriber` hands out `EventListener`s for any
/// coordinator event.
pub struct EventSubscriber {
    keys_rx: EventListener<KeyPair>,
    params_rx: EventListener<RoundParameters>,
    phase_rx: EventListener<Phase>,
    sum_dict_rx: EventListener<DictionaryUpdate<SumDict>>,
    seed_dict_rx: EventListener<DictionaryUpdate<SeedDict>>,
}

impl EventPublisher {
    /// Initialize a new event publisher with the given initial events.
    pub fn init(keys: KeyPair, params: RoundParameters, phase: Phase) -> (Self, EventSubscriber) {
        let round = params.id;
        let (keys_tx, keys_rx) = watch::channel::<Event<KeyPair>>(Event { round, event: keys });

        let (phase_tx, phase_rx) = watch::channel::<Event<Phase>>(Event {
            round,
            event: phase,
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
            sum_dict_tx: sum_dict_tx.into(),
            seed_dict_tx: seed_dict_tx.into(),
        };

        let subscriber = EventSubscriber {
            keys_rx: keys_rx.into(),
            params_rx: params_rx.into(),
            phase_rx: phase_rx.into(),
            sum_dict_rx: sum_dict_rx.into(),
            seed_dict_rx: seed_dict_rx.into(),
        };

        (publisher, subscriber)
    }

    /// Emit a keys event
    pub fn broadcast_keys(&mut self, round: RoundId, keys: KeyPair) {
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
    pub fn broadcast_phase(&mut self, round: RoundId, phase: Phase) {
        let _ = self.phase_tx.broadcast(Event {
            round,
            event: phase,
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
    /// Get a listener for keys events
    pub fn keys_listener(&self) -> EventListener<KeyPair> {
        self.keys_rx.clone()
    }
    /// Get a listener for round parameters events
    pub fn params_listener(&self) -> EventListener<RoundParameters> {
        self.params_rx.clone()
    }
    /// Get a listener for new phase events
    pub fn phase_listener(&self) -> EventListener<Phase> {
        self.phase_rx.clone()
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
/// `EventListe
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
