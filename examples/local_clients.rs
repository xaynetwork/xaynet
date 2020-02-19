#![feature(or_patterns)]

#[macro_use]
extern crate log;

use std::{
    clone::Clone,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use derive_more::Display;
use futures::{future::FutureExt, ready, stream::Stream};
use rand::{seq::IteratorRandom, Rng};
use tokio::{sync::mpsc, time::delay_for};

use xain_fl::{
    common::ClientId,
    coordinator::{
        Aggregator, CoordinatorConfig, CoordinatorHandle, CoordinatorService, HeartBeatResponse,
        RendezVousResponse, RequestError, Selector, StartTrainingResponse,
    },
};

/// Represent the state of a client
#[derive(Display)]
pub enum ClientState<T> {
    /// The client is waiting to be selected
    Waiting,

    #[display(fmt = "StartTraining")]
    StartTraining(Pin<Box<dyn Future<Output = T> + Send>>),

    /// The client is currently training
    #[display(fmt = "Training")]
    Training(Pin<Box<dyn Future<Output = T> + Send>>),

    /// The client finished training and is waiting for its "end
    /// training" request to be handled
    #[display(fmt = "EntTraining")]
    EndTraining(Pin<Box<dyn Future<Output = ()> + Send>>),
}

#[derive(Display)]
#[display(fmt = "Client(id={}, state={})", id, state)]
pub struct Client<T> {
    handle: CoordinatorHandle<T>,
    state: ClientState<T>,
    id: ClientId,
    heartbeat: mpsc::Receiver<HeartBeatResponse>,
    train_function: Box<dyn Fn(T) -> Pin<Box<dyn Future<Output = T> + Send>> + Send>,
}

pub struct HeartBeat<T> {
    id: ClientId,
    tx: mpsc::Sender<HeartBeatResponse>,
    handle: CoordinatorHandle<T>,
}

impl<T> HeartBeat<T> {
    pub async fn start(mut self) {
        async {
            loop {
                match self.handle.heartbeat(self.id).await {
                    Err(_) => {
                        error!("heartbeat channel closed: coordinator stopped");
                        return;
                    }
                    Ok(HeartBeatResponse::Finish) | Ok(HeartBeatResponse::Reject) => {
                        if self.tx.send(HeartBeatResponse::Finish).await.is_err() {
                            error!("heartbeat channel closed: client dropped");
                        }
                        break;
                    }
                    Ok(response) => {
                        if self.tx.send(response).await.is_err() {
                            error!("heartbeat channel closed: client dropped");
                        }
                        delay_for(Duration::from_millis(1000)).await;
                    }
                }
            }
        }
        .await
    }
}

impl<T> Client<T>
where
    T: 'static + Send,
{
    /// Send a rendez-vous request and if it is accepted return a new
    /// client.
    pub async fn new(
        mut handle: CoordinatorHandle<T>,
        train_function: Box<dyn Fn(T) -> Pin<Box<dyn Future<Output = T> + Send>> + Send>,
    ) -> Result<(Self, HeartBeat<T>), ()> {
        match handle.rendez_vous().await {
            Ok(RendezVousResponse::Accept(id)) => {
                info!("rendez-vous request accepted by the coordinator");
                info!("got client ID {}", id);
                let (tx, rx) = mpsc::channel(10);
                let client = Self {
                    handle: handle.clone(),
                    state: ClientState::Waiting,
                    id,
                    heartbeat: rx,
                    train_function,
                };
                let heartbeat = HeartBeat { handle, id, tx };
                Ok((client, heartbeat))
            }
            Ok(RendezVousResponse::Reject) => {
                error!("rendez-vous rejected by the coordinator");
                Err(())
            }
            Err(_) => {
                error!("failed to send rendez-vous request to the coordinator");
                Err(())
            }
        }
    }

    fn poll_heartbeats(&mut self, cx: &mut Context) -> Poll<()> {
        debug!("polling hearbeat responses");
        loop {
            match ready!(Pin::new(&mut self.heartbeat).poll_next(cx)) {
                Some(response) => {
                    use HeartBeatResponse::*;

                    match (&response, &self.state) {
                        (Finish | Reject, _) => {
                            warn!("heartbeat response {:?}: stopping client", response);
                            return Poll::Ready(());
                        }
                        (Round(_), ClientState::Waiting) => {
                            info!(
                                "heartbeat response {:?}: client {} got selected",
                                response, self.id
                            );
                            self.start_training();
                        }
                        _ => {
                            trace!("ignoring heartbeat response {:?}", response);
                        }
                    }
                }
                None => return Poll::Ready(()),
            }
        }
    }

    fn start_training(&mut self) {
        let mut handle = self.handle.clone();
        let id = self.id;
        self.state = ClientState::StartTraining(Box::pin(async move {
            handle
                .start_training(id)
                .map(|resp| match resp {
                    Ok(StartTrainingResponse::Accept(payload)) => payload.global_weights,
                    Ok(StartTrainingResponse::Reject) => {
                        // FIXME: out client is pretty basic and
                        // doesn't handle this for now.
                        panic!("start training response rejected");
                    }
                    Err(RequestError) => {
                        panic!("start training request failed: coordinator dropped")
                    }
                })
                .await
        }));
    }
}

impl<T> Future for Client<T>
where
    T: 'static + Send,
{
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pin = self.get_mut();
        match pin.poll_heartbeats(cx) {
            Poll::Ready(()) => return Poll::Ready(()),
            Poll::Pending => {}
        }
        debug!("polling client {}", pin);
        match &mut pin.state {
            ClientState::StartTraining(f) => {
                if let Poll::Ready(weights) = f.as_mut().poll(cx) {
                    debug!("client {} entering Training state", pin.id);
                    pin.state = ClientState::Training(Box::pin((pin.train_function)(weights)));
                } else {
                    debug!("client {} still in StartTraining state", pin.id);
                }
            }
            ClientState::Training(f) => {
                if let Poll::Ready(result) = f.as_mut().poll(cx) {
                    info!(
                        "client {} done training, sending the results to the coordinator",
                        pin.id
                    );
                    let handle = pin.handle.clone();
                    let id = pin.id;
                    debug!("client {} entering EndTraining state", pin.id);
                    pin.state =
                        ClientState::EndTraining(Box::pin(async move {
                            handle.clone().end_training(id, result).map(|res| {
                            if res.is_err() {
                                error!("could not send end training request: coordinator stopped");
                            } else {
                                trace!("received end training response")
                            }
                        }).await
                        }));
                } else {
                    debug!("client {} still in Training state", pin.id);
                }
            }
            ClientState::EndTraining(f) => {
                if let Poll::Ready(_) = f.as_mut().poll(cx) {
                    debug!("client {} back to Waiting state", pin.id);
                    pin.state = ClientState::Waiting;
                } else {
                    debug!("client {} still in EndTraining state", pin.id);
                }
            }
            _ => {}
        }
        Poll::Pending
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let config = CoordinatorConfig {
        rounds: 3,
        min_clients: 3,
        participants_ratio: 0.5,
    };
    let (coordinator, handle) =
        CoordinatorService::new(MeanAggregator::new(), RandomSelector, 0, config);
    tokio::spawn(coordinator);

    for _ in 0..9 {
        let (client, heartbeat) = Client::new(handle.clone(), Box::new(train)).await.unwrap();
        tokio::spawn(heartbeat.start());
        tokio::spawn(client);
    }

    let (client, heartbeat) = Client::new(handle.clone(), Box::new(train)).await.unwrap();
    tokio::spawn(heartbeat.start());
    client.await;

    Ok(())
}

pub struct RandomSelector;

impl Selector for RandomSelector {
    fn select(
        &mut self,
        min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        _selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId> {
        waiting.choose_multiple(&mut rand::thread_rng(), min_count)
    }
}

#[derive(Debug, Default)]
pub struct MeanAggregator {
    sum: u32,
    results_count: u32,
}

impl MeanAggregator {
    fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, Display)]
pub struct NoError;
impl ::std::error::Error for NoError {}

impl Aggregator<u32> for MeanAggregator {
    type Error = NoError;

    fn add_local_result(&mut self, result: u32) -> Result<(), Self::Error> {
        self.sum += result;
        self.results_count += 1;
        Ok(())
    }

    fn aggregate(&mut self) -> Result<u32, Self::Error> {
        let mean = self.sum as f32 / self.results_count as f32;
        Ok(f32::ceil(mean) as i32 as u32)
    }
}

fn train(weights: u32) -> Pin<Box<dyn Future<Output = u32> + Send>> {
    Box::pin(async move {
        // sleep for 3 seconds to pretend training takes some time
        delay_for(Duration::from_millis(3000)).await;
        let mut rng = rand::thread_rng();
        let random_increment: u8 = rng.gen();
        weights + random_increment as u32
    })
}
