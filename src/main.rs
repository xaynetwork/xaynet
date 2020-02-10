#![feature(or_patterns)]
#![feature(bool_to_option)]
#[macro_use]
extern crate log;
use derive_more::Display;

mod coordinator;
use coordinator::{Aggregator, ClientId, CoordinatorConfig, CoordinatorService, Selector};

use env_logger;
use rand::seq::IteratorRandom;
use std::iter::Iterator;

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
    let (coordinator, mut handle) =
        CoordinatorService::new(MeanAggregator::new(), RandomSelector, 0, config);
    tokio::spawn(coordinator);
    let uuid = handle.rendez_vous(None).await.unwrap();
    handle.rendez_vous(None).await.unwrap();
    Ok(())
}
