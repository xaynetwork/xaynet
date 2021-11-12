use std::{pin::Pin, sync::Arc, task::Poll};

use futures::{future::Future, task::Context};
use rayon::ThreadPool;
use tokio::sync::oneshot;
use tower::{
    limit::concurrency::{future::ResponseFuture, ConcurrencyLimit},
    Service,
};
use tracing::{debug, info, trace};

use crate::{
    services::messages::{BoxedServiceFuture, ServiceError},
    state_machine::events::{EventListener, EventSubscriber},
};
use xaynet_core::crypto::EncryptKeyPair;

/// A service for decrypting PET messages.
///
/// Since this is a CPU-intensive task for large messages, this
/// service offloads the processing to a `rayon` thread-pool to avoid
/// overloading the tokio thread-pool with blocking tasks.
#[derive(Clone)]
struct RawDecryptor {
    /// A listener to retrieve the latest coordinator keys. These are
    /// necessary for decrypting messages and verifying their
    /// signature.
    keys_events: EventListener<EncryptKeyPair>,

    /// Thread-pool the CPU-intensive tasks are offloaded to.
    thread_pool: Arc<ThreadPool>,
}

impl<T> Service<T> for RawDecryptor
where
    T: AsRef<[u8]> + Sync + Send + 'static,
{
    type Response = Vec<u8>;
    type Error = ServiceError;
    #[allow(clippy::type_complexity)]
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'static + Send + Sync>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, data: T) -> Self::Future {
        debug!("retrieving the current keys");
        let keys = self.keys_events.get_latest().event;
        let (tx, rx) = oneshot::channel::<Result<Self::Response, Self::Error>>();

        trace!("spawning decryption task on threadpool");
        self.thread_pool.spawn(move || {
            info!("decrypting message");
            let res = keys
                .secret
                .decrypt(data.as_ref(), &keys.public)
                .map_err(|_| ServiceError::Decrypt);
            let _ = tx.send(res);
        });
        Box::pin(async move {
            rx.await.unwrap_or_else(|_| {
                Err(ServiceError::InternalError(
                    "failed to receive response from thread-pool".to_string(),
                ))
            })
        })
    }
}

#[derive(Clone)]
pub struct Decryptor(ConcurrencyLimit<RawDecryptor>);

impl Decryptor {
    pub fn new(state_machine_events: &EventSubscriber, thread_pool: Arc<ThreadPool>) -> Self {
        let limit = thread_pool.current_num_threads();
        let keys_events = state_machine_events.keys_listener();
        let service = RawDecryptor {
            keys_events,
            thread_pool,
        };
        Self(ConcurrencyLimit::new(service, limit))
    }
}

impl<T> Service<T> for Decryptor
where
    T: AsRef<[u8]> + Sync + Send + 'static,
{
    type Response = Vec<u8>;
    type Error = ServiceError;
    type Future = ResponseFuture<BoxedServiceFuture<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <ConcurrencyLimit<RawDecryptor> as Service<T>>::poll_ready(&mut self.0, cx)
    }

    fn call(&mut self, data: T) -> Self::Future {
        self.0.call(data)
    }
}

#[cfg(test)]
mod tests {
    use rayon::ThreadPoolBuilder;
    use tokio_test::assert_ready;
    use tower_test::mock::Spawn;

    use crate::{
        services::tests::utils,
        state_machine::events::{EventPublisher, EventSubscriber},
    };

    use super::*;

    fn spawn_svc() -> (EventPublisher, EventSubscriber, Spawn<Decryptor>) {
        let (publisher, subscriber) = utils::new_event_channels();
        let thread_pool = Arc::new(ThreadPoolBuilder::new().build().unwrap());
        let task = Spawn::new(Decryptor::new(&subscriber, thread_pool));
        (publisher, subscriber, task)
    }

    #[tokio::test]
    async fn test_decrypt_fail() {
        let (_publisher, _subscriber, mut task) = spawn_svc();
        assert_ready!(task.poll_ready::<Vec<u8>>()).unwrap();

        let req = vec![0, 1, 2, 3, 4, 5, 6];
        match task.call(req).await {
            Err(ServiceError::Decrypt) => {}
            _ => panic!("expected decrypt error"),
        }
        assert_ready!(task.poll_ready::<Vec<u8>>()).unwrap();
    }

    #[tokio::test]
    async fn test_decrypt_ok() {
        let (_publisher, subscriber, mut task) = spawn_svc();
        assert_ready!(task.poll_ready::<Vec<u8>>()).unwrap();

        let round_params = subscriber.params_listener().get_latest().event;
        let (message, participant_signing_keys) = utils::new_sum_message(&round_params);
        let serialized_message = utils::serialize_message(&message, &participant_signing_keys);
        let encrypted_message =
            utils::encrypt_message(&message, &round_params, &participant_signing_keys);

        // Call the service
        let decrypted_message = task.call(encrypted_message).await.unwrap();
        assert_eq!(decrypted_message, serialized_message);
    }
}
