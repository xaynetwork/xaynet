use futures::{ready, stream::Stream};
use tokio::{
    sync::mpsc,
    time::{delay_for, Delay},
};

use crate::common::ClientId;

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

struct ExpirationNotifier(Option<(ClientId, mpsc::UnboundedSender<ClientId>)>);

impl ExpirationNotifier {
    fn run(&mut self) {
        if let Some((id, channel)) = self.0.take() {
            channel.send(id).unwrap_or_else(|_| {
                warn!("failed to send timer expiration notification: channel is closed")
            });
        } else {
            unreachable!("invalid ExpirationNotifier state");
        }
    }
}

pub struct HeartBeatTimer {
    expiration_notifier: ExpirationNotifier,
    resets: mpsc::Receiver<Duration>,
    timer: Delay,
}

impl HeartBeatTimer {
    pub fn new(
        client_id: ClientId,
        delay: Duration,
        expiration_tx: mpsc::UnboundedSender<ClientId>,
        resets_rx: mpsc::Receiver<Duration>,
    ) -> Self {
        Self {
            expiration_notifier: ExpirationNotifier(Some((client_id, expiration_tx))),
            resets: resets_rx,
            timer: delay_for(delay),
        }
    }

    fn poll_resets(&mut self, cx: &mut Context) -> Poll<()> {
        loop {
            match ready!(Pin::new(&mut self.resets).poll_next(cx)) {
                Some(duration) => {
                    self.timer = delay_for(duration);
                    debug!("heartbeat timer reset");
                }
                None => return Poll::Ready(()),
            }
        }
    }

    fn poll_timer(&mut self, cx: &mut Context) -> Poll<()> {
        ready!(Pin::new(&mut self.timer).poll(cx));
        self.expiration_notifier.run();
        Poll::Ready(())
    }
}

impl Future for HeartBeatTimer {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling heartbeat timer");
        if let Poll::Ready(()) = self.as_mut().poll_resets(cx) {
            trace!("dropping heartbeat timer: reset channel closed");
            return Poll::Ready(());
        }
        if let Poll::Ready(()) = self.as_mut().poll_timer(cx) {
            trace!("heartbeat timer expired");
            return Poll::Ready(());
        }
        Poll::Pending
    }
}
