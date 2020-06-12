use std::task::{Context, Poll};

use futures::future::{self, Ready};
use tower::Service;

use crate::state_machine::{
    coordinator::RoundParameters,
    events::{EventListener, EventSubscriber},
};

pub struct RoundParamsRequest;
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
