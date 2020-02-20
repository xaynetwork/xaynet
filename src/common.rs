use derive_more::Display;
use uuid::Uuid;

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone, Display, Serialize, Deserialize, Default)]
/// A unique random client identifier
pub struct ClientId(Uuid);

impl ClientId {
    /// Return a new random client identifier
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone, Display, Serialize, Deserialize, Default)]
/// A unique random token
pub struct Token(Uuid);

impl Token {
    /// Return a new random token
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

// use tokio::sync::{mpsc, oneshot};
// use std::{
//     future::Future,
//     pin::Pin,
//     task::{Context, Poll},
// };

// struct BrokenChannel;

// pub struct RequestRx<T, U>(mpsc::UnboundedReceiver<(T, ResponseTx<U>)>);
// pub struct RequestTx<T, U>(mpsc::UnboundedSender<(T, ResponseTx<U>)>);

// impl<T, U> RequestTx<T, U> {
//     fn send(&mut self, request: T) -> Result<ResponseRx<U>, BrokenChannel> {
//         let (resp_tx, resp_rx) = response_channel();
//         self.0.send((request, resp_tx)).map_err(|_| BrokenChannel)
//     }
// }

// pub struct ResponseRx<U>(oneshot::Receiver<U>);
// pub struct ResponseTx<U>(oneshot::Sender<U>);

// pub fn response_channel<U>() -> (ResponseTx<U>, ResponseRx<U>) {
//     let (tx, rx) = oneshot::channel::<U>();
//     (ResponseTx(tx), ResponseRx(rx))
// }

// pub fn request_channel<T, U>() -> (RequestTx<T, U>, RequestRx<T, U>) {
//     let (tx, rx) = mpsc::unbounded_channel::<(T, ResponseTx<U>)>();
//     (RequestTx(tx), RequestRx(rx))
// }

// impl<U> Future for ResponseRx<U> {
//     type Output = Result<U, BrokenChannel>;
//     fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
//         Pin::new(&mut self.get_mut().0)
//             .as_mut()
//             .poll(cx)
//             .map_err(|_| BrokenChannel)
//     }
// }

// impl<U> ResponseTx<U> {
//     pub fn send(self, response: U) -> Result<U, BrokenChannel> {
//         self.0.send(response).map(|_| BrokenChannel)
//     }
// }
