use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::future::{self, Ready};
use tower::Service;
use tracing::Span;

use crate::{
    state_machine::events::{DictionaryUpdate, EventListener, EventSubscriber},
    utils::Traceable,
    SeedDict,
};

/// A service that serves the seed dictionary for the current round.
pub struct SeedDictService(EventListener<DictionaryUpdate<SeedDict>>);

impl SeedDictService {
    pub fn new(events: &EventSubscriber) -> Self {
        Self(events.seed_dict_listener())
    }
}

/// [`SeedDictService`]'s request type
#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub struct SeedDictRequest;

impl Traceable for SeedDictRequest {
    fn make_span(&self) -> Span {
        error_span!("seed_dict_fetch_request")
    }
}

/// [`SeedDictService`]'s response type.
///
/// The response is `None` when no seed dictionary is currently
/// available
pub type SeedDictResponse = Option<Arc<SeedDict>>;

impl Service<SeedDictRequest> for SeedDictService {
    type Response = SeedDictResponse;
    type Error = std::convert::Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: SeedDictRequest) -> Self::Future {
        future::ready(match self.0.get_latest().event {
            DictionaryUpdate::Invalidate => Ok(None),
            DictionaryUpdate::New(dict) => Ok(Some(dict)),
        })
    }
}
