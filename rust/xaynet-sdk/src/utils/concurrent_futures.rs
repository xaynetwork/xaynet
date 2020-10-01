use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures::{
    stream::{FuturesUnordered, Stream, StreamExt},
    Future,
};
use tokio::{
    task::{JoinError, JoinHandle},
    time::delay_for,
};

/// `ConcurrentFutures` can keep a capped number of futures running concurrently, and yield their
/// result as they finish. When the max number of concurrent futures is reached, new tasks are
/// queued until some in-flight futures finish.
#[pin_project]
pub struct ConcurrentFutures<T>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    /// in-flight futures
    #[pin]
    running: FuturesUnordered<JoinHandle<T::Output>>,
    /// buffered tasks
    pending: VecDeque<T>,
    /// max number of concurrent futures
    max_in_flight: usize,
}

impl<T> ConcurrentFutures<T>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    pub fn new(max_in_flight: usize) -> Self {
        Self {
            running: FuturesUnordered::new(),
            pending: VecDeque::new(),
            max_in_flight,
        }
    }

    pub fn push(&mut self, task: T) {
        self.pending.push_back(task)
    }
}

impl<T> Stream for ConcurrentFutures<T>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    type Item = Result<T::Output, JoinError>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();
        while this.running.len() < *this.max_in_flight {
            if let Some(pending) = this.pending.pop_front() {
                let handle = tokio::spawn(pending);
                this.running.push(handle);
            } else {
                break;
            }
        }
        this.running.poll_next(cx)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test() {
        let mut stream =
            ConcurrentFutures::<Pin<Box<dyn Future<Output = u8> + Send + 'static>>>::new(2);

        stream.push(Box::pin(async {
            delay_for(Duration::from_millis(10_u64)).await;
            1_u8
        }));

        stream.push(Box::pin(async {
            delay_for(Duration::from_millis(25_u64)).await;
            2_u8
        }));

        stream.push(Box::pin(async {
            delay_for(Duration::from_millis(12_u64)).await;
            3_u8
        }));

        stream.push(Box::pin(async {
            delay_for(Duration::from_millis(1_u64)).await;
            4_u8
        }));

        // poll_next hasn't been called yet so nothing is running
        assert_eq!(stream.running.len(), 0);
        assert_eq!(stream.pending.len(), 4);
        assert_eq!(stream.next().await.unwrap().unwrap(), 1);

        // two futures have been spawned, but one of them just finished: one is still running, two are
        // still pending
        assert_eq!(stream.running.len(), 1);
        assert_eq!(stream.pending.len(), 2);
        assert_eq!(stream.next().await.unwrap().unwrap(), 3);

        // three futures have been spawned, two finished: one is still running, one is still pending
        assert_eq!(stream.running.len(), 1);
        assert_eq!(stream.pending.len(), 1);
        assert_eq!(stream.next().await.unwrap().unwrap(), 4);

        // four futures have been spawn, three finished: one is still running
        assert_eq!(stream.next().await.unwrap().unwrap(), 2);
        assert_eq!(stream.running.len(), 0);
        assert_eq!(stream.pending.len(), 0);
    }
}
