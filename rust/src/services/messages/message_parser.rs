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
    crypto::{ByteObject, KeyPair},
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

pub struct MessageParserService {
    keys_events: EventListener<KeyPair>,
    phase_events: EventListener<PhaseEvent>,

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

#[derive(From, Debug)]
pub struct MessageParserRequest(Vec<u8>);

pub type MessageParserResponse = Result<MessageOwned, MessageParserError>;

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

struct Handler {
    /// Coordinator keys for the current round
    keys: KeyPair,
    /// Current phase of the coordinator
    phase: PhaseEvent,
}

impl Handler {
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

    fn decrypt(&self, encrypted_message: Vec<u8>) -> Result<Vec<u8>, MessageParserError> {
        Ok(self
            .keys
            .secret
            .decrypt(&encrypted_message.as_ref(), &self.keys.public)
            .map_err(|_| MessageParserError::Decrypt)?)
    }

    fn parse_header(&self, raw_message: &[u8]) -> Result<HeaderOwned, MessageParserError> {
        Ok(HeaderOwned::from_bytes(&&raw_message[Signature::LENGTH..])
            .map_err(MessageParserError::Parsing)?)
    }

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
