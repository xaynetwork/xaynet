use std::task::{Context, Poll};

use futures::future::{self, Ready};
use tower::Service;
use tracing::Span;

use crate::{
    common::RoundParameters,
    state_machine::events::{EventListener, EventSubscriber},
    utils::Traceable,
};

/// [`RoundParamsService`]'s request type
#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub struct RoundParamsRequest;

impl Traceable for RoundParamsRequest {
    fn make_span(&self) -> Span {
        error_span!("round_params_fetch_request")
    }
}

/// [`RoundParamsService`]'s response type
pub type RoundParamsResponse = RoundParameters;

/// A service that serves the round parameters for the current round.
pub struct RoundParamsService(EventListener<RoundParameters>);

impl RoundParamsService {
    pub fn new(events: &EventSubscriber) -> Self {
        Self(events.params_listener())
    }
}

impl Service<RoundParamsRequest> for RoundParamsService {
    type Response = RoundParameters;
    type Error = ::std::convert::Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: RoundParamsRequest) -> Self::Future {
        future::ready(Ok(self.0.get_latest().event))
    }
}
