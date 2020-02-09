#![feature(or_patterns)]
#![feature(bool_to_option)]
#[macro_use]
extern crate log;
use derive_more::Display;

mod coordinator;
use coordinator::{Aggregator, ClientId, Selector};

use rand::{rngs::ThreadRng, seq::IteratorRandom};
use std::iter::Iterator;

pub struct RandomSelector(ThreadRng);

impl RandomSelector {
    fn new() -> Self {
        Self(rand::thread_rng())
    }
}
impl Selector for RandomSelector {
    fn select(
        &mut self,
        min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        _selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId> {
        waiting.choose_multiple(&mut self.0, min_count)
    }
}

pub struct MeanAggregator {
    sum: u32,
    results_count: u32,
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

fn main() {}
