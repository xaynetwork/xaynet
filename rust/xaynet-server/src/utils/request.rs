use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::Span;
use uuid::Uuid;

/// A type that can be associated to a span, making it traceable.
pub trait Traceable {
    fn make_span(&self) -> Span;
}

impl<'a, T> Traceable for &'a T
where
    T: Traceable,
{
    fn make_span(&self) -> Span {
        <T as Traceable>::make_span(*self)
    }
}

// NOTE: currently `id` and `timestamp` are immutable. `span` is
// mutable, but when it is changed other copies of the RequestMetadata
// are not affected. In the future, we can have shared mutable fields
// if we want to, by adding an Arc<Lock<_>> field.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestMetadata {
    /// A random UUID associated to the request
    id: Uuid,
    /// Time the request was created
    timestamp: SystemTime,
    /// Current span associated to this request
    span: Span,
}

impl RequestMetadata {
    fn new() -> Self {
        let id = Uuid::new_v4();
        let timestamp = SystemTime::now();
        let span = error_span!("request", id = %id, timestamp = %timestamp.duration_since(UNIX_EPOCH).unwrap_or_else(|_| Duration::new(0, 0)).as_millis());
        Self {
            id,
            timestamp,
            span,
        }
    }

    /// Time elapsed since this request was created, in milli seconds
    fn elapsed(&self) -> u128 {
        SystemTime::now()
            .duration_since(self.timestamp)
            .unwrap_or_else(|_| Duration::new(0, 0))
            .as_millis()
    }

    /// Return the span associated with the metadata
    fn span(&self) -> Span {
        self.span.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
/// A request that can be handled by a service
pub struct Request<T> {
    /// Content of the request
    inner: T,
    /// Metadata associated to this request
    metadata: RequestMetadata,
}

impl<T> Request<T>
where
    T: Traceable,
{
    /// Create a new request
    pub fn new(t: T) -> Self {
        Self {
            inner: t,
            metadata: RequestMetadata::new(),
        }
    }

    /// Create a [`Request<T>`] with the given metadata and inner
    /// request value.
    pub fn from_parts(metadata: RequestMetadata, inner: T) -> Self {
        Self { metadata, inner }
    }

    /// Return the metadata attached to this [`Request<T>`]
    pub fn metadata(&self) -> RequestMetadata {
        self.metadata.clone()
    }

    /// Turn this `Request<T>` into a `Request<U>`. A new span is
    /// created with `<U as Traceable>::make_span` and attached to the
    /// request.
    pub fn map<F, U>(self, f: F) -> Request<U>
    where
        F: ::std::ops::FnOnce(T) -> U,
        U: Traceable,
    {
        let Request {
            mut metadata,
            inner,
        } = self;
        let mapped = f(inner);

        // self.span() is the parent of the span associated to the
        // inner type
        let new_span = metadata.span().in_scope(|| mapped.make_span());
        metadata.span = new_span;

        Request {
            metadata,
            inner: mapped,
        }
    }

    /// Span associated with this request
    pub fn span(&self) -> Span {
        self.metadata.span()
    }

    /// Time elapsed since this request was created, in milli seconds
    pub fn elapsed(&self) -> u128 {
        self.metadata.elapsed()
    }

    /// Get a reference to the request's inner value
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the request's inner value
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consume this request and return its inner value
    pub fn into_inner(self) -> T {
        self.inner
    }
}
