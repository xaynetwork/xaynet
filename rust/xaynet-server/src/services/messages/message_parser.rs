use std::{convert::TryInto, sync::Arc, task::Poll};

use futures::{future, task::Context};
use rayon::ThreadPool;
use tokio::sync::oneshot;
use tower::{layer::Layer, limit::concurrency::ConcurrencyLimit, Service, ServiceBuilder};
use xaynet_core::{
    crypto::{EncryptKeyPair, PublicEncryptKey},
    message::{FromBytes, Message, MessageBuffer, Tag},
};

use crate::{
    services::messages::{BoxedServiceFuture, ServiceError},
    state_machine::{
        events::{EventListener, EventSubscriber},
        phases::PhaseName,
    },
};

/// A type that hold a un-parsed message
struct RawMessage<T> {
    /// The buffer that contains the message to parse
    buffer: Arc<MessageBuffer<T>>,
}

impl<T> Clone for RawMessage<T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
        }
    }
}

impl<T> From<MessageBuffer<T>> for RawMessage<T> {
    fn from(buffer: MessageBuffer<T>) -> Self {
        RawMessage {
            buffer: Arc::new(buffer),
        }
    }
}

/// A service that wraps a buffer `T` representing a message into a
/// [`RawMessage<T>`]
#[derive(Debug, Clone)]
struct BufferWrapper<S>(S);

impl<S, T> Service<T> for BufferWrapper<S>
where
    T: AsRef<[u8]> + Send + 'static,
    S: Service<RawMessage<T>, Response = Message, Error = ServiceError>,
    S::Future: Sync + Send + 'static,
{
    type Response = Message;
    type Error = ServiceError;
    type Future = BoxedServiceFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, req: T) -> Self::Future {
        debug!("creating a RawMessage request");
        match MessageBuffer::new(req) {
            Ok(buffer) => {
                let fut = self.0.call(RawMessage::from(buffer));
                Box::pin(async move {
                    trace!("calling inner service");
                    fut.await
                })
            }
            Err(e) => Box::pin(future::ready(Err(ServiceError::Parsing(e)))),
        }
    }
}

struct BufferWrapperLayer;

impl<S> Layer<S> for BufferWrapperLayer {
    type Service = BufferWrapper<S>;

    fn layer(&self, service: S) -> BufferWrapper<S> {
        BufferWrapper(service)
    }
}

/// A service that discards messages that are not expected in the current phase
#[derive(Debug, Clone)]
struct PhaseFilter<S> {
    /// A listener to retrieve the current phase
    phase: EventListener<PhaseName>,
    /// Next service to be called
    next_svc: S,
}

impl<T, S> Service<RawMessage<T>> for PhaseFilter<S>
where
    T: AsRef<[u8]> + Send + 'static,
    S: Service<RawMessage<T>, Response = Message, Error = ServiceError>,
    S::Future: Sync + Send + 'static,
{
    type Response = Message;
    type Error = ServiceError;
    type Future = BoxedServiceFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.next_svc.poll_ready(cx)
    }

    fn call(&mut self, req: RawMessage<T>) -> Self::Future {
        debug!("retrieving the current phase");
        let phase = self.phase.get_latest().event;
        match req.buffer.tag().try_into() {
            Ok(tag) => match (phase, tag) {
                (PhaseName::Sum, Tag::Sum)
                | (PhaseName::Update, Tag::Update)
                | (PhaseName::Sum2, Tag::Sum2) => {
                    let fut = self.next_svc.call(req);
                    Box::pin(async move { fut.await })
                }
                _ => Box::pin(future::ready(Err(ServiceError::UnexpectedMessage))),
            },
            Err(e) => Box::pin(future::ready(Err(ServiceError::Parsing(e)))),
        }
    }
}

struct PhaseFilterLayer {
    phase: EventListener<PhaseName>,
}

impl<S> Layer<S> for PhaseFilterLayer {
    type Service = PhaseFilter<S>;

    fn layer(&self, service: S) -> PhaseFilter<S> {
        PhaseFilter {
            phase: self.phase.clone(),
            next_svc: service,
        }
    }
}

