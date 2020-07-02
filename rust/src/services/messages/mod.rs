//! This module provides the services for processing PET
//! messages.
//!
//! There are multiple such services and the [`PetMessageHandler`]
//! trait provides a single unifying interface for all of these.
mod message_parser;
mod pre_processor;
mod state_machine;

pub use self::{
    message_parser::{
        MessageParserError,
        MessageParserRequest,
        MessageParserResponse,
        MessageParserService,
    },
    pre_processor::{
        PreProcessorError,
        PreProcessorRequest,
        PreProcessorResponse,
        PreProcessorService,
    },
    state_machine::{
        StateMachineError,
        StateMachineRequest,
        StateMachineResponse,
        StateMachineService,
    },
};

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{future::poll_fn, Future};
use thiserror::Error;
use tower::{Service, ServiceBuilder};
use tracing_futures::Instrument;
use tracing_tower::request_span;
use uuid::Uuid;

use crate::{
    message::message::MessageOwned,
    utils::trace::{Traceable, Traced},
};

/// Associate an ID to the given request, and attach a span to the request.
fn make_traceable_request<R>(req: R) -> Traced<R> {
    let id = Uuid::new_v4();
    let span = error_span!("request", id = ?id);
    Traced::new(req, span)
}

/// Return the [`tracing::Span`] associated to the given request.
fn req_span<R>(req: &Traced<R>) -> tracing::Span {
    req.span().clone()
}

/// Decorate the given service with a tracing middleware.
fn with_tracing<S, R>(service: S) -> TracingService<S, R>
where
    S: Service<Traced<R>>,
{
    ServiceBuilder::new()
        .layer(request_span::layer(req_span as for<'r> fn(&'r _) -> _))
        .service(service)
}

type TracingService<S, R> = request_span::Service<S, Traced<R>, fn(&Traced<R>) -> tracing::Span>;

/// Error returned by the [`PetMessageHandler`] methods.
#[derive(Debug, Error)]
pub enum PetMessageError {
    #[error("failed to parse message: {0}")]
    Parser(MessageParserError),

    #[error("failed to pre-process message: {0}")]
    PreProcessor(PreProcessorError),

    #[error("state machine failed to handle message: {0}")]
    StateMachine(StateMachineError),
}

#[doc(hidden)]
#[async_trait]
pub trait _PetMessageHandler {
    /// Parse an encrypted message
    async fn call_parser(&self, enc_message: Traced<Vec<u8>>) -> MessageParserResponse;

    /// Pre-process a PET message
    async fn call_pre_processor(&self, message: Traced<MessageOwned>) -> PreProcessorResponse;

    /// Have a PET message processed by the state machine
    async fn call_state_machine(&self, message: Traced<MessageOwned>) -> StateMachineResponse;
}

/// A single interface for all the PET message processing sub-services
/// ([`MessageParserService`], [`PreProcessorService`] and
/// [`StateMachineService`]).
#[async_trait]
pub trait PetMessageHandler {
    /// Handle an incoming encrypted PET message form a participant.
    async fn handle_message(&self, enc_message: Vec<u8>) -> Result<(), PetMessageError>;
}

#[async_trait]
impl<T> PetMessageHandler for T
where
    T: _PetMessageHandler + Sync,
{
    async fn handle_message(&self, enc_message: Vec<u8>) -> Result<(), PetMessageError> {
        let req = make_traceable_request(enc_message);
        let span = req.span().clone();
        let message = self
            .call_parser(req)
            .await
            .map_err(PetMessageError::Parser)?;

        let req = Traced::new(message, span.clone());
        let message = self
            .call_pre_processor(req)
            .await
            .map_err(PetMessageError::PreProcessor)?;

        let req = Traced::new(message, span.clone());
        Ok(self
            .call_state_machine(req)
            .await
            .map_err(PetMessageError::StateMachine)?)
    }
}

