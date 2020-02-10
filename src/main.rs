#![feature(or_patterns)]
#![feature(bool_to_option)]
#[macro_use]
extern crate log;
use derive_more::Display;
use futures::ready;

use rand::Rng;

mod coordinator;
use coordinator::{
    Aggregator, ClientId, CoordinatorConfig, CoordinatorHandle, CoordinatorService,
    HeartBeatResponse, RendezVousResponse, Selector, StartTrainingResponse,
};
use futures::stream::Stream;

use env_logger;
use rand::seq::IteratorRandom;
use std::{
    future::Future,
    iter::Iterator,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    sync::{
        mpsc,
        oneshot::{self, error::TryRecvError},
    },
    time::delay_for,
};

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let config = CoordinatorConfig {
        rounds: 10,
        min_clients: 1,
        participants_ratio: 1.0,
    };
    let (coordinator, handle) =
        CoordinatorService::new(MeanAggregator::new(), RandomSelector, 0, config);
    tokio::spawn(coordinator);

    for _ in 0..10 {
        let (client, heartbeat) = Client::new(handle.clone()).await;
        tokio::spawn(heartbeat.start());
        tokio::spawn(client);
    }

    let (client, heartbeat) = Client::new(handle.clone()).await;
    tokio::spawn(heartbeat.start());
    client.await;

    Ok(())
}

enum ClientState {
    Waiting,
    Training(oneshot::Receiver<u32>),
    EndTraining(oneshot::Receiver<()>),
}

struct Client {
    handle: CoordinatorHandle<u32>,
    state: ClientState,
    id: ClientId,
    heartbeat_rx: mpsc::Receiver<HeartBeatResponse>,
}

struct HeartBeat {
    id: ClientId,
    tx: mpsc::Sender<HeartBeatResponse>,
    handle: CoordinatorHandle<u32>,
}
impl HeartBeat {
    async fn start(mut self) {
        async {
            loop {
                match self.handle.heartbeat(self.id).await {
                    Err(()) => return,
                    Ok(HeartBeatResponse::Finish) | Ok(HeartBeatResponse::Reject) => {
                        self.tx.send(HeartBeatResponse::Finish).await.unwrap();
                        break;
                    }
                    Ok(response) => {
                        self.tx.send(response).await.unwrap();
                        // sleep for 1 second
                        delay_for(Duration::from_millis(1000)).await;
                    }
                }
            }
        }
        .await
    }
}

impl Client {
    async fn new(mut handle: CoordinatorHandle<u32>) -> (Self, HeartBeat) {
        match handle.rendez_vous().await.unwrap() {
            RendezVousResponse::Accept(id) => {
                let (tx, rx) = mpsc::channel(10);
                let client = Self {
                    handle: handle.clone(),
                    state: ClientState::Waiting,
                    id: id.clone(),
                    heartbeat_rx: rx,
                };
                let heartbeat = HeartBeat { handle, id, tx };
                (client, heartbeat)
            }
            RendezVousResponse::Reject => panic!(),
        }
    }

    fn poll_heartbeats(&mut self, cx: &mut Context) -> Poll<()> {
        loop {
            match ready!(Pin::new(&mut self.heartbeat_rx).poll_next(cx)) {
                Some(response) => {
                    use HeartBeatResponse::*;
                    match (response, &self.state) {
                        (Finish | Reject, _) => {
                            return Poll::Ready(());
                        }
                        (Round(_), ClientState::Waiting) => {
                            // we've been selected!
                            let (result_tx, result_rx) = oneshot::channel();
                            self.state = ClientState::Training(result_rx);
                            tokio::spawn(start_training(self.id, self.handle.clone(), result_tx));
                        }
                        _ => {}
                    }
                }
                None => return Poll::Ready(()),
            }
        }
    }
}

impl Future for Client {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pin = self.get_mut();
        match pin.poll_heartbeats(cx) {
            Poll::Ready(()) => return Poll::Ready(()),
            Poll::Pending => {}
        }
        match &mut pin.state {
            ClientState::Training(result_rx) => match result_rx.try_recv() {
                Ok(result) => {
                    let (end_training_tx, end_training_rx) = oneshot::channel();
                    pin.state = ClientState::EndTraining(end_training_rx);
                    let mut handle = pin.handle.clone();
                    let id = pin.id.clone();
                    tokio::spawn(async move {
                        handle.end_training(id, result).await;
                        end_training_tx.send(()).unwrap();
                    });
                    Poll::Pending
                }
                Err(TryRecvError::Empty) => Poll::Pending,
                Err(TryRecvError::Closed) => panic!(),
            },
            ClientState::EndTraining(rx) => match rx.try_recv() {
                Ok(()) => {
                    pin.state = ClientState::Waiting;
                    Poll::Pending
                }
                Err(TryRecvError::Empty) => Poll::Pending,
                Err(TryRecvError::Closed) => panic!(),
            },
            _ => Poll::Pending,
        }
    }
}

async fn start_training(
    id: ClientId,
    mut handle: CoordinatorHandle<u32>,
    result_tx: oneshot::Sender<u32>,
) {
    match handle.start_training(id).await.unwrap() {
        StartTrainingResponse::Accept(msg) => {
            let global_weights = msg.global_weights;
            delay_for(Duration::from_millis(10000)).await;
            let mut rng = rand::thread_rng();
            let random_increment: u8 = rng.gen();
            result_tx
                .send(global_weights + random_increment as u32)
                .unwrap();
        }
        _ => panic!(),
    }
}