/// A service for verifying the signature of PET messages
///
/// Since this is a CPU-intensive task for large messages, this
/// service offloads the processing to a `rayon` thread-pool to avoid
/// overloading the tokio thread-pool with blocking tasks.
#[derive(Debug, Clone)]
struct SignatureVerifier<S> {
    /// Thread-pool the CPU-intensive tasks are offloaded to.
    thread_pool: Arc<ThreadPool>,
    /// The service to be called after the [`SignatureVerifier`]
    next_svc: S,
}

impl<T, S> Service<RawMessage<T>> for SignatureVerifier<S>
where
    T: AsRef<[u8]> + Sync + Send + 'static,
    S: Service<RawMessage<T>, Response = Message, Error = ServiceError>
        + Clone
        + Sync
        + Send
        + 'static,
    S::Future: Sync + Send + 'static,
{
    type Response = Message;
    type Error = ServiceError;
    type Future = BoxedServiceFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.next_svc.poll_ready(cx)
    }

    fn call(&mut self, req: RawMessage<T>) -> Self::Future {
        let (tx, rx) = oneshot::channel::<Result<(), ServiceError>>();

        let req_clone = req.clone();
        trace!("spawning signature verification task on thread-pool");
        self.thread_pool.spawn(move || {
            let res = match req.buffer.as_ref().as_ref().check_signature() {
                Ok(()) => {
                    info!("found a valid message signature");
                    Ok(())
                }
                Err(e) => {
                    warn!("invalid message signature: {:?}", e);
                    Err(ServiceError::InvalidMessageSignature)
                }
            };
            let _ = tx.send(res);
        });

        let mut next_svc = self.next_svc.clone();
        let fut = async move {
            rx.await.map_err(|_| {
                ServiceError::InternalError(
                    "failed to receive response from thread-pool".to_string(),
                )
            })??;
            next_svc.call(req_clone).await
        };
        Box::pin(fut)
    }
}

struct SignatureVerifierLayer {
    thread_pool: Arc<ThreadPool>,
}

impl<S> Layer<S> for SignatureVerifierLayer {
    type Service = ConcurrencyLimit<SignatureVerifier<S>>;

    fn layer(&self, service: S) -> Self::Service {
        let limit = self.thread_pool.current_num_threads();
        // FIXME: we actually want to limit the concurrency of just
        // the SignatureVerifier middleware. Right now we're limiting
        // the whole stack of services.
        ConcurrencyLimit::new(
            SignatureVerifier {
                thread_pool: self.thread_pool.clone(),
                next_svc: service,
            },
            limit,
        )
    }
}

/// A service that verifies the coordinator public key embedded in PET
/// messsages
#[derive(Debug, Clone)]
struct CoordinatorPublicKeyValidator<S> {
    /// A listener to retrieve the latest coordinator keys
    keys: EventListener<EncryptKeyPair>,
    /// Next service to be called
    next_svc: S,
}

impl<T, S> Service<RawMessage<T>> for CoordinatorPublicKeyValidator<S>
where
    T: AsRef<[u8]> + Send + 'static,
    S: Service<RawMessage<T>, Response = Message, Error = ServiceError>,
    S::Future: Sync + Send + 'static,
{
    type Response = Message;
    type Error = ServiceError;
    type Future = BoxedServiceFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.next_svc.poll_ready(cx)
    }

    fn call(&mut self, req: RawMessage<T>) -> Self::Future {
        debug!("retrieving the current keys");
        let coord_pk = self.keys.get_latest().event.public;
        match PublicEncryptKey::from_byte_slice(&req.buffer.as_ref().as_ref().coordinator_pk()) {
            Ok(pk) => {
                if pk != coord_pk {
                    warn!("found an invalid coordinator public key");
                    Box::pin(future::ready(Err(
                        ServiceError::InvalidCoordinatorPublicKey,
                    )))
                } else {
                    info!("found a valid coordinator public key");
                    let fut = self.next_svc.call(req);
                    Box::pin(async move { fut.await })
                }
            }
            Err(_) => Box::pin(future::ready(Err(
                ServiceError::InvalidCoordinatorPublicKey,
            ))),
        }
    }
}

struct CoordinatorPublicKeyValidatorLayer {
    keys: EventListener<EncryptKeyPair>,
}

