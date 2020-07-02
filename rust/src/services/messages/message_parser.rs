use std::{pin::Pin, sync::Arc, task::Poll};

use derive_more::From;
use futures::{
    future::{self, Either, Future},
    task::Context,
};
use rayon::ThreadPool;
use thiserror::Error;
use tokio::sync::oneshot;
use tower::Service;

use crate::{
    crypto::{encrypt::EncryptKeyPair, ByteObject},
    message::{
        DecodeError,
        FromBytes,
        HeaderOwned,
        MessageOwned,
        PayloadOwned,
        Sum2Owned,
        SumOwned,
        Tag,
        ToBytes,
        UpdateOwned,
    },
    state_machine::events::{EventListener, EventSubscriber, PhaseEvent},
    utils::trace::{Traceable, Traced},
    Signature,
};

/// A service for decrypting and parsing PET messages.
///
/// Since this is a CPU-intensive task for large messages, this
/// service offloads the processing to a `rayon` thread-pool to avoid
/// overloading the tokio thread-pool with blocking tasks.
pub struct MessageParserService {
    /// A listener to retrieve the latest coordinator keys. These are
    /// necessary for decrypting messages and verifying their
    /// signature.
    keys_events: EventListener<EncryptKeyPair>,

    /// A listener to retrieve the current coordinator phase. Messages
    /// that cannot be handled in the current phase will be
    /// rejected. The idea is to perform this filtering as early as
    /// possible.
    phase_events: EventListener<PhaseEvent>,

    /// Thread-pool the CPU-intensive tasks are offloaded to
    thread_pool: Arc<ThreadPool>,
}

impl MessageParserService {
    pub fn new(subscriber: &EventSubscriber, thread_pool: Arc<ThreadPool>) -> Self {
        Self {
            keys_events: subscriber.keys_listener(),
            phase_events: subscriber.phase_listener(),
            thread_pool,
        }
    }
}

/// Request type for the [`MessageParserService`].
///
/// It contains the encrypted message.
#[derive(From, Debug)]
pub struct MessageParserRequest(Vec<u8>);

/// Response type for the [`MessageParserService`].
///
/// It contains the parsed message.
pub type MessageParserResponse = Result<MessageOwned, MessageParserError>;

/// Error type for the [`MessageParserService`]
#[derive(Debug, Error)]
pub enum MessageParserError {
    #[error("Failed to decrypt the message with the coordinator secret key")]
    Decrypt,

    #[error("Parsing failed: {0:?}")]
    Parsing(DecodeError),

    #[error("Invalid message signature")]
    InvalidMessageSignature,

    #[error("The message was rejected because the coordinator did not expect it")]
    UnexpectedMessage,

    // TODO: we should have a retry layer that automatically retries
    // requests that fail with this error.
    #[error("The request could not be processed due to a temporary internal error")]
    TemporaryInternalError,

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl Service<Traced<MessageParserRequest>> for MessageParserService {
    type Response = MessageParserResponse;
    type Error = std::convert::Infallible;

    #[allow(clippy::type_complexity)]
    type Future = Either<
        future::Ready<Result<Self::Response, Self::Error>>,
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'static + Send + Sync>>,
    >;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Traced<MessageParserRequest>) -> Self::Future {
        debug!("retrieving the current keys and current phase");
        let keys_ev = self.keys_events.get_latest();
        let phase_ev = self.phase_events.get_latest();

        // This can happen if the coordinator is switching starting a
        // new phase. The error should be temporary and we should be
        // able to retry the request.
        if keys_ev.round != phase_ev.round {
            return Either::Left(future::ready(Ok(Err(
                MessageParserError::TemporaryInternalError,
            ))));
        }

        let handler = Handler {
            keys: keys_ev.event,
            phase: phase_ev.event,
        };

        let (tx, rx) = oneshot::channel::<Self::Response>();

        trace!("spawning pre-processor handler on thread-pool");
        self.thread_pool.spawn(move || {
            let span = req.span().clone();
            let _enter = span.enter();
            let resp = handler.call(req.into_inner().0);
            let _ = tx.send(resp);
        });
        Either::Right(Box::pin(async move {
            Ok(rx.await.unwrap_or_else(|_| {
                Err(MessageParserError::InternalError(
                    "failed to receive response from pre-processor".to_string(),
                ))
            }))
        }))
    }
}

