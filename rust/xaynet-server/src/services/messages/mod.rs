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

use xaynet_core::message::Message;

use crate::{
    services::{
        messages::message_parser::RawMessage,
        utils::{with_tracing, TracedService},
    },
    utils::Request,
};

use futures::future::poll_fn;
use thiserror::Error;
use tower::Service;

type TracedMessageParser<S> = TracedService<S, RawMessage<Vec<u8>>>;
type TracedPreProcessor<S> = TracedService<S, Message>;
type TracedStateMachine<S> = TracedService<S, Message>;

/// Error returned by the [`PetMessageHandler`] methods.
#[derive(Debug, Error)]
pub enum PetMessageError {
    #[error("failed to parse message: {0}")]
    Parser(MessageParserError),

    #[error("failed to pre-process message: {0}")]
    PreProcessor(PreProcessorError),

    #[error("state machine failed to handle message: {0}")]
    StateMachine(StateMachineError),

    #[error("the service failed to process the request: {0}")]
    ServiceError(Box<dyn ::std::error::Error + Send + Sync + 'static>),
}

/// A single interface for all the PET message processing sub-services
/// ([`MessageParserService`], [`PreProcessorService`] and
/// [`StateMachineService`]).
#[async_trait]
pub trait PetMessageHandler: Send {
    async fn handle_message(
        &mut self,
        // FIXME: this should take a `Request<_>` instead that should
        // be created by the caller (in the rest layer).
        req: Vec<u8>,
    ) -> Result<(), PetMessageError> {
        let req = Request::new(RawMessage::from(req));
        let metadata = req.metadata();
        let message = self.call_parser(req).await?;

        let req = Request::from_parts(metadata.clone(), message);
        let message = self.call_pre_processor(req).await?;

        let req = Request::from_parts(metadata, message);
        Ok(self.call_state_machine(req).await?)
    }

    /// Parse an encrypted message
    async fn call_parser(
        &mut self,
        enc_message: MessageParserRequest<Vec<u8>>,
    ) -> Result<Message, PetMessageError>;

    /// Pre-process a PET message
    async fn call_pre_processor(
        &mut self,
        message: PreProcessorRequest,
    ) -> Result<Message, PetMessageError>;

    /// Have a PET message processed by the state machine
    async fn call_state_machine(
        &mut self,
        message: StateMachineRequest,
    ) -> Result<(), PetMessageError>;
}

#[async_trait]
impl<MP, PP, SM> PetMessageHandler for PetMessageService<MP, PP, SM>
where
    Self: Send + Sync + 'static,

    MP: Service<MessageParserRequest<Vec<u8>>, Response = MessageParserResponse> + Send + 'static,
    <MP as Service<MessageParserRequest<Vec<u8>>>>::Future: Send + 'static,
    <MP as Service<MessageParserRequest<Vec<u8>>>>::Error:
        Into<Box<dyn ::std::error::Error + Send + Sync + 'static>>,

    PP: Service<PreProcessorRequest, Response = PreProcessorResponse> + Send + 'static,
    <PP as Service<PreProcessorRequest>>::Future: Send + 'static,
    <PP as Service<PreProcessorRequest>>::Error:
        Into<Box<dyn ::std::error::Error + Send + Sync + 'static>>,

    SM: Service<StateMachineRequest, Response = StateMachineResponse> + Send + 'static,
    <SM as Service<StateMachineRequest>>::Future: Send + 'static,
    <SM as Service<StateMachineRequest>>::Error:
        Into<Box<dyn ::std::error::Error + Send + Sync + 'static>>,
{
    async fn call_parser(
        &mut self,
        enc_message: MessageParserRequest<Vec<u8>>,
    ) -> Result<Message, PetMessageError> {
        poll_fn(|cx| {
            <MP as Service<MessageParserRequest<Vec<u8>>>>::poll_ready(&mut self.message_parser, cx)
        })
        .await
        // FIXME: we should actually downcast the error and
        // distinguish between the various services errors we can
        // have. Currently, this will just turn the error into a
        // Box<dyn Error>
        .map_err(|e| PetMessageError::ServiceError(Into::into(e)))?;

        <MP as Service<MessageParserRequest<Vec<u8>>>>::call(
            &mut self.message_parser,
            enc_message.map(Into::into),
        )
        .await
        .map_err(|e| PetMessageError::ServiceError(Into::into(e)))?
        .map_err(PetMessageError::Parser)
    }

    async fn call_pre_processor(
        &mut self,
        message: PreProcessorRequest,
    ) -> Result<Message, PetMessageError> {
        poll_fn(|cx| <PP as Service<PreProcessorRequest>>::poll_ready(&mut self.pre_processor, cx))
            .await
            .map_err(|e| PetMessageError::ServiceError(Into::into(e)))?;

        <PP as Service<PreProcessorRequest>>::call(&mut self.pre_processor, message.map(Into::into))
            .await
            .map_err(|e| PetMessageError::ServiceError(Into::into(e)))?
            .map_err(PetMessageError::PreProcessor)
    }

    async fn call_state_machine(
        &mut self,
        message: StateMachineRequest,
    ) -> Result<(), PetMessageError> {
        poll_fn(|cx| <SM as Service<StateMachineRequest>>::poll_ready(&mut self.state_machine, cx))
            .await
            .map_err(|e| PetMessageError::ServiceError(Into::into(e)))?;

        <SM as Service<StateMachineRequest>>::call(&mut self.state_machine, message.map(Into::into))
            .await
            .map_err(|e| PetMessageError::ServiceError(Into::into(e)))?
            .map_err(PetMessageError::StateMachine)
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
#[derive(Debug, Clone)]
pub struct PetMessageService<MessageParser, PreProcessor, StateMachine> {
    message_parser: MessageParser,
    pre_processor: PreProcessor,
    state_machine: StateMachine,
}

impl<MP, PP, SM>
    PetMessageService<TracedMessageParser<MP>, TracedPreProcessor<PP>, TracedStateMachine<SM>>
where
    MP: Service<MessageParserRequest<Vec<u8>>, Response = MessageParserResponse>,
    PP: Service<PreProcessorRequest, Response = PreProcessorResponse>,
    SM: Service<StateMachineRequest, Response = StateMachineResponse>,
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

use crate::utils::Traceable;
use tracing::Span;
use xaynet_core::message::{Payload, ToBytes};

impl Traceable for Message {
    fn make_span(&self) -> Span {
        let message_type = match self.payload {
            Payload::Sum(_) => "sum",
            Payload::Update(_) => "update",
            Payload::Sum2(_) => "sum2",
        };
        error_span!(
            "Message",
            message_type = message_type,
            message_length = self.buffer_length()
        )
    }
}
