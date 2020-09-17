use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::future::{self, Ready};
use tower::Service;
use tracing_futures::{Instrument, Instrumented};
use xaynet_core::SumDict;

use crate::state_machine::events::{DictionaryUpdate, EventListener, EventSubscriber};

/// A service that returns the sum dictionary for the current round.
pub struct SumDictService(EventListener<DictionaryUpdate<SumDict>>);

/// [`SumDictService`]'s request type
#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub struct SumDictRequest;

/// [`SumDictService`]'s response type.
///
/// The response is `None` when no sum dictionary is currently
/// available
pub type SumDictResponse = Option<Arc<SumDict>>;

impl SumDictService {
    pub fn new(events: &EventSubscriber) -> Self {
        Self(events.sum_dict_listener())
    }
}

impl Service<SumDictRequest> for SumDictService {
    type Response = SumDictResponse;
    type Error = std::convert::Infallible;
    type Future = Instrumented<Ready<Result<Self::Response, Self::Error>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: SumDictRequest) -> Self::Future {
        future::ready(match self.0.get_latest().event {
            DictionaryUpdate::Invalidate => Ok(None),
            DictionaryUpdate::New(dict) => Ok(Some(dict)),
        })
        .instrument(error_span!("sum_dict_fetch_request"))
    }
}
