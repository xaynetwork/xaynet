use crate::{aggregator::service::ServiceHandle, common::client::Credentials};
use std::{future::Future, io, iter, pin::Pin, time::Duration};
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tarpc::{
    client::Config,
    rpc::server::{BaseChannel, Channel},
    serde_transport::{tcp::listen, Transport},
};
use tokio::{net::ToSocketAddrs, stream::StreamExt};
use tokio_serde::formats::Json;
use tracing_futures::Instrument;

mod inner {
    use super::Credentials;

    #[tarpc::service]
    /// Definition of the methods exposed by the aggregator RPC service.
    pub trait Rpc {
        /// Notify the aggregator that the given client has been selected
        /// and should use the given token to download the global weights
        /// and upload their local weights.
        async fn select(credentials: Credentials) -> Result<(), ()>;

        /// Notify the aggregator that it should clear its pool of client
        /// IDs and tokens. This should be called before starting a new
        /// round.
        async fn aggregate() -> Result<(), ()>;
    }
}

pub use inner::Rpc;

#[cfg(test)]
pub use crate::tests::mocks::rpc::aggregator::Client;
#[cfg(not(test))]
pub use inner::RpcClient as Client;

/// A server that serves a single client. A new `Server` is created
/// for each new client.
#[derive(Clone)]
struct Server(ServiceHandle);

impl Rpc for Server {
    type SelectFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;
    type AggregateFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

    fn select(self, _: tarpc::context::Context, credentials: Credentials) -> Self::SelectFut {
        debug!("handling select request");
        let span = trace_span!("rpc_select_handler", client_id = %credentials.id());
        Box::pin(async move { self.0.select(credentials).await }.instrument(span))
    }

    fn aggregate(self, _: tarpc::context::Context) -> Self::AggregateFut {
        debug!("handling aggregate request");
        let span = trace_span!("rpc_aggregate_handler");
        Box::pin(async move { self.0.aggregate().await }.instrument(span))
    }
}

/// A future that keeps trying to connect to the `AggregatorRpc` at the
/// given address.
pub async fn client_connect<A: ToSocketAddrs + Unpin + Clone + Send + Sync + 'static>(
    addr: A,
) -> io::Result<Client> {
    let reconnect_opts = ReconnectOptions::new()
        .with_exit_if_first_connect_fails(false)
        .with_retries_generator(|| iter::repeat(Duration::from_secs(1)));
    let tcp_stream = StubbornTcpStream::connect_with_options(addr, reconnect_opts).await?;
    let transport = Transport::from((tcp_stream, Json::default()));
    Client::new(Config::default(), transport).spawn()
}

/// Run an RPC server that processes only one connection at a time.
pub async fn serve<A: ToSocketAddrs + Send + Sync + 'static>(
    addr: A,
    service_handle: ServiceHandle,
) -> ::std::io::Result<()> {
    let mut listener = listen(addr, Json::default).await?;

    while let Some(accept_result) = listener.next().await {
        match accept_result {
            Ok(transport) => {
                let channel = BaseChannel::with_defaults(transport);
                let server = Server(service_handle.clone());
                let handler = channel.respond_with(server.serve());
                handler
                    .execute()
                    // FIXME: add peer to span
                    .instrument(trace_span!("rpc_handler"))
                    .await;
            }
            Err(e) => error!("failed to accept RPC connection: {:?}", e),
        }
    }
    Ok(())
}
