use crate::common::{ClientId, Token};
use derive_more::Display;
use futures::{future::TryFutureExt, ready, stream::Stream};
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
    stream::StreamExt,
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
        async fn aggregate() -> Result<(), ()>;
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
    aggregate: mpsc::UnboundedSender<AggregateRequest>,
}

impl Server {
    fn new() -> (Self, RequestStream) {
        let (select_tx, select_rx) = mpsc::unbounded_channel::<SelectRequest>();
        let (aggregate_tx, aggregate_rx) = mpsc::unbounded_channel::<AggregateRequest>();

        let server = Server {
            select: select_tx,
            aggregate: aggregate_tx,
        };

        let handle = RequestStream::new(select_rx, aggregate_rx);

        (server, handle)
    }
}

/// An incoming [`AggregatorRpc::select`] RPC request
#[derive(Display)]
#[display(fmt = "Select({})", id)]
pub struct SelectRequest {
    pub id: ClientId,
    pub token: Token,
    pub response_tx: oneshot::Sender<()>,
}

/// An incoming [`AggregatorRpc::aggregate`] RPC request
#[derive(Display)]
#[display(fmt = "Aggregate")]
pub struct AggregateRequest {
    pub response_tx: oneshot::Sender<()>,
}

/// An incoming RPC request
#[derive(Display)]
pub enum Request {
    /// An incoming [`AggregatorRpc::select`] RPC request
    #[display(fmt = "{}", _0)]
    Select(SelectRequest),
    /// An incoming [`AggregatorRpc::aggregate`] RPC request
    #[display(fmt = "{}", _0)]
    Aggregate(AggregateRequest),
}

/// Stream of requests made to an RPC server instance. A `Server` is
/// spawned for each client that connects, so a distinct
/// `RequestStream` is created for each client.
pub struct RequestStream(Pin<Box<dyn Stream<Item = Request> + Send>>);

impl RequestStream {
    fn new(
        select: mpsc::UnboundedReceiver<SelectRequest>,
        aggregate: mpsc::UnboundedReceiver<AggregateRequest>,
    ) -> Self {
        Self(Box::pin(
            aggregate
                .map(Request::Aggregate)
                .merge(select.map(Request::Select)),
        ))
    }
}

impl Stream for RequestStream {
    type Item = Request;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("polling RequestStream");
        self.0.as_mut().poll_next(cx)
    }
}

impl Rpc for Server {
    type SelectFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;
    type AggregateFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

    fn select(self, _: tarpc::context::Context, id: ClientId, token: Token) -> Self::SelectFut {
        debug!("received select request");
        let (response_tx, response_rx) = oneshot::channel();
        Box::pin(async move {
            self.select
                .send(SelectRequest {
                    id,
                    token,
                    response_tx,
                })
                .map_err(|_| ())?;
            response_rx.map_err(|_| ()).await
        })
    }

    fn aggregate(self, _: tarpc::context::Context) -> Self::AggregateFut {
        debug!("received aggregate request");
        let (response_tx, response_rx) = oneshot::channel();
        Box::pin(async move {
            self.aggregate
                .send(AggregateRequest { response_tx })
                .map_err(|_| ())?;
            response_rx.map_err(|_| ()).await
        })
    }
}

/// RPC requests are received via [`RequestStream`] streams, but a
/// distinct [`RequestStream`] is created for each client that
/// connects. `RpcRequestsMux` multiplexes multiple
/// `RequestStreams`: it consumes each `RequestStream` created by the
/// RPC server task, sequentially.
pub struct RpcRequestsMux {
    requests: Option<RequestStream>,
    streams: mpsc::Receiver<RequestStream>,
}

impl RpcRequestsMux {
    /// Create a new `RpcRequestMux` that will process the
    /// `RequestStream`s produced by the given receiver.
    pub fn new(streams: mpsc::Receiver<RequestStream>) -> Self {
        Self {
            requests: None,
            streams,
        }
    }
}

impl Stream for RpcRequestsMux {
    type Item = Request;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("polling RpcRequestsMux");

        let Self {
            ref mut requests,
            ref mut streams,
        } = self.get_mut();

        // If we have a requests channel poll it
        if let Some(stream) = requests {
            if let Some(item) = ready!(Pin::new(stream).poll_next(cx)) {
                trace!("RequestStream: received new request");
                return Poll::Ready(Some(item));
            } else {
                debug!("RequestStream closed");
                *requests = None;
            }
        }

        trace!("no RequestStream, polling the RequestStream receiver");
        let mut pin = Pin::new(streams);

        loop {
            if let Some(mut stream) = ready!(pin.as_mut().poll_next(cx)) {
                trace!("received new RequeStream, polling it");
                match Pin::new(&mut stream).poll_next(cx) {
                    Poll::Ready(Some(item)) => {
                        trace!("RequestStream: received new request");
                        *requests = Some(stream);
                        return Poll::Ready(Some(item));
                    }
                    Poll::Ready(None) => {
                        // This is suspect, let's log a warning here
                        warn!("RequestStream: closed already ???");
                    }
                    Poll::Pending => {
                        // This should be the most common case
                        trace!("RequestStream: no request yet");
                        *requests = Some(stream);
                        // Note that it is important not to return
                        // here. We MUST poll the `streams` future
                        // until it returns Pending, if we want the
                        // executor to wakes the task up later!
                    }
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

/// Run an RPC server that processes only one connection at a time.
pub async fn serve<A: ToSocketAddrs + Send + Sync + 'static>(
    addr: A,
    mut request_stream_tx: mpsc::Sender<RequestStream>,
) -> ::std::io::Result<()> {
    let mut listener = listen(addr, Json::default).await?;

    while let Some(accept_result) = listener.next().await {
        match accept_result {
            Ok(transport) => {
                let channel = BaseChannel::with_defaults(transport);
                let (server, handle) = Server::new();
                if request_stream_tx.send(handle).await.is_err() {
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
