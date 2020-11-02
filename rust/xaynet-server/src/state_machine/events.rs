//! This module provides the [`StateMachine`]'s `Events`, `EventSubscriber` and `EventPublisher`
//! types.
//!
//! [`StateMachine`]: crate::state_machine::StateMachine

use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::Stream;
use tokio::sync::watch;
use xaynet_core::{
    common::RoundParameters,
    crypto::EncryptKeyPair,
    mask::Model,
    SeedDict,
    SumDict,
};

use crate::state_machine::phases::PhaseName;

/// An event emitted by the coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event<E> {
    /// Metadata that associates this event to the round in which it is
    /// emitted.
    pub round_id: u64,
    /// The event itself
    pub event: E,
}

// FIXME: should we simply use `Option`s here?
/// Global model update event.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelUpdate {
    Invalidate,
    New(Arc<Model>),
}

/// Mask length update event.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MaskLengthUpdate {
    Invalidate,
    New(usize),
}

/// Dictionary update event.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DictionaryUpdate<D> {
    Invalidate,
    New(Arc<D>),
}

/// A convenience type to emit any coordinator event.
#[derive(Debug)]
pub struct EventPublisher {
    /// Round ID that is attached to all the requests.
    round_id: u64,
    keys_tx: EventBroadcaster<EncryptKeyPair>,
    params_tx: EventBroadcaster<RoundParameters>,
    phase_tx: EventBroadcaster<PhaseName>,
    model_tx: EventBroadcaster<ModelUpdate>,
    mask_length_tx: EventBroadcaster<MaskLengthUpdate>,
    sum_dict_tx: EventBroadcaster<DictionaryUpdate<SumDict>>,
    seed_dict_tx: EventBroadcaster<DictionaryUpdate<SeedDict>>,
}

/// The `EventSubscriber` hands out `EventListener`s for any
/// coordinator event.
#[derive(Debug)]
pub struct EventSubscriber {
    keys_rx: EventListener<EncryptKeyPair>,
    params_rx: EventListener<RoundParameters>,
    phase_rx: EventListener<PhaseName>,
    model_rx: EventListener<ModelUpdate>,
    mask_length_rx: EventListener<MaskLengthUpdate>,
    sum_dict_rx: EventListener<DictionaryUpdate<SumDict>>,
    seed_dict_rx: EventListener<DictionaryUpdate<SeedDict>>,
}

impl EventPublisher {
    /// Initialize a new event publisher with the given initial events.
    pub fn init(
        round_id: u64,
        keys: EncryptKeyPair,
        params: RoundParameters,
        phase: PhaseName,
        model: ModelUpdate,
    ) -> (Self, EventSubscriber) {
        let (keys_tx, keys_rx) = watch::channel::<Event<EncryptKeyPair>>(Event {
            round_id,
            event: keys,
        });

        let (phase_tx, phase_rx) = watch::channel::<Event<PhaseName>>(Event {
            round_id,
            event: phase,
        });

        let (model_tx, model_rx) = watch::channel::<Event<ModelUpdate>>(Event {
            round_id,
            event: model,
        });

        let (mask_length_tx, mask_length_rx) = watch::channel::<Event<MaskLengthUpdate>>(Event {
            round_id,
            event: MaskLengthUpdate::Invalidate,
        });

        let (sum_dict_tx, sum_dict_rx) =
            watch::channel::<Event<DictionaryUpdate<SumDict>>>(Event {
                round_id,
                event: DictionaryUpdate::Invalidate,
            });

        let (seed_dict_tx, seed_dict_rx) =
            watch::channel::<Event<DictionaryUpdate<SeedDict>>>(Event {
                round_id,
                event: DictionaryUpdate::Invalidate,
            });

        let (params_tx, params_rx) = watch::channel::<Event<RoundParameters>>(Event {
            round_id,
            event: params,
        });

        let publisher = EventPublisher {
            round_id,
            keys_tx: keys_tx.into(),
            params_tx: params_tx.into(),
            phase_tx: phase_tx.into(),
            model_tx: model_tx.into(),
            mask_length_tx: mask_length_tx.into(),
            sum_dict_tx: sum_dict_tx.into(),
            seed_dict_tx: seed_dict_tx.into(),
        };

        let subscriber = EventSubscriber {
            keys_rx: keys_rx.into(),
            params_rx: params_rx.into(),
            phase_rx: phase_rx.into(),
            model_rx: model_rx.into(),
            mask_length_rx: mask_length_rx.into(),
            sum_dict_rx: sum_dict_rx.into(),
            seed_dict_rx: seed_dict_rx.into(),
        };

        (publisher, subscriber)
    }

    /// Set the round ID that is attached to the events the publisher broadcasts.
    pub fn set_round_id(&mut self, id: u64) {
        self.round_id = id;
    }

    fn event<T>(&self, event: T) -> Event<T> {
        Event {
            round_id: self.round_id,
            event,
        }
    }

    /// Emit a keys event
    pub fn broadcast_keys(&mut self, keys: EncryptKeyPair) {
        let _ = self.keys_tx.broadcast(self.event(keys));
    }

    /// Emit a round parameters event
    pub fn broadcast_params(&mut self, params: RoundParameters) {
        let _ = self.params_tx.broadcast(self.event(params));
    }

    /// Emit a phase event
    pub fn broadcast_phase(&mut self, phase: PhaseName) {
        let _ = self.phase_tx.broadcast(self.event(phase));
    }

    /// Emit a model event
    pub fn broadcast_model(&mut self, update: ModelUpdate) {
        let _ = self.model_tx.broadcast(self.event(update));
    }

    /// Emit a mask_length event
    pub fn broadcast_mask_length(&mut self, update: MaskLengthUpdate) {
        let _ = self.mask_length_tx.broadcast(self.event(update));
    }

    /// Emit a sum dictionary update
    pub fn broadcast_sum_dict(&mut self, update: DictionaryUpdate<SumDict>) {
        let _ = self.sum_dict_tx.broadcast(self.event(update));
    }

    /// Emit a seed dictionary update
    pub fn broadcast_seed_dict(&mut self, update: DictionaryUpdate<SeedDict>) {
        let _ = self.seed_dict_tx.broadcast(self.event(update));
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
    pub fn phase_listener(&self) -> EventListener<PhaseName> {
        self.phase_rx.clone()
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
#[derive(Debug, Clone)]
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
