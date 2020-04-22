use crate::coordinator::{MaskHash, Phase, SeedDict, SumDict};
use counter::Counter;
use sodiumoxide::crypto::{box_, sign};
use std::collections::HashMap;

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
    EncrPk(box_::PublicKey),
    EncrSk(box_::SecretKey),
    SignPk(sign::PublicKey),
    SignSk(sign::SecretKey),
    Sum(f64),
    Update(f64),
    Seed(Vec<u8>),
    MinSum(usize),
    MinUpdate(usize),
    Phase(Phase),
    Round(u64),
}

pub enum CoordinatorStateRequest {
    EncrPk,
    EncrSk,
    SignPk,
    SignSk,
    Sum,
    Update,
    Seed,
    MinSum,
    MinUpdate,
    Phase,
    Round,
}

pub struct SumDictEntry(pub box_::PublicKey, pub box_::PublicKey);
pub struct SeedDictEntry(pub box_::PublicKey, pub HashMap<box_::PublicKey, Vec<u8>>);
pub struct MaskDictEntry(pub Vec<u8>);

pub type CoordinatorPartialStateResult = CoordinatorPartialState;
pub type SumDictResult = SumDict;
pub type SeedDictResult = SeedDict;
pub type SubSeedDictResult = HashMap<box_::PublicKey, Vec<u8>>;
pub type MaskDictResult = Counter<MaskHash>;

#[derive(Debug, PartialEq)]
pub struct CoordinatorPartialState {
    pub(crate) encr_pk: box_::PublicKey,
    pub(crate) encr_sk: box_::SecretKey,
    pub(crate) sign_pk: sign::PublicKey,
    pub(crate) sign_sk: sign::SecretKey,
    pub(crate) sum: f64,
    pub(crate) update: f64,
    pub(crate) seed: Vec<u8>,
    pub(crate) min_sum: usize,
    pub(crate) min_update: usize,
    pub(crate) phase: Phase,
}
