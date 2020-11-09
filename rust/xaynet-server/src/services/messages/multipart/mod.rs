mod buffer;
mod service;

use std::task::{Context, Poll};

use futures::future::TryFutureExt;
use tower::{buffer::Buffer, Service, ServiceBuilder};

use crate::services::messages::ServiceError;
use xaynet_core::message::Message;

type Inner = Buffer<service::MultipartHandler, Message>;

#[derive(Clone)]
pub struct MultipartHandler(Inner);

impl Service<Message> for MultipartHandler {
    type Response = Option<Message>;
    type Error = ServiceError;
    #[allow(clippy::type_complexity)]
    type Future = futures::future::MapErr<
        <Inner as Service<Message>>::Future,
        fn(<Inner as Service<Message>>::Error) -> ServiceError,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <Inner as Service<Message>>::poll_ready(&mut self.0, cx).map_err(ServiceError::from)
    }

    fn call(&mut self, req: Message) -> Self::Future {
        <<Inner as Service<Message>>::Future>::map_err(self.0.call(req), ServiceError::from)
    }
}

impl MultipartHandler {
    pub fn new() -> Self {
        Self(
            ServiceBuilder::new()
                .buffer(100)
                .service(service::MultipartHandler::new()),
        )
    }
}
