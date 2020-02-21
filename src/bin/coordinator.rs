use rand::seq::IteratorRandom;

use xain_fl::{
    common::ClientId,
    coordinator::core::{CoordinatorConfig, CoordinatorService, Selector},
};

#[tokio::main]
async fn main() {
    _main().await;
}

async fn _main() {
    env_logger::init();
    let config = CoordinatorConfig {
        rounds: 3,
        min_clients: 3,
        participants_ratio: 0.5,
    };
    let (coordinator, _handle) =
        CoordinatorService::new(RandomSelector, config, "localhost:5555", "localhost:6666");
    coordinator.await;
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
