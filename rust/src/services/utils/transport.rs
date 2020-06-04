use std::{fmt::Debug, pin::Pin, task::Poll};

use futures::{sink::Sink, stream::Stream, task::Context};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::services::utils::trace::Traced;

#[derive(Debug, Clone, Error)]
pub enum TransportError {
    #[error("sink failed: peer channel disconnected")]
    Send,
}

pub type TransportClient<Req, Resp> = Transport<Resp, Req>;
pub type TransportServer<Req, Resp> = Transport<Req, Resp>;

/// A transport layer backed by channels. It can be used for
/// communication between two services
pub struct Transport<Rx, Tx> {
    rx: mpsc::UnboundedReceiver<Rx>,
    tx: mpsc::UnboundedSender<Tx>,
}

pub fn transport<Req, Resp>() -> (TransportClient<Req, Resp>, TransportServer<Req, Resp>) {
    let (tx1, rx1) = mpsc::unbounded_channel::<Req>();
    let (tx2, rx2) = mpsc::unbounded_channel::<Resp>();

    let chan1: TransportClient<Req, Resp> = Transport { rx: rx2, tx: tx1 };
    let chan2: TransportServer<Req, Resp> = Transport { rx: rx1, tx: tx2 };
    (chan1, chan2)
}

pub fn traceable_transport<Req, Resp>() -> (
    TransportClient<Traced<Req>, Resp>,
    TransportServer<Traced<Req>, Resp>,
) {
    transport::<Traced<Req>, Resp>()
}

impl<Rx, Tx> Sink<Tx> for Transport<Rx, Tx>
where
    Rx: Debug,
    Tx: Debug,
{
    type Error = TransportError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        // unbounded channels are always ready to send
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Tx) -> Result<(), Self::Error> {
        self.tx.send(item).map_err(|_| TransportError::Send)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(())) // no-op because all sends succeed immediately
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(())) // no-op because channel is closed on drop and flush is no-op
    }
}

impl<Rx, Tx> Stream for Transport<Rx, Tx>
where
    Rx: Debug,
    Tx: Debug,
{
    // With channels, the stream cannot fail
    type Item = Result<Rx, ::std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx).map(|s| s.map(Ok))
    }
}