impl<S> Layer<S> for CoordinatorPublicKeyValidatorLayer {
    type Service = CoordinatorPublicKeyValidator<S>;

    fn layer(&self, service: S) -> CoordinatorPublicKeyValidator<S> {
        CoordinatorPublicKeyValidator {
            keys: self.keys.clone(),
            next_svc: service,
        }
    }
}

#[derive(Debug, Clone)]
struct Parser;

impl<T> Service<RawMessage<T>> for Parser
where
    T: AsRef<[u8]> + Send + 'static,
{
    type Response = Message;
    type Error = ServiceError;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RawMessage<T>) -> Self::Future {
        let bytes = req.buffer.inner();
        future::ready(Message::from_byte_slice(&bytes).map_err(ServiceError::Parsing))
    }
}

type InnerService = BufferWrapper<
    PhaseFilter<ConcurrencyLimit<SignatureVerifier<CoordinatorPublicKeyValidator<Parser>>>>,
>;

#[derive(Debug, Clone)]
pub struct MessageParser(InnerService);

impl<T> Service<T> for MessageParser
where
    T: AsRef<[u8]> + Sync + Send + 'static,
{
    type Response = Message;
    type Error = ServiceError;
    type Future = BoxedServiceFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <InnerService as Service<T>>::poll_ready(&mut self.0, cx)
    }

    fn call(&mut self, req: T) -> Self::Future {
        let fut = self.0.call(req);
        Box::pin(async move { fut.await })
    }
}

impl MessageParser {
    pub fn new(events: &EventSubscriber, thread_pool: Arc<ThreadPool>) -> Self {
        let inner = ServiceBuilder::new()
            .layer(BufferWrapperLayer)
            .layer(PhaseFilterLayer {
                phase: events.phase_listener(),
            })
            .layer(SignatureVerifierLayer { thread_pool })
            .layer(CoordinatorPublicKeyValidatorLayer {
                keys: events.keys_listener(),
            })
            .service(Parser);
        Self(inner)
    }
}

#[cfg(test)]
mod tests {
    use rayon::ThreadPoolBuilder;
    use tokio_test::assert_ready;
    use tower_test::mock::Spawn;

    use super::*;
    use crate::{
        services::tests::utils,
        state_machine::events::{EventPublisher, EventSubscriber},
    };

    fn spawn_svc() -> (EventPublisher, EventSubscriber, Spawn<MessageParser>) {
        let (publisher, subscriber) = utils::new_event_channels();
        let thread_pool = Arc::new(ThreadPoolBuilder::new().build().unwrap());
        let task = Spawn::new(MessageParser::new(&subscriber, thread_pool));
        (publisher, subscriber, task)
    }

    #[tokio::test]
    async fn test_valid_request() {
        let (mut publisher, subscriber, mut task) = spawn_svc();
        assert_ready!(task.poll_ready::<Vec<u8>>()).unwrap();

        let round_params = subscriber.params_listener().get_latest().event;
        let (message, signing_keys) = utils::new_sum_message(&round_params);
        let serialized_message = utils::serialize_message(&message, &signing_keys);

        // Simulate the state machine broadcasting the sum phase
        // (otherwise the request will be rejected by the phase
        // filter)
        publisher.broadcast_phase(PhaseName::Sum);

        // Call the service
        let mut resp = task.call(serialized_message).await.unwrap();
        // The signature should be set. However in `message` it's not been
        // computed, so we just check that it's there, then set it to
        // `None` in `resp`
        assert!(resp.signature.is_some());
        resp.signature = None;
        // Now the comparison should work
        assert_eq!(resp, message);
    }

    #[tokio::test]
    async fn test_unexpected_message() {
        let (_publisher, subscriber, mut task) = spawn_svc();
        assert_ready!(task.poll_ready::<Vec<u8>>()).unwrap();

        let round_params = subscriber.params_listener().get_latest().event;
        let (message, signing_keys) = utils::new_sum_message(&round_params);
        let serialized_message = utils::serialize_message(&message, &signing_keys);
        let err = task.call(serialized_message).await.unwrap_err();
        match err {
            ServiceError::UnexpectedMessage => {}
            _ => panic!("expected ServiceError::UnexpectedMessage got {:?}", err),
        }
    }
}
