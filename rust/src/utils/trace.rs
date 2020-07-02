use tracing::Span;
/// A type that can be associated to a span, making it traceable.
pub trait Traceable {
    type Target: Sized;
    fn span(&self) -> &Span;
    fn span_mut(&mut self) -> &mut Span;
    fn into_inner(self) -> Self::Target;
}

/// A wrapper that associates a tracing span to `T`
#[derive(Debug, Hash, Clone)]
pub struct Traced<T> {
    inner: T,
    span: Span,
}

impl<T> Traced<T> {
    pub fn new(req: T, span: Span) -> Self {
        Self { inner: req, span }
    }

    pub fn map<F, U>(self, f: F) -> Traced<U>
    where
        F: ::std::ops::FnOnce(T) -> U,
    {
        let Traced { span, inner } = self;
        Traced {
            span,
            inner: f(inner),
        }
    }
}

impl<T> Traceable for Traced<T> {
    type Target = T;

    fn span(&self) -> &Span {
        &self.span
    }
    fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }
    fn into_inner(self) -> T {
        self.inner
    }
}
