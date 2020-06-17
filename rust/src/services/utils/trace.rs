use std::task::Poll;

use futures::task::Context;
use tower::{layer::Layer, Service};
use tracing::Span;
use tracing_futures::{Instrument, Instrumented};

pub trait Traceable {
    type Target: Sized;
    fn span(&self) -> &Span;
    fn span_mut(&mut self) -> &mut Span;
    fn into_inner(self) -> Self::Target;
}

#[derive(Debug, Clone)]
pub struct TracingService<S> {
    service: S,
}

impl<R, S> Service<R> for TracingService<S>
where
    S: Service<<R as Traceable>::Target>,
    R: Traceable,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Instrumented<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: R) -> Self::Future {
        let svc_span = req.span().clone();
        let _enter = svc_span.enter();
        let fut_span = req.span().clone();
        self.service.call(req.into_inner()).instrument(fut_span)
    }
}

pub struct TracingLayer;

impl<S> Layer<S> for TracingLayer {
    type Service = TracingService<S>;

    fn layer(&self, service: S) -> Self::Service {
        TracingService { service }
    }
}

/// A request
#[derive(Debug, Hash, Clone)]
pub struct Traced<R> {
    inner: R,
    span: Span,
}

impl<R> Traced<R> {
    pub fn new(req: R, span: Span) -> Self {
        Self { inner: req, span }
    }
}

impl<R> Traceable for Traced<R> {
    type Target = R;

    fn span(&self) -> &Span {
        &self.span
    }
    fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }
    fn into_inner(self) -> R {
        self.inner
    }
}