#[async_trait]
impl<MP, PP, SM> _PetMessageHandler for PetMessageService<MP, PP, SM>
where
    Self: Clone
        + Send
        + Sync
        + 'static
        + Service<Traced<MessageParserRequest>, Response = MessageParserResponse>
        + Service<Traced<PreProcessorRequest>, Response = PreProcessorResponse>
        + Service<Traced<StateMachineRequest>, Response = StateMachineResponse>,

    <Self as Service<Traced<MessageParserRequest>>>::Future: Send + 'static,
    <Self as Service<Traced<MessageParserRequest>>>::Error:
        Into<Box<dyn ::std::error::Error + Send + Sync + 'static>>,

    <Self as Service<Traced<PreProcessorRequest>>>::Future: Send + 'static,
    <Self as Service<Traced<PreProcessorRequest>>>::Error:
        Into<Box<dyn ::std::error::Error + Send + Sync + 'static>>,

    <Self as Service<Traced<StateMachineRequest>>>::Future: Send + 'static,
    <Self as Service<Traced<StateMachineRequest>>>::Error:
        Into<Box<dyn ::std::error::Error + Send + Sync + 'static>>,
{
    async fn call_parser(&self, enc_message: Traced<Vec<u8>>) -> MessageParserResponse {
        let span = enc_message.span().clone();
        let mut svc = self.clone();
        async move {
            poll_fn(|cx| <Self as Service<Traced<MessageParserRequest>>>::poll_ready(&mut svc, cx))
                .await
                .map_err(Into::into)
                // FIXME: do not unwrap. For now it is fine because we
                // actually only use MessageParserService directly,
                // which never fails.
                .unwrap();
            <Self as Service<Traced<MessageParserRequest>>>::call(
                &mut svc,
                enc_message.map(Into::into),
            )
            .await
            .map_err(Into::into)
            // FIXME: do not unwrap. For now it is fine because we
            // actually only use MessageParserService directly,
            // which never fails.
            .unwrap()
        }
        .instrument(span)
        .await
    }

    async fn call_pre_processor(&self, message: Traced<MessageOwned>) -> PreProcessorResponse {
        let span = message.span().clone();
        let mut svc = self.clone();
        async move {
            poll_fn(|cx| <Self as Service<Traced<PreProcessorRequest>>>::poll_ready(&mut svc, cx))
                .await
                .map_err(Into::into)
                // FIXME: do not unwrap. For now it is fine because we
                // actually only use PreProcessorService directly,
                // which never fails.
                .unwrap();
            <Self as Service<Traced<PreProcessorRequest>>>::call(&mut svc, message.map(Into::into))
                .await
                .map_err(Into::into)
                // FIXME: do not unwrap. For now it is fine because we
                // actually only use PreProcessorService directly,
                // which never fails.
                .unwrap()
        }
        .instrument(span)
        .await
    }

    async fn call_state_machine(&self, message: Traced<MessageOwned>) -> StateMachineResponse {
        let span = message.span().clone();
        let mut svc = self.clone();
        async move {
            poll_fn(|cx| <Self as Service<Traced<StateMachineRequest>>>::poll_ready(&mut svc, cx))
                .await
                .map_err(Into::into)
                // FIXME: do not unwrap. For now it is fine because we
                // actually only use StateMachineService directly,
                // which never fails.
                .unwrap();
            <Self as Service<Traced<StateMachineRequest>>>::call(&mut svc, message.map(Into::into))
                .await
                .map_err(Into::into)
                // FIXME: do not unwrap. For now it is fine because we
                // actually only use StateMachineService directly,
                // which never fails.
                .unwrap()
        }
        .instrument(span)
        .await
    }
}

/// A service that processes requests from the beginning to the
/// end.
///
/// The processing is divided in three phases:
///
/// 1. The raw request (which is just a vector of bytes represented an
///    encrypted message) goes through the `MessageParser` service,
///    which decrypt the message, validates it, and parses it
///
/// 2. The message is passed to the `PreProcessor`, which depending on
///    the message type performs some additional checks. The
///    `PreProcessor` may also discard the message
///
/// 3. Finally, the message is handled by the `StateMachine` service.
#[derive(Clone)]
pub struct PetMessageService<MessageParser, PreProcessor, StateMachine> {
    message_parser: MessageParser,
    pre_processor: PreProcessor,
    state_machine: StateMachine,
}

