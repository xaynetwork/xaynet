use crate::storage::state::{GenericSnapshotHandler, Snapshot, SnapshotType};
use async_trait::async_trait;
use std::error::Error;
use tokio::{fs::File, prelude::*};

struct FileSnapshot {
    file_path_aggregator: String,
    file_path_coordinator: String,
}

impl FileSnapshot {
    pub fn new<S: Into<String>>(file_path_coordinator: S, file_path_aggregator: S) -> Self {
        Self {
            file_path_coordinator: file_path_coordinator.into(),
            file_path_aggregator: file_path_aggregator.into(),
        }
    }
}

#[async_trait]
impl GenericSnapshotHandler for FileSnapshot {
    async fn snapshot(
        &self,
        snapshot: Snapshot,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let (file_path, to_json) = match snapshot {
            Snapshot::Coordinator(coordinator) => (
                &self.file_path_coordinator,
                serde_json::to_vec(&coordinator)?,
            ),
            Snapshot::Aggregator(aggregator) => {
                (&self.file_path_aggregator, serde_json::to_vec(&aggregator)?)
            }
        };
        let mut file = File::create(file_path).await?;
        file.write_all(&to_json).await?;
        Ok(())
    }

    async fn restore(
        &self,
        snapshot_type: SnapshotType,
    ) -> Result<Snapshot, Box<dyn std::error::Error + 'static>> {
        let file_path = match snapshot_type {
            SnapshotType::Coordinator => &self.file_path_coordinator,
            SnapshotType::Aggregator => &self.file_path_aggregator,
        };

        let file = File::open(file_path).await?;
        let file = file.into_std().await;
        let reader = std::io::BufReader::new(file);
        let result = match snapshot_type {
            SnapshotType::Coordinator => Snapshot::Coordinator(serde_json::from_reader(reader)?),
            SnapshotType::Aggregator => Snapshot::Aggregator(serde_json::from_reader(reader)?),
        };
        Ok(result)
    }
}

#[tokio::test]
async fn test() {
    use crate::coordinator::{
        Coordinator, EncryptedMaskingSeed, Phase, UpdateParticipantPublicKey,
    };
    use sodiumoxide::{
        crypto::{box_, sign},
        randombytes::randombytes,
    };
    use std::{
        collections::HashMap,
        fs, iter,
        time::{Duration, Instant},
    };

    let ps = FileSnapshot::new("./coordinator.json", "./aff.json");

    // create new coordinator
    let mut coord = Coordinator::new().unwrap();

    // make snapshot
    ps.snapshot(Snapshot::from(&coord)).await.unwrap();

    // change some values
    coord.phase = Phase::Sum;
    for _ in 0..1000 {
        let (k, _) = box_::gen_keypair();
        coord.dict_sum.insert(k, k);
    }
    for _ in 0..1000 {
        let (k, _) = box_::gen_keypair();
        let mut sub_dict: HashMap<UpdateParticipantPublicKey, EncryptedMaskingSeed> =
            HashMap::new();
        sub_dict.insert(k, randombytes(80));
        coord.dict_seed.insert(k, sub_dict);
    }
    coord.dict_mask.update(iter::once(vec![0, 23, 4, 2]));

    println!("Begin Snapshot");
    let now = Instant::now();
    ps.snapshot(Snapshot::from(&coord)).await.unwrap();
    let new_now = Instant::now();
    println!("End Snapshot {:?}", new_now.duration_since(now));

    println!("Begin Restore");
    let now = Instant::now();
    let restored_coordinator = ps.restore(SnapshotType::Coordinator).await.unwrap();
    let new_now = Instant::now();
    println!("End Restore {:?}", new_now.duration_since(now));
}
