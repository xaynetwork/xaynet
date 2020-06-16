use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::future::{self, Ready};
use tower::Service;

use crate::{
    events::{DictionaryUpdate, EventListener},
    services::error::{RequestFailed, ServiceError},
    SeedDict,
};

/// A service that serves the seed dictionary for the current round.
struct SeedDictService {
    updates: EventListener<DictionaryUpdate<SeedDict>>,
}

pub struct SeedDictRequest;

impl Service<SeedDictRequest> for SeedDictService {
    type Response = Result<Option<Arc<SeedDict>>, RequestFailed>;
    type Error = ServiceError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: SeedDictRequest) -> Self::Future {
        future::ready(match self.updates.get_latest().event {
            DictionaryUpdate::Invalidate => Ok(Ok(None)),
            DictionaryUpdate::New(dict) => Ok(Ok(Some(dict))),
        })
    }
}
