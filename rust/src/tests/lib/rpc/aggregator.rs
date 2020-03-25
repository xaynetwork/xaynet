use crate::common::client::Credentials;
use futures::future;
use mockall::mock;
use std::{
    io,
    sync::{Arc, Mutex, MutexGuard},
};
use tarpc::{client::Config, context::Context, rpc::Transport};

mock! {
    pub NewClient {
        fn spawn(self) -> io::Result<Client>;
    }
}

mock! {
    pub Client {
        fn new<T: Transport<(), ()> + 'static>(config: Config, transport: T) -> MockNewClient;

        fn select(&mut self, ctx: Context, credentials: Credentials) -> future::Ready<io::Result<Result<(), ()>>>;

        fn aggregate(&mut self, ctx: Context) -> future::Ready<io::Result<Result<(), ()>>>;
    }
}

/// A clonable and thread safe mock for `aggregator::rpc::Client`.
///
/// We cannot directly use `MockClient` for two reasons:
///
/// - internally, the coordinator service clones the RPC client, so we
///   would have to set expectations for each clone.
/// - the coordinator service runs in its own task, and `MockClient`
///   is not thread safe, so we cannot control it directly.
///
/// Therefore, we wrap one `MockClient` instance in a
/// `Arc<Mutex<MockClient>>`, such that each clone of `Client` is
/// actually a reference to the same `MockClient` instance and we can
/// have references to it in multiple threads at the same time.
///
/// Note that using a Mutex is sub-optimal because locking blocks all
/// the tasks running in the same thread than the current task, but it
/// is good enough for testing.
#[derive(Clone)]
pub struct Client(pub Arc<Mutex<MockClient>>);

impl Client {
    pub fn new<T: Transport<(), ()> + 'static>(_: Config, _: T) -> MockNewClient {
        MockNewClient::default()
    }

    /// Get the inner `MockClient`'s `select` method.
    pub fn select(
        &mut self,
        ctx: Context,
        credentials: Credentials,
    ) -> future::Ready<io::Result<Result<(), ()>>> {
        self.mock().select(ctx, credentials)
    }

    /// Get the inner `MockClient`'s `aggregate` method.
    pub fn aggregate(&mut self, ctx: Context) -> future::Ready<io::Result<Result<(), ()>>> {
        self.mock().aggregate(ctx)
    }

    /// Get the inner `MockClient`.
    pub fn mock(&self) -> MutexGuard<MockClient> {
        self.0.lock().unwrap()
    }
}

impl From<MockClient> for Client {
    fn from(mock: MockClient) -> Self {
        Self(Arc::new(Mutex::new(mock)))
    }
}
