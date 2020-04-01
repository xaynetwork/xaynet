use crate::{
    aggregator::service::{Aggregator, ServiceHandle, ServiceHandleError},
    common::client::Credentials,
};
use futures::future::{self, TryFutureExt};
use std::{
    error::Error,
    fmt::{Debug, Display},
    future::Future,
    io, iter,
    pin::Pin,
    time::Duration,
};
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tarpc::{
    client::Config,
    context::Context,
    rpc::server::{BaseChannel, Channel},
    serde_transport::{tcp::listen, Transport},
};
use thiserror::Error;
use tokio::{net::ToSocketAddrs, stream::StreamExt};
use tokio_serde::formats::Json;
use tracing_futures::Instrument;

// NOTE: we do not want to return service::AggregateError because:
//
// - it contains some information that is irrelevant for the client,
//   which would just confuse users
// - we don't want to have to implement Serialize and Deserialize on
//   these types

/// Error returned by the RPC server.
#[derive(Error, Serialize, Deserialize, Debug)]
pub enum ServerError<E>
where
    E: Display + Debug,
{
    /// Returned when the aggregator failed to process a request correctly.
    #[error("failed to process RPC call `{0}`: unknown internal error")]
    Internal(&'static str),

    /// Returned when the aggregator processed a request correctly,
    /// but the response to that request is an error.
    #[error("RPC call `{0}` resulted in an error: {1}")]
    Request(&'static str, E),
}

impl<E> ServerError<E>
where
    E: Display + Debug,
{
    fn stringify(self) -> ServerError<String> {
        match self {
            ServerError::Internal(method) => ServerError::Internal(method),
            ServerError::Request(method, inner) => {
                ServerError::Request(method, format!("{}", inner))
            }
        }
    }
}

impl<E> From<(&'static str, ServiceHandleError<E>)> for ServerError<E>
where
    E: Error,
{
    fn from(err: (&'static str, ServiceHandleError<E>)) -> Self {
        match err {
            (method, ServiceHandleError::Handle(_)) => Self::Internal(method),
            (method, ServiceHandleError::Request(e)) => Self::Request(method, e),
        }
    }
}

#[derive(Error, Debug)]
pub enum ClientError<E>
where
    E: Display + Debug,
{
    #[error("an error occured in the RPC layer: {0}")]
    Rpc(#[from] io::Error),

    #[error("the aggregator failed to process the request")]
    Internal,

    #[error("request failed: {0}")]
    Request(E),
}

impl<E> From<ServerError<E>> for ClientError<E>
where
    E: Display + Debug,
{
    fn from(e: ServerError<E>) -> Self {
        match e {
            ServerError::Internal(_) => Self::Internal,
            ServerError::Request(_, e) => Self::Request(e),
        }
    }
}

mod inner {
    use super::*;

    // FIXME: ideally we'd like our trait to be generic over the
    // aggregator, so that we could directly return the aggregator's
    // error type:
    //
    // pub trait Rpc<A>
    //     where A: Aggregator
    // {
    //     async fn select(credentials: Credentials) -> Result<(), ServerError<A::Error>>;
    //     async fn aggregate() -> Result<(), ServerError<A::Error>>;
    // }
    //
    // Unfortunately that is currenctly not supported by `tarpc`. See:
    // https://github.com/google/tarpc/issues/257
    //
    // As a result, we convert the aggregator's error to a String
    // (hence `ServerError::stringify`)

    #[tarpc::service]
    /// Definition of the methods exposed by the aggregator RPC service.
    pub trait Rpc {
        /// Notify the aggregator that the given client has been selected
        /// and should use the given token to download the global weights
        /// and upload their local weights.
        async fn select(credentials: Credentials) -> Result<(), ServerError<String>>;

        /// Notify the aggregator that it should clear its pool of client
        /// IDs and tokens. This should be called before starting a new
        /// round.
        async fn aggregate() -> Result<(), ServerError<String>>;
    }

    #[derive(Clone)]
    pub struct Client(RpcClient);

    impl Client {
        pub async fn connect<A: ToSocketAddrs + Unpin + Clone + Send + Sync + 'static>(
            addr: A,
        ) -> io::Result<Self> {
            let reconnect_opts = ReconnectOptions::new()
                .with_exit_if_first_connect_fails(false)
                .with_retries_generator(|| iter::repeat(Duration::from_secs(1)));
            let tcp_stream = StubbornTcpStream::connect_with_options(addr, reconnect_opts).await?;
            let transport = Transport::from((tcp_stream, Json::default()));
            Ok(Self(RpcClient::new(Config::default(), transport).spawn()?))
        }

        pub fn select(
            &mut self,
            ctx: Context,
            credentials: Credentials,
        ) -> impl Future<Output = Result<(), ClientError<String>>> + '_ {
            self.0
                .aggregate(ctx)
                .map_err(ClientError::from)
                .and_then(|res| future::ready(res.map_err(ClientError::from)))
        }

        pub fn aggregate(
            &mut self,
            ctx: Context,
        ) -> impl Future<Output = Result<(), ClientError<String>>> + '_ {
            self.0
                .aggregate(ctx)
                .map_err(ClientError::from)
                .and_then(|res| future::ready(res.map_err(ClientError::from)))
        }
    }
}

pub use inner::{Client, Rpc};

#[cfg(test)]
pub use crate::tests::lib::rpc::aggregator::Client;
#[cfg(not(test))]
pub use inner::RpcClient;

/// A server that serves a single client. A new `Server` is created
/// for each new client.
pub struct Server<A>(ServiceHandle<A>)
where
    A: Aggregator;

impl<A> Clone for Server<A>
where
    A: Aggregator,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<A> Rpc for Server<A>
where
    A: Aggregator + 'static,
{
    type SelectFut = Pin<Box<dyn Future<Output = Result<(), ServerError<String>>> + Send>>;
    type AggregateFut = Pin<Box<dyn Future<Output = Result<(), ServerError<String>>> + Send>>;

    fn select(self, _: tarpc::context::Context, credentials: Credentials) -> Self::SelectFut {
        debug!("handling select request");
        let span = trace_span!("rpc_select_handler", client_id = %credentials.id());
        Box::pin(
            async move {
                self.0
                    .select(credentials)
                    .await
                    .map_err(|e| ServerError::<A::Error>::from(("select", e)).stringify())
            }
            .instrument(span),
        )
    }

    fn aggregate(self, _: tarpc::context::Context) -> Self::AggregateFut {
        debug!("handling aggregate request");
        let span = trace_span!("rpc_aggregate_handler");
        Box::pin(
            async move {
                self.0
                    .aggregate()
                    .await
                    .map_err(|e| ServerError::<A::Error>::from(("aggregate", e)).stringify())
            }
            .instrument(span),
        )
    }
}

/// Run an RPC server that processes only one connection at a time.
pub async fn serve<A, T>(addr: T, service_handle: ServiceHandle<A>) -> ::std::io::Result<()>
where
    A: Aggregator + 'static,
    T: ToSocketAddrs + Send + Sync + 'static,
{
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
