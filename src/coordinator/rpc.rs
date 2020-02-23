use crate::common::ClientId;
use futures::{
    future::{self, Ready, TryFutureExt},
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
use tokio::{net::ToSocketAddrs, sync::mpsc};
use tokio_serde::formats::Json;

use tarpc::{
    client::Config,
    rpc::server::{BaseChannel, Channel},
    serde_transport::{tcp::listen, Transport},
};

mod inner {
    use crate::common::ClientId;
    #[tarpc::service]
    pub trait Rpc {
        async fn end_training(id: ClientId, success: bool);
    }
}

pub use inner::{Rpc, RpcClient as Client};

// NOTE: the server is cloned on every request, so cloning should
// remain cheap!
#[derive(Clone)]
pub struct Server {
    end_training: mpsc::UnboundedSender<EndTrainingRequest>,
}

impl Rpc for Server {
    type EndTrainingFut = Ready<()>;

    fn end_training(
        self,
        _: tarpc::context::Context,
        id: ClientId,
        success: bool,
    ) -> Self::EndTrainingFut {
        if self.end_training.send((id, success)).is_err() {
            error!("failed to forward RPC request to AggregatorService: broken channel");
        };
        future::ready(())
    }
}

impl Server {
    fn new() -> (Self, RequestStream) {
        let (end_training_tx, end_training_rx) = mpsc::unbounded_channel::<EndTrainingRequest>();
        let server = Server {
            end_training: end_training_tx,
        };

        let handle = RequestStream::new(end_training_rx);

        (server, handle)
    }
}

/// An incoming [`Rpc::end_training`] RPC request
pub type EndTrainingRequest = (ClientId, bool);

/// A stream of RPC requests from a single client.
pub struct RequestStream(Pin<Box<dyn Stream<Item = EndTrainingRequest> + Send>>);

impl RequestStream {
    fn new(end_training: mpsc::UnboundedReceiver<EndTrainingRequest>) -> Self {
        Self(Box::pin(end_training))
    }
}

impl Stream for RequestStream {
    type Item = EndTrainingRequest;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.0.as_mut().poll_next(cx)
    }
}

/// A handle to receive the RPC requests made to the coordinator by
/// the aggregator.
pub struct RequestReceiver {
    /// A channel that receives RPC requests from the aggregator.
    requests: Option<RequestStream>,

    /// A channel that yields a new `RequestStream` when the aggregator
    /// opens a new connection to the coordinator RPC server.
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
    type Item = EndTrainingRequest;

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
