use crate::common::{ClientId, Token};
use futures::{
    future::TryFutureExt,
    ready,
    stream::{Stream, StreamExt},
};
use futures_retry::{FutureRetry, RetryPolicy};
use std::{
    future::Future,
    io, iter,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tarpc::{
    client::Config,
    rpc::server::{BaseChannel, Channel},
    serde_transport::{tcp::listen, Transport},
};

use tokio::{
    net::ToSocketAddrs,
    sync::{mpsc, oneshot},
};
use tokio_serde::formats::Json;

mod inner {
    use crate::common::{ClientId, Token};

    #[tarpc::service]
    /// Definition of the methods exposed by the aggregator RPC service.
    pub trait Rpc {
        /// Notify the aggregator that the given client has been selected
        /// and should use the given token to download the global weights
        /// and upload their local weights.
        async fn select(id: ClientId, token: Token) -> Result<(), ()>;

        /// Notify the aggregator that it should clear its pool of client
        /// IDs and tokens. This should be called before starting a new
        /// round.
        async fn reset() -> Result<(), ()>;
    }
}

pub use inner::{Rpc, RpcClient as Client};

// NOTE: the server is cloned on every request, so cloning should
// remain cheap!
#[derive(Clone)]
/// A server that serves a single client. A new `Server` is created
/// for each new client.
struct Server {
    select: mpsc::UnboundedSender<SelectRequest>,
    reset: mpsc::UnboundedSender<ResetRequest>,
}

impl Server {
    fn new() -> (Self, RequestStream) {
        let (select_tx, select_rx) = mpsc::unbounded_channel::<SelectRequest>();
        let (reset_tx, reset_rx) = mpsc::unbounded_channel::<ResetRequest>();

        let server = Server {
            select: select_tx,
            reset: reset_tx,
        };

        let handle = RequestStream::new(select_rx, reset_rx);

        (server, handle)
    }
}

/// An incoming [`AggregatorRpc::select`] RPC request
pub type SelectRequest = ((ClientId, Token), oneshot::Sender<()>);
/// An incoming [`AggregatorRpc::reset`] RPC request
pub type ResetRequest = oneshot::Sender<()>;

/// An incoming RPC request
pub enum Request {
    /// An incoming [`AggregatorRpc::select`] RPC request
    Select(SelectRequest),
    /// An incoming [`AggregatorRpc::reset`] RPC request
    Reset(ResetRequest),
}

/// A handle to receive the RPC requests received by the RPC
/// [`AggregatorRpc`].
pub struct RequestStream(Pin<Box<dyn Stream<Item = Request> + Send>>);

impl RequestStream {
    fn new(
        select: mpsc::UnboundedReceiver<SelectRequest>,
        reset: mpsc::UnboundedReceiver<ResetRequest>,
    ) -> Self {
        Self(Box::pin(
            reset.map(Request::Reset).chain(select.map(Request::Select)),
        ))
    }
}

impl Stream for RequestStream {
    type Item = Request;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.0.as_mut().poll_next(cx)
    }
}

impl Rpc for Server {
    type SelectFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;
    type ResetFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

    fn select(self, _: tarpc::context::Context, id: ClientId, token: Token) -> Self::SelectFut {
        let (tx, rx) = oneshot::channel();
        Box::pin(async move {
            self.select.send(((id, token), tx)).map_err(|_| ())?;
            rx.map_err(|_| ()).await
        })
    }

    fn reset(self, _: tarpc::context::Context) -> Self::ResetFut {
        let (tx, rx) = oneshot::channel();
        Box::pin(async move {
            self.reset.send(tx).map_err(|_| ())?;
            rx.map_err(|_| ()).await
        })
    }
}

pub struct RequestReceiver {
    requests: Option<RequestStream>,
    connections: mpsc::Receiver<RequestStream>,
}

impl RequestReceiver {
    fn new(connections: mpsc::Receiver<RequestStream>) -> Self {
        Self {
            requests: None,
            connections,
        }
    }
}

impl Stream for RequestReceiver {
    type Item = Request;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let Self {
            ref mut requests,
            ref mut connections,
        } = self.get_mut();

        // If we have a requests channel poll it
        if let Some(stream) = requests {
            if let Some(item) = ready!(Pin::new(stream).poll_next(cx)) {
                return Poll::Ready(Some(item));
            } else {
                *requests = None;
            }
        }

        // We don't have a requests channel, so poll the connections
        // channel to get a new one.
        let mut pin = Pin::new(connections);
        loop {
            if let Some(mut stream) = ready!(pin.as_mut().poll_next(cx)) {
                if let Some(item) = ready!(Pin::new(&mut stream).poll_next(cx)) {
                    *requests = Some(stream);
                    return Poll::Ready(Some(item));
                }
            } else {
                return Poll::Ready(None);
            }
        }
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

pub struct ConnectFuture(Pin<Box<dyn Future<Output = io::Result<Client>> + Send>>);

impl ConnectFuture {
    pub fn new<A: ToSocketAddrs + Clone + Unpin + Send + Sync + 'static>(addr: A) -> Self {
        Self(Box::pin(client_connect(addr)))
    }
}

impl Future for ConnectFuture {
    type Output = io::Result<Client>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling ConnectFuture");
        Pin::new(&mut self.get_mut().0).poll(cx)
    }
}

/// Spawn an RPC server and return a stream of `RequestStream`. A new
/// `RequestStream` is yielded for each new connection.
pub fn run<A: ToSocketAddrs + Send + Sync + 'static>(addr: A) -> RequestReceiver {
    let (tx, rx) = mpsc::channel(1);
    tokio::spawn(_run(addr, tx).map_err(|e| error!("RPC worker finished with an error: {}", e)));
    RequestReceiver::new(rx)
}

/// Run an RPC server that accepts only one connection at a time.
async fn _run<A: ToSocketAddrs + Send + Sync + 'static>(
    addr: A,
    mut rpc_handle_tx: mpsc::Sender<RequestStream>,
) -> ::std::io::Result<()> {
    let mut listener = listen(addr, Json::default).await?;

    while let Some(accept_result) = listener.next().await {
        match accept_result {
            Ok(transport) => {
                let channel = BaseChannel::with_defaults(transport);
                let (server, handle) = Server::new();
                if rpc_handle_tx.send(handle).await.is_err() {
                    continue;
                }
                let handler = channel.respond_with(server.serve());
                handler.execute().await;
            }
            Err(e) => error!("failed to accept RPC connection: {:?}", e),
        }
    }
    Ok(())
}
