#![feature(or_patterns)]
#![feature(bool_to_option)]
#[macro_use]
extern crate log;

#[macro_use]
extern crate serde;
use derive_more::Display;

use rand::seq::IteratorRandom;
mod aggregator;
mod client;
pub mod common;
mod coordinator;

use crate::{
    common::ClientId,
    coordinator::{Aggregator, CoordinatorConfig, CoordinatorService, Selector},
};
use client::Client;

use rand::Rng;
use std::{future::Future, pin::Pin, time::Duration};
use tokio::time::delay_for;

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
