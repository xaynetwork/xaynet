use tower::{Service, ServiceBuilder};

use crate::{
    utils::{Request, Traceable},
    vendor::tracing_tower,
};

/// Return the [`tracing::Span`] associated to the given request.
pub(in crate::services) fn req_span<T: Traceable>(req: &Request<T>) -> tracing::Span {
    req.span()
}

/// Decorate the given service with a tracing middleware.
pub(in crate::services) fn with_tracing<S, T>(service: S) -> TracedService<S, T>
where
    S: Service<Request<T>>,
    T: Traceable,
{
    ServiceBuilder::new()
        .layer(tracing_tower::layer(req_span as for<'r> fn(&'r _) -> _))
        .service(service)
}

/// A service `S` that handles `Request<T>` requests, decorated with a
/// tracing middleware that automatically enters the request's span.
pub type TracedService<S, T> =
    tracing_tower::Service<S, Request<T>, fn(&Request<T>) -> tracing::Span>;
