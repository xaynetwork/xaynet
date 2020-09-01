use std::{convert::TryInto, pin::Pin, sync::Arc, task::Poll};

use anyhow::Context as _;
use derive_more::From;
use futures::{
    future::{self, Either, Future},
    task::Context,
};
use rayon::ThreadPool;
use thiserror::Error;
use tokio::sync::oneshot;
use tower::Service;
use tracing::Span;
use xaynet_core::{
    crypto::EncryptKeyPair,
    message::{DecodeError, Message, MessageBuffer, Tag},
};

use crate::{
    state_machine::{
        events::{EventListener, EventSubscriber},
        phases::PhaseName,
    },
    utils::{Request, Traceable},
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
    phase_events: EventListener<PhaseName>,

    /// Thread-pool the CPU-intensive tasks are offloaded to.
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

/// A buffer that represents an encrypted message.
#[derive(From, Debug)]
pub struct RawMessage<T: AsRef<[u8]>>(T);

impl<T> Traceable for RawMessage<T>
where
    T: AsRef<[u8]>,
{
    fn make_span(&self) -> Span {
        error_span!("raw_message", payload_len = self.0.as_ref().len())
    }
}

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

/// Response type for the [`MessageParserService`]
pub type MessageParserResponse = Result<Message, MessageParserError>;

/// Request type for the [`MessageParserService`]
pub type MessageParserRequest<T> = Request<RawMessage<T>>;

impl<T> Service<MessageParserRequest<T>> for MessageParserService
where
    T: AsRef<[u8]> + Send + 'static,
{
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

    fn call(&mut self, req: MessageParserRequest<T>) -> Self::Future {
        debug!("retrieving the current keys and current phase");
        let keys_ev = self.keys_events.get_latest();
        let phase_ev = self.phase_events.get_latest();

        // This can happen if the coordinator is switching starting a
        // new phase. The error should be temporary and we should be
        // able to retry the request.
        if keys_ev.round_id != phase_ev.round_id {
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
            let span = req.span();
            let _span_guard = span.enter();
            let resp = handler.call(req.into_inner());
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
    phase: PhaseName,
}

impl Handler {
    /// Process the request. `data` is the encrypted PET message to
    /// process.
    fn call<T: AsRef<[u8]>>(self, data: RawMessage<T>) -> MessageParserResponse {
        info!("decrypting message");
        let raw = self.decrypt(&data.0.as_ref())?;

        let buf = MessageBuffer::new(&raw).map_err(MessageParserError::Parsing)?;

        info!("filtering message based on the current phase");
        let tag = buf
            .tag()
            .try_into()
            .context("failed to parse message tag field")
            .map_err(MessageParserError::Parsing)?;
        self.phase_filter(tag)?;

        info!("verifying the message signature");
        buf.check_signature().map_err(|e| {
            warn!("invalid message signature: {:?}", e);
            MessageParserError::InvalidMessageSignature
        })?;

        info!("parsing the message");
        let message = Message::from_bytes(&raw).map_err(MessageParserError::Parsing)?;

        info!("done pre-processing the message");
        Ok(message)
    }

    /// Decrypt the given payload with the coordinator secret key
    fn decrypt(&self, encrypted_message: &[u8]) -> Result<Vec<u8>, MessageParserError> {
        Ok(self
            .keys
            .secret
            .decrypt(&encrypted_message, &self.keys.public)
            .map_err(|_| MessageParserError::Decrypt)?)
    }

    /// Reject messages that cannot be handled by the coordinator in
    /// the current phase
    fn phase_filter(&self, tag: Tag) -> Result<(), MessageParserError> {
        match (tag, self.phase) {
            (Tag::Sum, PhaseName::Sum)
            | (Tag::Update, PhaseName::Update)
            | (Tag::Sum2, PhaseName::Sum2) => Ok(()),
            (tag, phase) => {
                warn!(
                    "rejecting request: message type is {:?} but phase is {:?}",
                    tag, phase
                );
                Err(MessageParserError::UnexpectedMessage)
            }
        }
    }
}
