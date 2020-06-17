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
use tower::{Service, ServiceBuilder};
use tracing_futures::Instrument;
use uuid::Uuid;

use crate::{
    message::MessageOwned,
    services::{
        message_parser::{MessageParserRequest, MessageParserResponse},
        pre_processor::{PreProcessorRequest, PreProcessorResponse},
        state_machine::{StateMachineRequest, StateMachineResponse},
        utils::trace::{Traceable, Traced, TracingLayer, TracingService},
    },
};

#[derive(Clone)]
pub struct CoordinatorService<MessageParser, PreProcessor, StateMachine> {
    message_parser: TracingService<MessageParser>,
    pre_processor: TracingService<PreProcessor>,
    state_machine: TracingService<StateMachine>,
}

impl<MessageParser, PreProcessor, StateMachine>
    CoordinatorService<MessageParser, PreProcessor, StateMachine>
where
    MessageParser: Service<MessageParserRequest, Response = MessageParserResponse>,
    PreProcessor: Service<PreProcessorRequest, Response = PreProcessorResponse>,
    StateMachine: Service<StateMachineRequest, Response = StateMachineResponse>,
{
    pub fn new(
        message_parser: MessageParser,
        pre_processor: PreProcessor,
        state_machine: StateMachine,
    ) -> Self {
        Self {
            message_parser: Self::with_tracing(message_parser),
            pre_processor: Self::with_tracing(pre_processor),
            state_machine: Self::with_tracing(state_machine),
        }
    }

    fn with_tracing<S, R>(service: S) -> TracingService<S>
    where
        S: Service<R>,
    {
        ServiceBuilder::new().layer(TracingLayer).service(service)
    }

    pub fn make_traceable_request<R>(req: R) -> Traced<R> {
        let id = Uuid::new_v4();
        let span = trace_span!("request", id = ?id);
        Traced::new(req, span)
    }
}

impl<MessageParser, PreProcessor, StateMachine> Service<MessageParserRequest>
    for CoordinatorService<MessageParser, PreProcessor, StateMachine>
where
    MessageParser:
        Service<MessageParserRequest, Response = MessageParserResponse> + Clone + 'static + Send,
    <MessageParser as Service<MessageParserRequest>>::Future: 'static + Send,
    <MessageParser as Service<MessageParserRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,
    PreProcessor:
        Service<PreProcessorRequest, Response = PreProcessorResponse> + Clone + 'static + Send,
    <PreProcessor as Service<PreProcessorRequest>>::Future: 'static + Send,
    <PreProcessor as Service<PreProcessorRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,
    StateMachine:
        Service<StateMachineRequest, Response = StateMachineResponse> + Clone + 'static + Send,
    <StateMachine as Service<StateMachineRequest>>::Future: 'static + Send,
    <StateMachine as Service<StateMachineRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,
{
    type Response = StateMachineResponse;
    type Error = Box<dyn ::std::error::Error + 'static + Send + Sync>;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'static + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <TracingService<MessageParser> as Service<Traced<MessageParserRequest>>>::poll_ready(
            &mut self.message_parser,
            cx,
        )
        .map_err(Into::into)
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

        let fut =
            async move {
                let resp: MessageOwned = match message_parser.call(req).await.map_err(Into::into)? {
                    Ok(message) => message,
                    Err(e) => return Ok(Err(e)),
                };

                poll_fn(|cx| {
                <TracingService<PreProcessor> as Service<Traced<PreProcessorRequest>>>::poll_ready(
                    &mut pre_processor,
                    cx,
                )
            })
            .await.map_err(Into::into)?;
                let resp: StateMachineRequest = match pre_processor
                    .call(Traced::new(resp, span_clone.clone()))
                    .await
                    .map_err(Into::into)?
                {
                    Ok(resp) => resp,
                    Err(e) => return Ok(Err(e)),
                };

                poll_fn(|cx| {
                <TracingService<StateMachine> as Service<Traced<StateMachineRequest>>>::poll_ready(
                    &mut state_machine,
                    cx,
                )
            })
            .await.map_err(Into::into)?;
                state_machine
                    .call(Traced::new(resp, span_clone.clone()))
                    .await
                    .map_err(Into::into)
            };
        Box::pin(fut.instrument(fut_span))
    }
}
