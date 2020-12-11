use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::future::{self, Ready};
use tower::Service;
use tracing::error_span;
use tracing_futures::{Instrument, Instrumented};

use crate::state_machine::events::{EventListener, EventSubscriber, SpecUpdate};
use xaynet_core::mask::Analytic;

/// [`SpecService`]'s request type
#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub struct SpecRequest;

/// [`SpecService`]'s response type.
///
/// The response is `None` when no spec is currently available.
pub type SpecResponse = Option<Arc<Analytic>>;

/// A service that serves the latest available global spec
pub struct SpecService(EventListener<SpecUpdate>);

impl SpecService {
    pub fn new(events: &EventSubscriber) -> Self {
        Self(events.spec_listener())
    }
}

impl Service<SpecRequest> for SpecService {
    type Response = SpecResponse;
    type Error = ::std::convert::Infallible;
    type Future = Instrumented<Ready<Result<Self::Response, Self::Error>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: SpecRequest) -> Self::Future {
        future::ready(match self.0.get_latest().event {
            SpecUpdate::Invalidate => Ok(None),
            SpecUpdate::New(spec) => Ok(Some(spec)),
        })
        .instrument(error_span!("spec_fetch_request"))
    }
}
