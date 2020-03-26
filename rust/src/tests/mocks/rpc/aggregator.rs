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

#[derive(Clone)]
pub struct Client(pub Arc<Mutex<MockClient>>);

impl Client {
    pub fn new<T: Transport<(), ()> + 'static>(_: Config, _: T) -> MockNewClient {
        MockNewClient::default()
    }

    pub fn select(
        &mut self,
        ctx: Context,
        credentials: Credentials,
    ) -> future::Ready<io::Result<Result<(), ()>>> {
        self.0.lock().unwrap().select(ctx, credentials)
    }

    pub fn aggregate(&mut self, ctx: Context) -> future::Ready<io::Result<Result<(), ()>>> {
        self.0.lock().unwrap().aggregate(ctx)
    }

    pub fn mock(&self) -> MutexGuard<MockClient> {
        self.0.lock().unwrap()
    }
}

impl From<MockClient> for Client {
    fn from(mock: MockClient) -> Self {
        Self(Arc::new(Mutex::new(mock)))
    }
}
