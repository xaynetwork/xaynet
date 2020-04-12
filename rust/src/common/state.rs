use derive_more::From;
use futures::{ready, stream::Stream};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    future::Future,
    io::{BufReader, Write},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    stream::StreamExt,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};

#[derive(Serialize, Deserialize)]
pub struct State {
    pub round: u32,
}

// use async_trait::async_trait;
// #[async_trait]
// pub trait StateHandle {
//     async fn read_last(self) -> State;
//     async fn write(self, state: State) -> Result<(), Box<dyn Error + 'static>>;
// }

// #[async_trait]
// impl StateHandle for CoordinatorStateHandle {
//     async fn read_last(self) -> State {
//         let contents = read(self.file_path).await.expect("");
//         serde_json::from_slice(&contents)?;
//     }

//     async fn write(self, state: State) -> Result<(), Box<dyn Error + 'static>> {
//         let mut file = File::create(self.file_path).await?;

//         let to_json = serde_json::to_vec(&state)?;
//         file.write_all(&to_json).await?;

//         Ok(())
//     }
// }

pub struct StateService {
    file_path: String,
    state_requests: StateRequests,
}

impl StateService {
    pub fn new<S: Into<String>>(file_path: S, state_requests: StateRequests) -> Self {
        Self {
            file_path: file_path.into(),
            state_requests,
        }
    }

    /// Handle the incoming requests.
    fn poll_requests(&mut self, cx: &mut Context) -> Poll<()> {
        trace!("polling requests");
        loop {
            match ready!(Pin::new(&mut self.state_requests).poll_next(cx)) {
                Some(request) => {
                    self.handle_request(request);
                }
                None => return Poll::Ready(()),
            }
        }
    }

    /// Handle a request
    fn handle_request(&mut self, request: Request) {
        match request {
            Request::Write(req) => self.handle_write_request(req),
            Request::ReadLast(req) => self.handle_read_last_request(req),
        }
    }

    /// Handle a write request
    fn handle_write_request(&mut self, req: WriteRequest) {
        // can this be async and spawned in the tokio?

        debug!("handling write request");
        let WriteRequest { state } = req;

        let to_json = match serde_json::to_vec(&state) {
            Ok(to_json) => to_json,
            Err(err) => {
                warn!("could not convert state {}", err);
                return;
            }
        };

        let mut file = match File::create(&self.file_path) {
            Ok(file) => file,
            Err(err) => {
                warn!("could not create state file {}", err);
                return;
            }
        };

        let _ = file.write_all(&to_json).map_err(|err| {
            warn!("could not write state {}", err);
        });
    }

    /// Handle a read last request
    fn handle_read_last_request(&mut self, req: ReadLastRequest) {
        debug!("handling read last request");
        let ReadLastRequest { response_tx } = req;

        let file = File::open(&self.file_path).expect("");
        let reader = BufReader::new(file);
        let response = serde_json::from_reader(reader).expect("msg");

        if response_tx.send(response).is_err() {
            warn!("failed to send response back: channel closed");
        }
    }
}

impl Future for StateService {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling Service");
        let pin = self.get_mut();

        match pin.poll_requests(cx) {
            Poll::Ready(()) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug)]
pub struct RequestError;

#[derive(From)]
pub enum Request {
    Write(WriteRequest),
    ReadLast(ReadLastRequest),
}

#[derive(From)]
pub struct WriteRequest {
    state: State,
}

#[derive(From)]
pub struct ReadLastRequest {
    response_tx: oneshot::Sender<State>,
}

pub struct StateRequests(Pin<Box<dyn Stream<Item = Request> + Send>>);

impl Stream for StateRequests {
    type Item = Request;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("polling StateRequest");
        self.0.as_mut().poll_next(cx)
    }
}

impl StateRequests {
    fn new(
        write: UnboundedReceiver<WriteRequest>,
        read_last: UnboundedReceiver<ReadLastRequest>,
    ) -> Self {
        let stream = write.map(Request::from).merge(read_last.map(Request::from));
        Self(Box::pin(stream))
    }
}

#[derive(Clone)]
pub struct StateHandle {
    write: UnboundedSender<WriteRequest>,
    read_last: UnboundedSender<ReadLastRequest>,
}

impl StateHandle {
    pub fn new() -> (Self, StateRequests) {
        let (write_tx, write_rx) = unbounded_channel::<WriteRequest>();
        let (read_last_tx, read_last_rx) = unbounded_channel::<ReadLastRequest>();

        let handle = Self {
            write: write_tx,
            read_last: read_last_tx,
        };
        let state_request = StateRequests::new(write_rx, read_last_rx);
        (handle, state_request)
    }

    // Not sure if this can be async
    pub fn write(&self, state: State) {
        Self::send_request(WriteRequest::from(state), &self.write);
    }

    pub async fn read_last(&self) -> Result<State, RequestError> {
        let (tx, rx) = oneshot::channel();
        Self::send_request(ReadLastRequest::from(tx), &self.read_last);
        rx.await.map_err(|_| {
            warn!("could not receive response: channel closed");
            RequestError
        })
    }

    fn send_request<P>(payload: P, chan: &UnboundedSender<P>) {
        trace!("send request to the state service");
        if chan.send(payload).is_err() {
            warn!("failed to send request: channel closed");
            return;
        }
        trace!("request sent");
    }
}
