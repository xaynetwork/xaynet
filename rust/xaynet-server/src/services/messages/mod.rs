//! This module provides the services for processing PET messages.
//!
//! There are multiple such services and [`PetMessageHandler`]
//! provides a single unifying interface for all of these.

mod decryptor;
mod error;
mod message_parser;
mod multipart;
mod state_machine;
mod task_validator;

use std::sync::Arc;

use futures::future::poll_fn;
use rayon::ThreadPoolBuilder;
use tower::Service;
use xaynet_core::message::Message;

pub use self::error::ServiceError;
use self::{
    decryptor::Decryptor,
    message_parser::MessageParser,
    multipart::MultipartHandler,
    state_machine::StateMachine,
    task_validator::TaskValidator,
};
use crate::state_machine::{events::EventSubscriber, requests::RequestSender};

impl PetMessageHandler {
    pub fn new(event_subscriber: &EventSubscriber, requests_tx: RequestSender) -> Self {
        // TODO: make this configurable. Users should be able to
        // choose how many threads they want etc.
        //
        // TODO: don't unwrap
        let thread_pool = Arc::new(ThreadPoolBuilder::new().build().unwrap());
        let decryptor = Decryptor::new(event_subscriber, thread_pool.clone());
        let multipart_handler = MultipartHandler::new();
        let message_parser = MessageParser::new(event_subscriber, thread_pool);
        let task_validator = TaskValidator::new(event_subscriber);
        let state_machine = StateMachine::new(requests_tx);

        Self {
            decryptor,
            multipart_handler,
            message_parser,
            task_validator,
            state_machine,
        }
    }
    async fn decrypt(&mut self, enc_data: Vec<u8>) -> Result<Vec<u8>, ServiceError> {
        poll_fn(|cx| <Decryptor as Service<Vec<u8>>>::poll_ready(&mut self.decryptor, cx)).await?;
        self.decryptor.call(enc_data).await
    }

    async fn parse(&mut self, data: Vec<u8>) -> Result<Message, ServiceError> {
        poll_fn(|cx| <MessageParser as Service<Vec<u8>>>::poll_ready(&mut self.message_parser, cx))
            .await?;
        self.message_parser.call(data).await
    }

    async fn handle_multipart(
        &mut self,
        message: Message,
    ) -> Result<Option<Message>, ServiceError> {
        poll_fn(|cx| self.multipart_handler.poll_ready(cx)).await?;
        self.multipart_handler.call(message).await
    }

    async fn validate_task(&mut self, message: Message) -> Result<Message, ServiceError> {
        poll_fn(|cx| self.task_validator.poll_ready(cx)).await?;
        self.task_validator.call(message).await
    }

    async fn process(&mut self, message: Message) -> Result<(), ServiceError> {
        poll_fn(|cx| self.state_machine.poll_ready(cx)).await?;
        self.state_machine.call(message).await
    }

    pub async fn handle_message(&mut self, enc_data: Vec<u8>) -> Result<(), ServiceError> {
        let raw_message = self.decrypt(enc_data).await?;
        let message = self.parse(raw_message).await?;
        match self.handle_multipart(message).await? {
            Some(message) => {
                let message = self.validate_task(message).await?;
                self.process(message).await
            }
            None => Ok(()),
        }
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
/// 2. The message is passed to the `TaskValidator`, which depending on
///    the message type performs some additional checks. The
///    `TaskValidator` may also discard the message
///
/// 3. Finally, the message is handled by the `StateMachine` service.
#[derive(Clone)]
pub struct PetMessageHandler {
    decryptor: Decryptor,
    multipart_handler: MultipartHandler,
    message_parser: MessageParser,
    task_validator: TaskValidator,
    state_machine: StateMachine,
}

pub type BoxedServiceFuture<Response, Error> = std::pin::Pin<
    Box<dyn futures::Future<Output = Result<Response, Error>> + 'static + Send + Sync>,
>;
