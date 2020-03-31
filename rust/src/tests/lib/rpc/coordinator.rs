use crate::common::client::ClientId;
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

        fn end_training(&mut self, ctx: Context, id: ClientId, success: bool) -> future::Ready<io::Result<()>>;
    }
}

#[derive(Clone)]
pub struct Client(pub Arc<Mutex<MockClient>>);

impl Client {
    pub fn new<T: Transport<(), ()> + 'static>(_: Config, _: T) -> MockNewClient {
        MockNewClient::default()
    }

    pub fn end_training(
        &mut self,
        ctx: Context,
        id: ClientId,
        success: bool,
    ) -> future::Ready<io::Result<()>> {
        self.mock().end_training(ctx, id, success)
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
