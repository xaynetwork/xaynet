use crate::{service::data::RoundParametersData, SumParticipantPublicKey};
use derive_more::From;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    stream::Stream,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};

/// An event handled by the coordinator
#[derive(From)]
pub enum Event {
    /// A message from a participant.
    Message(Message),

    /// A request for retrieving the coordinator parameters for this
    /// round: public key, seed, and fractions of sum and update
    /// participants.
    RoundParameters(RoundParametersRequest),

    /// A request for retrieving the sum dictionary for the current
    /// round
    SumDict(SumDictRequest),

    /// A request to retrieve the masking seeds dictionary for the
    /// given participant.
    SeedDict(SeedDictRequest),
}

/// Event for an incoming message from a participant
pub struct Message {
    /// Encrypted message
    pub buffer: Vec<u8>,
    // FIXME: there should be a channel to send a response back
}

pub type SerializedGlobalModel = Arc<Vec<u8>>;

/// Event for a request to retrieve the round parameters
pub struct RoundParametersRequest {
    /// Channel for sending the round parameters back
    pub response_tx: oneshot::Sender<Option<Arc<RoundParametersData>>>,
}

pub type SerializedSumDict = Arc<Vec<u8>>;

/// Event for a request to retrieve the sum dictionary
pub struct SumDictRequest {
    /// Channel for sending the sum dictionary back
    pub response_tx: oneshot::Sender<Option<SerializedSumDict>>,
}

pub type SerializedSeedDict = Arc<Vec<u8>>;

/// Event for a request to retrieve the seed dictionary
pub struct SeedDictRequest {
    /// Public key of the sum participant that
    pub public_key: SumParticipantPublicKey,

    /// Channel for sending the seeds dictionary back
    pub response_tx: oneshot::Sender<Option<Arc<Vec<u8>>>>,
}

/// A handle to send events to be handled by [`Service`]
#[derive(Clone)]
pub struct Handle(UnboundedSender<Event>);

impl Handle {
    /// Create a new `Handle`, and return an `EventStream` that yields
    /// events the `Handle` produces.
    pub fn new() -> (Self, EventStream) {
        let (tx, rx) = unbounded_channel::<Event>();
        (Self(tx), EventStream(rx))
    }

    /// Send a [`Event::Message`] event with the given `message`
    pub async fn send_message(&self, message: Vec<u8>) {
        self.send_event(Message { buffer: message });
    }

    /// Send a [`Event::RoundParameters`] event to retrieve the
    /// current round parameters. The availability of the round
    /// parameters depends on the current coordinator state.
    pub async fn get_round_parameters(&self) -> Option<Arc<RoundParametersData>> {
        let (tx, rx) = oneshot::channel::<Option<Arc<RoundParametersData>>>();
        self.send_event(RoundParametersRequest { response_tx: tx });
        rx.await.unwrap()
    }

    /// Send a [`Event::SumDict`] event to retrieve the current sum
    /// dictionary, in its serialized form. The availability of the
    /// sum dictionary depends on the current coordinator state.
    pub async fn get_sum_dict(&self) -> Option<SerializedSumDict> {
        let (tx, rx) = oneshot::channel::<Option<SerializedSumDict>>();
        self.send_event(SumDictRequest { response_tx: tx });
        rx.await.unwrap()
    }

    /// Send a [`Event::SeedDict`] event to retrieve the current seed
    /// dictionary for the given sum participant public key. The
    /// availability of the seed dictionary depends on the current
    /// coordinator state.
    pub async fn get_seed_dict(&self, key: SumParticipantPublicKey) -> Option<SerializedSeedDict> {
        let (tx, rx) = oneshot::channel::<Option<SerializedSeedDict>>();
        let event = SeedDictRequest {
            public_key: key,
            response_tx: tx,
        };
        self.send_event(event);
        rx.await.unwrap()
    }

    fn send_event<T: Into<Event>>(&self, event: T) {
        trace!("sending event to the service");
        if self.0.send(event.into()).is_err() {
            // FIXME: this method should return an error instead
            panic!("failed to send request: channel closed");
        }
    }
}

/// A stream that yields events to be handled by the [`Service`]
pub struct EventStream(UnboundedReceiver<Event>);

impl Stream for EventStream {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().0).poll_next(cx)
    }
}
