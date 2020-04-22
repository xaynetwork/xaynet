use crate::{
    coordinator::{MaskDict, MaskHash, Phase},
    storage::redis,
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    EncrMaskSeed,
    LocalSeedDict,
    SeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

use std::collections::HashMap;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

// pub struct StorageService {
//     store: RedisStore,
//     service_req_rx: UnboundedReceiver<CoordinatorStateRequest>,
// }

// Service API
pub enum StoreRequests {
    WriteCoordinatorState(CoordinatorState),
    // WriteMultiCoordinatorState(Vec<CoordinatorState>),
    WriteSumDictEntry(SumDictEntry),
    WriteSeedDictEntry(SeedDictEntry),
    WriteMaskDictEntry(MaskDictEntry),

    ReadCoordinatorPartialState(CoordinatorStateRequest), // -> CoordinatorPartialStateResult
    ReadSumDict,                                          // -> SumDictResult
    ReadSeedDict,                                         // -> SeedDictResult
    ReadMaskDict,                                         // -> MaskDictResult
}

#[derive(Debug, PartialEq)]
pub enum CoordinatorState {
    CoordPk(CoordinatorPublicKey),
    CoordSk(CoordinatorSecretKey),
    Sum(f64),
    Update(f64),
    Seed(Vec<u8>),
    MinSum(usize),
    MinUpdate(usize),
    Phase(Phase),
    Round(u64),
}

pub enum CoordinatorStateRequest {
    CoordPk,
    CoordSk,
    Sum,
    Update,
    Seed,
    MinSum,
    MinUpdate,
    Phase,
    Round,
}

pub struct SumDictEntry(
    pub SumParticipantPublicKey,
    pub SumParticipantEphemeralPublicKey,
);
pub struct SeedDictEntry(
    pub SumParticipantPublicKey,
    pub HashMap<UpdateParticipantPublicKey, EncrMaskSeed>,
);
pub struct MaskDictEntry(pub MaskHash);

pub type CoordinatorPartialStateResult = CoordinatorPartialState;
pub type SumDictResult = SumDict;
pub type SeedDictResult = SeedDict;
pub type SubSeedDictResult = HashMap<UpdateParticipantPublicKey, EncrMaskSeed>;
pub type MaskDictResult = MaskDict;

#[derive(Debug, PartialEq)]
pub struct CoordinatorPartialState {
    pub(crate) pk: CoordinatorPublicKey,
    pub(crate) sk: CoordinatorSecretKey,
    pub(crate) sum: f64,
    pub(crate) update: f64,
    pub(crate) seed: Vec<u8>,
    pub(crate) min_sum: usize,
    pub(crate) min_update: usize,
    pub(crate) phase: Phase,
}
