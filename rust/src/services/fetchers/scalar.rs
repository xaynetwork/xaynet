use std::task::{Context, Poll};

use futures::future::{self, Ready};
use tower::Service;

use crate::state_machine::events::{EventListener, EventSubscriber, ScalarUpdate};

pub struct ScalarRequest;
pub type ScalarResponse = Option<f64>;

/// A service that serves the scalar for the current round.
pub struct ScalarService(EventListener<ScalarUpdate>);

impl ScalarService {
    pub fn new(events: &EventSubscriber) -> Self {
        Self(events.scalar_listener())
    }
}

impl Service<ScalarRequest> for ScalarService {
    type Response = Option<f64>;
    type Error = std::convert::Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: ScalarRequest) -> Self::Future {
        future::ready(match self.0.get_latest().event {
            ScalarUpdate::Invalidate => Ok(None),
            ScalarUpdate::New(scalar) => Ok(Some(scalar)),
        })
    }
}
