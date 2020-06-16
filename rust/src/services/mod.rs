pub mod error;
pub mod message_parser;
pub mod pre_processor;
pub mod state_machine;
pub mod utils;

pub mod seed_dict;
pub mod sum_dict;

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::future::{poll_fn, Future};
use tower::Service;
use tracing_futures::Instrument;
use uuid::Uuid;

use crate::{
    message::MessageOwned,
    services::{
        message_parser::{MessageParserRequest, MessageParserResponse},
        pre_processor::{PreProcessorRequest, PreProcessorResponse},
        state_machine::{StateMachineRequest, StateMachineResponse},
        utils::{
            client::{Client, ClientError},
            trace::{Traceable, Traced},
            transport::TransportClient,
        },
    },
};

type MessageParserTransport = TransportClient<Traced<MessageParserRequest>, MessageParserResponse>;
type MessageParserClient =
    Client<MessageParserTransport, MessageParserRequest, MessageParserResponse>;

type PreProcessorTransport = TransportClient<Traced<PreProcessorRequest>, PreProcessorResponse>;
type PreProcessorClient = Client<PreProcessorTransport, PreProcessorRequest, PreProcessorResponse>;

type StateMachineTransport = TransportClient<Traced<StateMachineRequest>, StateMachineResponse>;
type StateMachineClient = Client<StateMachineTransport, StateMachineRequest, StateMachineResponse>;

#[derive(Clone)]
pub struct CoordinatorService {
    message_parser: MessageParserClient,
    pre_processor: PreProcessorClient,
    state_machine: StateMachineClient,
}

impl CoordinatorService {
    pub fn new(
        message_parser: MessageParserClient,
        pre_processor: PreProcessorClient,
        state_machine: StateMachineClient,
    ) -> Self {
        Self {
            message_parser,
            pre_processor,
            state_machine,
        }
    }

    pub fn make_traceable_request<R>(req: R) -> Traced<R> {
        let id = Uuid::new_v4();
        let span = trace_span!("request", id = ?id);
        Traced::new(req, span)
    }
}

impl Service<MessageParserRequest> for CoordinatorService {
    type Response = StateMachineResponse;
    type Error = ClientError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'static + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.message_parser.poll_ready(cx)
    }

    fn call(&mut self, req: MessageParserRequest) -> Self::Future {
        let req = Self::make_traceable_request(req);

        let span = req.span().clone();
        let _enter = span.enter();

        let fut_span = req.span().clone();
        let span_clone = req.span().clone();

        let mut message_parser = self.message_parser.clone();
        let mut pre_processor = self.pre_processor.clone();
        let mut state_machine = self.state_machine.clone();

        let fut = async move {
            let resp: MessageOwned = match message_parser.call(req).await? {
                Ok(message) => message,
                Err(e) => return Ok(Err(e)),
            };

            poll_fn(|cx| pre_processor.poll_ready(cx)).await?;
            let resp: StateMachineRequest = match pre_processor
                .call(Traced::new(resp, span_clone.clone()))
                .await?
            {
                Ok(resp) => resp,
                Err(e) => return Ok(Err(e)),
            };

            poll_fn(|cx| state_machine.poll_ready(cx)).await?;
            state_machine
                .call(Traced::new(resp, span_clone.clone()))
                .await
        };
        Box::pin(fut.instrument(fut_span))
    }
}
