use std::{
    pin::Pin,
    task::{Context, Poll},
};

use anyhow::anyhow;
use futures::future::{poll_fn, Future};
use tower::{Service, ServiceBuilder};
use tracing::Span;
use tracing_futures::Instrument;
use uuid::Uuid;

use crate::services::{
    pre_processing::{
        PreProcessingClient,
        PreProcessingClientError,
        PreProcessingRequest,
        PreProcessingResponse,
    },
    state_machine::{
        StateMachineClient,
        StateMachineClientError,
        StateMachineRequest,
        StateMachineResponse,
    },
    trace::{Traceable, Traced},
    transport::{ChannelTransportClient, ChannelTransportServer},
    ServiceError,
};

type PreProcessingClientTransport =
    ChannelTransportClient<Traced<PreProcessingRequest>, PreProcessingResponse>;
type PreProcessingServerTransport =
    ChannelTransportServer<Traced<PreProcessingRequest>, PreProcessingResponse>;

type StateMachineClientTransport =
    ChannelTransportClient<Traced<StateMachineRequest>, StateMachineResponse>;
type StateMachineServerTransport =
    ChannelTransportServer<Traced<StateMachineRequest>, StateMachineResponse>;

pub struct Client {
    pre_processing_client: tower::buffer::Buffer<
        PreProcessingClient<PreProcessingClientTransport>,
        Traced<PreProcessingRequest>,
    >,
    state_machine_client: tower::buffer::Buffer<
        StateMachineClient<StateMachineClientTransport>,
        Traced<StateMachineRequest>,
    >,
}

impl Client {
    pub fn new(
        pre_processing_transport: PreProcessingClientTransport,
        state_machine_transport: StateMachineClientTransport,
    ) -> Self {
        Self {
            pre_processing_client: ServiceBuilder::new()
                .buffer(1000)
                .service(PreProcessingClient::new(pre_processing_transport)),
            state_machine_client: ServiceBuilder::new()
                .buffer(1000)
                .service(StateMachineClient::new(state_machine_transport)),
        }
    }

    pub fn make_traceable_request<R>(req: R) -> Traced<R> {
        let id = Uuid::new_v4();
        let span = trace_span!("request", id = ?id);
        Traced::new(req, span)
    }

    pub fn call_pre_processing(
        &self,
        req: PreProcessingRequest,
    ) -> Pin<Box<dyn Future<Output = PreProcessingResponse> + 'static + Send>> {
        let req = Self::make_traceable_request(req);
        let span = req.span().clone();
        let _enter = span.enter();
        let fut_span = req.span().clone();

        let mut client = self.pre_processing_client.clone();
        Box::pin(async move { client.call(req).await.unwrap() }.instrument(fut_span))
    }

    pub fn call_state_machine(
        &self,
        req: StateMachineRequest,
    ) -> Pin<Box<dyn Future<Output = StateMachineResponse> + 'static + Send>> {
        let req = Self::make_traceable_request(req);
        let span = req.span().clone();
        let _enter = span.enter();
        let fut_span = req.span().clone();

        let mut client = self.state_machine_client.clone();
        Box::pin(async move { client.call(req).await.unwrap() }.instrument(fut_span))
    }

    pub fn call(
        &self,
        req: PreProcessingRequest,
    ) -> Pin<
        Box<dyn Future<Output = Result<StateMachineResponse, CoordinatorError>> + 'static + Send>,
    > {
        let req = Self::make_traceable_request(req);
        let span = req.span().clone();
        let _enter = span.enter();
        let fut_span = req.span().clone();

        let mut pre_processing_client = self.pre_processing_client.clone();
        let mut state_machine_client = self.state_machine_client.clone();

        let fut = async move {
            Ok(
                match pre_processing_client
                    .call(req)
                    .await
                    .map_err(CoordinatorError::from)?
                {
                    Ok(message) => {
                        // FIXME: is this OK??
                        poll_fn(|cx| state_machine_client.poll_ready(cx))
                            .await
                            .map_err(Co)?;
                        state_machine_client
                            .call(Self::make_traceable_request(message.into()))
                            .await
                            .map_err(Into::into)?;
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
            )
        }
        .instrument(fut_span);
        Box::pin(fut)
    }
}

use thiserror::Error;
#[derive(Debug, Error)]
pub enum CoordinatorError {
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Service internal error: {0:?}")]
    Service(ServiceError),
}
impl From<PreProcessingClientError> for CoordinatorError {
    fn from(e: PreProcessingClientError) -> Self {
        match e {
            PreProcessingClientError::Transport(e) => Self::Transport(e),
            PreProcessingClientError::Service(e) => Self::Service(e),
        }
    }
}

impl From<StateMachineClientError> for CoordinatorError {
    fn from(e: StateMachineClientError) -> Self {
        match e {
            StateMachineClientError::Transport(e) => Self::Transport(e),
            StateMachineClientError::Service(e) => Self::Service(e),
        }
    }
}
