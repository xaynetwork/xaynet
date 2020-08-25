//! This module contains the bits of
//! https://github.com/tokio-rs/tracing/blob/master/tracing-tower that
//! we're using. We copied them here because without a release of
//! tracing-tower, we cannot publish to crates.io ourself.

// Copyright (c) 2019 Tokio Contributors
//
// Permission is hereby granted, free of charge, to any person
// obtaining a copy of this software and associated documentation
// files (the "Software"), to deal in the Software without
// restriction, including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
// BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
// ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{
    marker::PhantomData,
    task::{Context, Poll},
};
use tracing_futures::Instrument;

pub trait GetSpan<T>: sealed::Sealed<T> {
    fn span_for(&self, target: &T) -> tracing::Span;
}

impl<T, F> sealed::Sealed<T> for F where F: Fn(&T) -> tracing::Span {}

impl<T, F> GetSpan<T> for F
where
    F: Fn(&T) -> tracing::Span,
{
    #[inline]
    fn span_for(&self, target: &T) -> tracing::Span {
        (self)(target)
    }
}

impl<T> sealed::Sealed<T> for tracing::Span {}

impl<T> GetSpan<T> for tracing::Span {
    #[inline]
    fn span_for(&self, _: &T) -> tracing::Span {
        self.clone()
    }
}

mod sealed {
    pub trait Sealed<T = ()> {}
}

#[derive(Debug)]
pub struct Service<S, R, G = fn(&R) -> tracing::Span>
where
    S: tower::Service<R>,
    G: GetSpan<R>,
{
    get_span: G,
    inner: S,
    _p: PhantomData<fn(R)>,
}

pub use self::layer::*;

mod layer {
    use super::*;

    #[derive(Debug)]
    pub struct Layer<R, G = fn(&R) -> tracing::Span>
    where
        G: GetSpan<R> + Clone,
    {
        get_span: G,
        _p: PhantomData<fn(R)>,
    }

    pub fn layer<R, G>(get_span: G) -> Layer<R, G>
    where
        G: GetSpan<R> + Clone,
    {
        Layer {
            get_span,
            _p: PhantomData,
        }
    }

    // === impl Layer ===
    impl<S, R, G> tower::layer::Layer<S> for Layer<R, G>
    where
        S: tower::Service<R>,
        G: GetSpan<R> + Clone,
    {
        type Service = Service<S, R, G>;

        fn layer(&self, service: S) -> Self::Service {
            Service::new(service, self.get_span.clone())
        }
    }

    impl<R, G> Clone for Layer<R, G>
    where
        G: GetSpan<R> + Clone,
    {
        fn clone(&self) -> Self {
            Self {
                get_span: self.get_span.clone(),
                _p: PhantomData,
            }
        }
    }
}

// === impl Service ===

impl<S, R, G> tower::Service<R> for Service<S, R, G>
where
    S: tower::Service<R>,
    G: GetSpan<R> + Clone,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = tracing_futures::Instrumented<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: R) -> Self::Future {
        let span = self.get_span.span_for(&request);
        let _enter = span.enter();
        self.inner.call(request).instrument(span.clone())
    }
}

impl<S, R, G> Clone for Service<S, R, G>
where
    S: tower::Service<R> + Clone,
    G: GetSpan<R> + Clone,
{
    fn clone(&self) -> Self {
        Service {
            get_span: self.get_span.clone(),
            inner: self.inner.clone(),
            _p: PhantomData,
        }
    }
}

impl<S, R, G> Service<S, R, G>
where
    S: tower::Service<R>,
    G: GetSpan<R> + Clone,
{
    pub fn new(inner: S, get_span: G) -> Self {
        Service {
            get_span,
            inner,
            _p: PhantomData,
        }
    }
}
