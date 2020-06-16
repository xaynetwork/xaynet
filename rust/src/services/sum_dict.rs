use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::future::{self, Ready};
use tower::Service;

use crate::{
    events::{DictionaryUpdate, EventListener},
    services::error::{RequestFailed, ServiceError},
    SumDict,
};

/// A service that returns the sum dictionary for the current round.
struct SumDictService {
    updates: EventListener<DictionaryUpdate<SumDict>>,
}

pub struct SumDictRequest;

impl Service<SumDictRequest> for SumDictService {
    type Response = Result<Option<Arc<SumDict>>, RequestFailed>;
    type Error = ServiceError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: SumDictRequest) -> Self::Future {
        future::ready(match self.updates.get_latest().event {
            DictionaryUpdate::Invalidate => Ok(Ok(None)),
            DictionaryUpdate::New(dict) => Ok(Ok(Some(dict))),
        })
    }
}
