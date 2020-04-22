use crate::coordinator::Coordinator;
use async_trait::async_trait;
use std::error::Error;

// Dummy struct for the real Aggregator struct
pub mod aggregator {

    #[derive(Clone, Serialize, Deserialize, Debug)]
    pub struct Aggregator {
        pub global_model: String,
    }
}

use crate::coordserde::state::Snapshot::Aggregator;

pub enum Snapshot {
    Coordinator(Coordinator),
    Aggregator(aggregator::Aggregator),
}

pub enum SnapshotType {
    Coordinator,
    Aggregator,
}

impl From<&Coordinator> for Snapshot {
    fn from(coordinator: &Coordinator) -> Self {
        Self::Coordinator(coordinator.clone())
    }
}

impl From<aggregator::Aggregator> for Snapshot {
    fn from(aggregator: aggregator::Aggregator) -> Self {
        Self::Aggregator(aggregator.clone())
    }
}
#[async_trait]
pub trait GenericSnapshotHandler {
    async fn snapshot(
        &self,
        snapshot: Snapshot,
    ) -> Result<(), Box<dyn std::error::Error + 'static>>;

    async fn restore(
        &self,
        snapshot_type: SnapshotType,
    ) -> Result<Snapshot, Box<dyn std::error::Error + 'static>>;
}