impl<MP, PP, SM>
    PetMessageService<
        TracingService<MP, MessageParserRequest>,
        TracingService<PP, PreProcessorRequest>,
        TracingService<SM, StateMachineRequest>,
    >
where
    MP: Service<Traced<MessageParserRequest>, Response = MessageParserResponse>,
    PP: Service<Traced<PreProcessorRequest>, Response = PreProcessorResponse>,
    SM: Service<Traced<StateMachineRequest>, Response = StateMachineResponse>,
{
    /// Instantiate a new [`PetMessageService`] with the given sub-services
    pub fn new(message_parser: MP, pre_processor: PP, state_machine: SM) -> Self {
        Self {
            message_parser: with_tracing(message_parser),
            pre_processor: with_tracing(pre_processor),
            state_machine: with_tracing(state_machine),
        }
    }
}

impl<MP, PP, SM> Service<Traced<MessageParserRequest>> for PetMessageService<MP, PP, SM>
where
    MP: Service<Traced<MessageParserRequest>, Response = MessageParserResponse>
        + Clone
        + Send
        + 'static,
    <MP as Service<Traced<MessageParserRequest>>>::Future: Send + 'static,
    <MP as Service<Traced<MessageParserRequest>>>::Error:
        Into<Box<dyn ::std::error::Error + Sync + Send + 'static>>,
{
    type Response = MessageParserResponse;
    type Error = Box<dyn ::std::error::Error + Send + Sync + 'static>;
    #[allow(clippy::type_complexity)]
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <MP as Service<Traced<MessageParserRequest>>>::poll_ready(&mut self.message_parser, cx)
            .map_err(Into::into)
    }

    fn call(&mut self, req: Traced<MessageParserRequest>) -> Self::Future {
        let mut svc = self.message_parser.clone();
        let fut = async move {
            info!("calling the message parser service on the request");
            svc.call(req).await.map_err(Into::into)
        };
        Box::pin(fut)
    }
}

impl<MP, PP, SM> Service<Traced<PreProcessorRequest>> for PetMessageService<MP, PP, SM>
where
    PP: Service<Traced<PreProcessorRequest>, Response = PreProcessorResponse>
        + Clone
        + Send
        + 'static,
    <PP as Service<Traced<PreProcessorRequest>>>::Future: Send + 'static,
    <PP as Service<Traced<PreProcessorRequest>>>::Error:
        Into<Box<dyn ::std::error::Error + Send + Sync + 'static>>,
{
    type Response = PreProcessorResponse;
    type Error = Box<dyn ::std::error::Error + Send + Sync + 'static>;
    #[allow(clippy::type_complexity)]
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <PP as Service<Traced<PreProcessorRequest>>>::poll_ready(&mut self.pre_processor, cx)
            .map_err(Into::into)
    }

    fn call(&mut self, req: Traced<PreProcessorRequest>) -> Self::Future {
        let mut svc = self.pre_processor.clone();
        let fut = async move {
            info!("calling the pre-processor service on the request");
            svc.call(req).await.map_err(Into::into)
        };
        Box::pin(fut)
    }
}

impl<MP, PP, SM> Service<Traced<StateMachineRequest>> for PetMessageService<MP, PP, SM>
where
    SM: Service<Traced<StateMachineRequest>, Response = StateMachineResponse>
        + Clone
        + Send
        + 'static,
    <SM as Service<Traced<StateMachineRequest>>>::Future: Send + 'static,
    <SM as Service<Traced<StateMachineRequest>>>::Error:
        Into<Box<dyn ::std::error::Error + Send + Sync + 'static>>,
{
    type Response = StateMachineResponse;
    type Error = Box<dyn ::std::error::Error + Send + Sync + 'static>;
    #[allow(clippy::type_complexity)]
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <SM as Service<Traced<StateMachineRequest>>>::poll_ready(&mut self.state_machine, cx)
            .map_err(Into::into)
    }

    fn call(&mut self, req: Traced<StateMachineRequest>) -> Self::Future {
        let mut svc = self.state_machine.clone();
        let fut = async move {
            info!("calling the state machine service on the request");
            svc.call(req).await.map_err(Into::into)
        };
        Box::pin(fut)
    }
}
