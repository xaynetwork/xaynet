use std::{sync::Arc, task::Poll};

use anyhow::anyhow;
use futures::{
    future::{MapErr, TryFutureExt},
    task::Context,
};
use rayon::ThreadPool;
use tokio::sync::oneshot;
use tower::Service;

use crate::{
    coordinator::{CoordinatorWatcher, Phase},
    crypto::{ByteObject, KeyPair},
    message::{
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
    services::error::{RequestFailed, ServiceError},
    Signature,
};

pub struct MessageParserService {
    watcher: CoordinatorWatcher,
    thread_pool: Arc<ThreadPool>,
}

impl MessageParserService {
    pub fn new(watcher: CoordinatorWatcher, thread_pool: Arc<ThreadPool>) -> Self {
        Self {
            watcher,
            thread_pool,
        }
    }
}

pub type MessageParserRequest = Vec<u8>;
pub type MessageParserResponse = Result<MessageOwned, RequestFailed>;

impl Service<MessageParserRequest> for MessageParserService {
    type Response = MessageParserResponse;
    type Error = ServiceError;
    type Future =
        MapErr<oneshot::Receiver<Self::Response>, fn(oneshot::error::RecvError) -> Self::Error>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: MessageParserRequest) -> Self::Future {
        let pre_processor = PreProcessor {
            keys: self.watcher.get_keys(),
            phase: self.watcher.get_phase(),
        };
        let (tx, rx) = oneshot::channel::<Self::Response>();
        let span = tracing::Span::current();
        self.thread_pool.spawn(move || {
            let _enter = span.enter();
            let resp = pre_processor.call(req);
            let _ = tx.send(resp);
        });
        rx.map_err(|_| anyhow!("failed to receive response from pre-processor"))
    }
}

struct PreProcessor {
    /// Coordinator keys for the current round
    keys: KeyPair,
    /// Current phase of the coordinator
    phase: Phase,
}

impl PreProcessor {
    fn call(self, data: Vec<u8>) -> Result<MessageOwned, RequestFailed> {
        let raw = self.decrypt(data)?;
        let header = self.parse_header(raw.as_slice())?;
        self.phase_filter(header.tag)?;
        self.verify_signature(raw.as_slice(), &header)?;
        let payload = self.parse_payload(raw.as_slice(), &header)?;
        Ok(MessageOwned { header, payload })
    }

    fn decrypt(&self, encrypted_message: Vec<u8>) -> Result<Vec<u8>, RequestFailed> {
        Ok(self
            .keys
            .secret
            .decrypt(&encrypted_message.as_ref(), &self.keys.public)
            .map_err(|_| RequestFailed::Decrypt)?)
    }

    fn parse_header(&self, raw_message: &[u8]) -> Result<HeaderOwned, RequestFailed> {
        Ok(HeaderOwned::from_bytes(&&raw_message[Signature::LENGTH..])
            .map_err(|e| RequestFailed::Parsing(e))?)
    }

    fn phase_filter(&self, tag: Tag) -> Result<(), RequestFailed> {
        match (tag, self.phase) {
            (Tag::Sum, Phase::Sum) | (Tag::Update, Phase::Update) | (Tag::Sum2, Phase::Sum2) => {
                Ok(())
            }
            (_tag, _phase) => Err(RequestFailed::UnexpectedMessage),
        }
    }

    fn verify_signature(
        &self,
        raw_message: &[u8],
        header: &HeaderOwned,
    ) -> Result<(), RequestFailed> {
        // UNWRAP_SAFE: We already parsed the header, so we now the
        // message is at least as big as: signature length + header
        // length
        let signature = Signature::from_slice(&raw_message[..Signature::LENGTH]).unwrap();
        let bytes = &raw_message[Signature::LENGTH..];
        if header.participant_pk.verify_detached(&signature, bytes) {
            Ok(())
        } else {
            Err(RequestFailed::InvalidMessageSignature)
        }
    }

    fn parse_payload(
        &self,
        raw_message: &[u8],
        header: &HeaderOwned,
    ) -> Result<PayloadOwned, RequestFailed> {
        let bytes = &raw_message[header.buffer_length() + Signature::LENGTH..];
        match header.tag {
            Tag::Sum => {
                let parsed = SumOwned::from_bytes(&bytes)
                    .map_err(|e| RequestFailed::Parsing(e.context("invalid sum payload")))?;
                Ok(PayloadOwned::Sum(parsed))
            }
            Tag::Update => {
                let parsed = UpdateOwned::from_bytes(&bytes)
                    .map_err(|e| RequestFailed::Parsing(e.context("invalid update payload")))?;
                Ok(PayloadOwned::Update(parsed))
            }
            Tag::Sum2 => {
                let parsed = Sum2Owned::from_bytes(&bytes)
                    .map_err(|e| RequestFailed::Parsing(e.context("invalid sum2 payload")))?;
                Ok(PayloadOwned::Sum2(parsed))
            }
        }
    }
}
