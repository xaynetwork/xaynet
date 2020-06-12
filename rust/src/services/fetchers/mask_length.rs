use std::task::{Context, Poll};

use futures::future::{self, Ready};
use tower::Service;

use crate::state_machine::events::{EventListener, EventSubscriber, MaskLengthUpdate};

pub struct MaskLengthRequest;
pub type MaskLengthResponse = Option<usize>;

/// A service that serves the mask_length for the current round.
pub struct MaskLengthService(EventListener<MaskLengthUpdate>);

impl MaskLengthService {
    pub fn new(events: &EventSubscriber) -> Self {
        Self(events.mask_length_listener())
    }
}

impl Service<MaskLengthRequest> for MaskLengthService {
    type Response = MaskLengthResponse;
    type Error = ::std::convert::Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: MaskLengthRequest) -> Self::Future {
        future::ready(match self.0.get_latest().event {
            MaskLengthUpdate::Invalidate => Ok(None),
            MaskLengthUpdate::New(mask_length) => Ok(Some(mask_length)),
        })
    }
}