/// Handler created by the [`MessageParserService`] for each request.
struct Handler {
    /// Coordinator keys for the current round
    keys: EncryptKeyPair,
    /// Current phase of the coordinator
    phase: PhaseEvent,
}

impl Handler {
    /// Process the request. `data` is the encrypted PET message to
    /// process.
    fn call(self, data: Vec<u8>) -> Result<MessageOwned, MessageParserError> {
        info!("decrypting message");
        let raw = self.decrypt(data)?;

        info!("parsing message header");
        let header = self.parse_header(raw.as_slice())?;

        info!("filtering message based on the current phase");
        self.phase_filter(header.tag)?;

        info!("verifying the message signature");
        self.verify_signature(raw.as_slice(), &header)?;

        info!("parsing the message payload");
        let payload = self.parse_payload(raw.as_slice(), &header)?;

        info!("done pre-processing the message");
        Ok(MessageOwned { header, payload })
    }

    /// Decrypt the given payload with the coordinator secret key
    fn decrypt(&self, encrypted_message: Vec<u8>) -> Result<Vec<u8>, MessageParserError> {
        Ok(self
            .keys
            .secret
            .decrypt(&encrypted_message.as_ref(), &self.keys.public)
            .map_err(|_| MessageParserError::Decrypt)?)
    }

    /// Attempt to parse the message header from the raw message
    fn parse_header(&self, raw_message: &[u8]) -> Result<HeaderOwned, MessageParserError> {
        Ok(HeaderOwned::from_bytes(&&raw_message[Signature::LENGTH..])
            .map_err(MessageParserError::Parsing)?)
    }

    /// Reject messages that cannot be handled by the coordinator in
    /// the current phase
    fn phase_filter(&self, tag: Tag) -> Result<(), MessageParserError> {
        match (tag, self.phase) {
            (Tag::Sum, PhaseEvent::Sum)
            | (Tag::Update, PhaseEvent::Update)
            | (Tag::Sum2, PhaseEvent::Sum2) => Ok(()),
            (tag, phase) => {
                warn!(
                    "rejecting request: message type is {:?} but phase is {:?}",
                    tag, phase
                );
                Err(MessageParserError::UnexpectedMessage)
            }
        }
    }

    /// Verify the integrity of the given message by checking the
    /// signature embedded in the header.
    fn verify_signature(
        &self,
        raw_message: &[u8],
        header: &HeaderOwned,
    ) -> Result<(), MessageParserError> {
        // UNWRAP_SAFE: We already parsed the header, so we now the
        // message is at least as big as: signature length + header
        // length
        let signature = Signature::from_slice(&raw_message[..Signature::LENGTH]).unwrap();
        let bytes = &raw_message[Signature::LENGTH..];
        if header.participant_pk.verify_detached(&signature, bytes) {
            Ok(())
        } else {
            Err(MessageParserError::InvalidMessageSignature)
        }
    }

    /// Parse the payload of the given message
    fn parse_payload(
        &self,
        raw_message: &[u8],
        header: &HeaderOwned,
    ) -> Result<PayloadOwned, MessageParserError> {
        let bytes = &raw_message[header.buffer_length() + Signature::LENGTH..];
        match header.tag {
            Tag::Sum => {
                let parsed = SumOwned::from_bytes(&bytes)
                    .map_err(|e| MessageParserError::Parsing(e.context("invalid sum payload")))?;
                Ok(PayloadOwned::Sum(parsed))
            }
            Tag::Update => {
                let parsed = UpdateOwned::from_bytes(&bytes).map_err(|e| {
                    MessageParserError::Parsing(e.context("invalid update payload"))
                })?;
                Ok(PayloadOwned::Update(parsed))
            }
            Tag::Sum2 => {
                let parsed = Sum2Owned::from_bytes(&bytes)
                    .map_err(|e| MessageParserError::Parsing(e.context("invalid sum2 payload")))?;
                Ok(PayloadOwned::Sum2(parsed))
            }
        }
    }
}
