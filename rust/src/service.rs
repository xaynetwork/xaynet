use crate::coordinator::{Coordinator, RoundParameters};
use sodiumoxide::crypto::box_;
use std::{
    collections::HashMap,
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

/// The `Service` is the task that drives the PET protocol. It reacts
/// to the various messages from the participants and drives the
/// protocol.
pub struct Service {
    /// The coordinator holds the protocol state: crypto material, sum
    /// and update dictionaries, configuration, etc.
    coordinator: Coordinator,

    /// Events to handle
    events: EventStream,

    /// Cache for the data the service needs to serve
    cache: ServiceCache,
}

/// Cache for the data served by the service. There are some
/// potentially large datastructures that the coordinator needs to be
/// able to serve, so the cache provides two optimizations for some of
/// them:
///
/// - they are wrapped reference counted pointers
/// - they are already serialized
struct ServiceCache {
    /// Current round parameters
    round_parameters: Arc<RoundParameters>,

    /// Serialized sum dictionary
    sum_dict: Option<Arc<Vec<u8>>>,

    /// Serialized seeds dictionaries
    seed_dict: Option<HashMap<box_::PublicKey, Vec<u8>>>,
}

impl Service {
    /// Dispatch the given event to the appropriate handler
    fn dispatch_event(&mut self, event: Event) {
        match event {
            Event::Message(Message { buffer }) => self.handle_message(buffer),
            _ => unimplemented!(),
        }
    }

    // TODO/FIXME:
    //
    // when we fail to handle a message, we should send back and error
    // so that the participant is informed. One problem currently is
    // that the `validate` methods don't return specific errors.
    /// Handle a message
    fn handle_message(&mut self, buffer: Vec<u8>) {
        let _ = self.coordinator.validate_message(&buffer[..]);
    }
}

/// An event handled by the coordinator
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

impl From<RoundParametersRequest> for Event {
    fn from(req: RoundParametersRequest) -> Self {
        Self::RoundParameters(req)
    }
}
impl From<SumDictRequest> for Event {
    fn from(req: SumDictRequest) -> Self {
        Self::SumDict(req)
    }
}
impl From<SeedDictRequest> for Event {
    fn from(req: SeedDictRequest) -> Self {
        Self::SeedDict(req)
    }
}
impl From<Message> for Event {
    fn from(msg: Message) -> Self {
        Self::Message(msg)
    }
}

/// Event for an incoming message from a participant
pub struct Message {
    /// Encrypted message
    buffer: Vec<u8>,
    // FIXME: there should be a channel to send a response back
}

/// Event for a request to retrieve the round parameters
pub struct RoundParametersRequest {
    /// Channel for sending the round parameters back
    response_tx: oneshot::Sender<Option<Arc<RoundParameters>>>,
}

/// Event for a request to retrieve the sum dictionary
pub struct SumDictRequest {
    /// Channel for sending the sum dictionary back
    response_tx: oneshot::Sender<Option<Arc<Vec<u8>>>>,
}

/// Event for a request to retrieve the seed dictionary
pub struct SeedDictRequest {
    /// Public key of the sum participant that
    public_key: box_::PublicKey,

    /// Channel for sending the seeds dictionary back
    response_tx: oneshot::Sender<Option<Arc<Vec<u8>>>>,
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
    /// current round parameters.
    pub async fn get_round_parameters(&self) -> Option<Arc<RoundParameters>> {
        let (tx, rx) = oneshot::channel::<Option<Arc<RoundParameters>>>();
        let event = RoundParametersRequest { response_tx: tx };
        self.send_event(event);
        rx.await.unwrap()
    }

    /// Send a [`Event::SumDict`] event to retrieve the current sum
    /// dictionary, in its serialized form.
    pub async fn get_sum_dict(&self) -> Option<Arc<Vec<u8>>> {
        unimplemented!()
    }

    /// Send a [`Event::SeedDict`] event to retrieve the current seed
    /// dictionary, in its serialized form.
    pub async fn get_seed_dict(&self) -> Option<Arc<Vec<u8>>> {
        unimplemented!()
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
