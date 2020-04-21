use crate::{
    coordinator::{Coordinator, EncryptedMaskingSeed, MaskHash, SumDict, SumParticipantPublicKey},
    storage::error::*,
};
use counter::Counter;
use data_encoding::HEXUPPER;
use redis::ToRedisArgs;
use sodiumoxide::crypto::box_;
use std::{
    collections::HashMap,
    convert::{From, TryFrom},
    iter,
    str::FromStr,
};

fn public_key_to_hex(pk: &box_::PublicKey) -> String {
    HEXUPPER.encode(pk.as_ref())
}

fn hex_to_public_key(hex: &String) -> Result<box_::PublicKey, StoreError> {
    HEXUPPER
        .decode(hex.as_bytes())
        .map_err(|_| StoreError::Convert)
        .and_then(|pk_as_vec| {
            box_::PublicKey::from_slice(pk_as_vec.as_slice()).ok_or(StoreError::Convert)
        })
}

#[derive(Debug, PartialEq)]
pub struct CoordinatorPartialState {
    encr_pk: String,
    encr_sk: String,
    sign_pk: String,
    sign_sk: String,
    sum: f64,
    update: f64,
    seed: String,
    min_sum: usize,
    min_update: usize,
    phase: String,
}

impl CoordinatorPartialState {
    pub fn to_args(&self) -> Vec<(impl ToRedisArgs, impl ToRedisArgs)> {
        vec![
            ("encr_pk", self.encr_pk.clone()),
            ("encr_sk", self.encr_sk.clone()),
            ("sign_pk", self.sign_pk.clone()),
            ("sign_sk", self.sign_sk.clone()),
            ("sum", self.sum.to_string()),
            ("update", self.update.to_string()),
            ("seed", self.seed.clone()),
            ("min_sum", self.min_sum.to_string()),
            ("min_update", self.min_update.to_string()),
            ("phase", self.phase.clone()),
        ]
    }

    pub fn keys() -> Vec<&'static str> {
        vec![
            "encr_pk",
            "encr_sk",
            "sign_pk",
            "sign_sk",
            "sum",
            "update",
            "seed",
            "min_sum",
            "min_update",
            "phase",
        ]
    }
}

impl From<&Coordinator> for CoordinatorPartialState {
    fn from(coordinator: &Coordinator) -> Self {
        Self {
            encr_pk: HEXUPPER.encode(&coordinator.encr_pk.as_ref()),
            encr_sk: HEXUPPER.encode(&coordinator.encr_sk.as_ref()),
            sign_pk: HEXUPPER.encode(&coordinator.sign_pk.as_ref()),
            sign_sk: HEXUPPER.encode(&coordinator.sign_sk.as_ref()),
            sum: coordinator.sum,
            update: coordinator.update,
            seed: HEXUPPER.encode(&coordinator.seed.as_ref()),
            min_sum: coordinator.min_sum,
            min_update: coordinator.min_update,
            phase: coordinator.phase.to_string(),
        }
    }
}

pub struct CoordinatorPartialStateResult(
    pub  (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<usize>,
        Option<usize>,
        Option<String>,
    ),
);

impl TryFrom<CoordinatorPartialStateResult> for CoordinatorPartialState {
    type Error = StoreError;
    fn try_from(result: CoordinatorPartialStateResult) -> Result<Self, Self::Error> {
        Ok(Self {
            encr_pk: (result.0).0.ok_or(StoreError::Convert)?,
            encr_sk: (result.0).1.ok_or(StoreError::Convert)?,
            sign_pk: (result.0).2.ok_or(StoreError::Convert)?,
            sign_sk: (result.0).3.ok_or(StoreError::Convert)?,
            sum: f64::from_str(&(result.0).4.ok_or(StoreError::Convert)?)
                .map_err(|_| StoreError::Convert)?,
            update: f64::from_str(&(result.0).5.ok_or(StoreError::Convert)?)
                .map_err(|_| StoreError::Convert)?,
            seed: (result.0).6.ok_or(StoreError::Convert)?,
            min_sum: (result.0).7.ok_or(StoreError::Convert)?,
            min_update: (result.0).8.ok_or(StoreError::Convert)?,
            phase: (result.0).9.ok_or(StoreError::Convert)?,
        })
    }
}

pub struct SumDictEntry(pub box_::PublicKey, pub box_::PublicKey);

impl SumDictEntry {
    pub fn key() -> &'static str {
        "sum_dict"
    }
}

impl SumDictEntry {
    pub fn to_args(&self) -> (String, String) {
        (public_key_to_hex(&self.0), public_key_to_hex(&self.1))
    }
}

pub struct SumDictResult(pub Vec<(String, String)>);

impl TryFrom<SumDictResult> for SumDict {
    type Error = StoreError;
    fn try_from(result: SumDictResult) -> Result<Self, Self::Error> {
        let mut sum_dict: SumDict = HashMap::new();
        for (k, v) in result.0.iter() {
            let sum_pk = hex_to_public_key(&k)?;
            let sum_epk = hex_to_public_key(&v)?;
            sum_dict.insert(sum_pk, sum_epk);
        }
        Ok(sum_dict)
    }
}

pub struct SeedDictEntry(pub box_::PublicKey, pub HashMap<box_::PublicKey, Vec<u8>>);

impl SeedDictEntry {
    pub fn key() -> &'static str {
        "seed_dict"
    }
}

impl SeedDictEntry {
    pub fn to_args(&self) -> (String, Vec<(String, String)>) {
        let sub_dict = &self
            .1
            .iter()
            .map(|(update_pk, seed)| (public_key_to_hex(&update_pk), HEXUPPER.encode(&seed)))
            .collect::<Vec<(String, String)>>();
        (public_key_to_hex(&self.0), sub_dict.to_vec())
    }
}

pub struct SeedDictValueEntryResult(pub Vec<(String, String)>);

impl TryFrom<SeedDictValueEntryResult> for HashMap<SumParticipantPublicKey, EncryptedMaskingSeed> {
    type Error = StoreError;
    fn try_from(result: SeedDictValueEntryResult) -> Result<Self, Self::Error> {
        let mut sub_dict: HashMap<SumParticipantPublicKey, EncryptedMaskingSeed> = HashMap::new();
        for (k, v) in result.0.iter() {
            let update_pk = hex_to_public_key(&k)?;
            let seed = HEXUPPER
                .decode(&v.as_bytes())
                .map_err(|_| StoreError::Convert)?;
            sub_dict.insert(update_pk, seed);
        }
        Ok(sub_dict)
    }
}

pub struct SeedDictKeyResult(pub String);

impl TryFrom<SeedDictKeyResult> for SumParticipantPublicKey {
    type Error = StoreError;
    fn try_from(result: SeedDictKeyResult) -> Result<Self, Self::Error> {
        let sum_pk = hex_to_public_key(&result.0)?;
        Ok(sum_pk)
    }
}

pub struct MaskDictEntry(pub Vec<u8>);

impl MaskDictEntry {
    pub fn to_args(&self) -> String {
        HEXUPPER.encode(&self.0.as_ref())
    }
}

impl MaskDictEntry {
    pub fn key() -> &'static str {
        "mask_dict"
    }
}

pub struct MaskDictResult(pub Vec<String>);

impl TryFrom<MaskDictResult> for Counter<MaskHash> {
    type Error = StoreError;
    fn try_from(result: MaskDictResult) -> Result<Self, Self::Error> {
        let mut counter: Counter<MaskHash> = Counter::new();
        for v in result.0.iter() {
            let mask_hash = HEXUPPER
                .decode(&v.as_bytes())
                .map_err(|_| StoreError::Convert)?;
            counter.update(iter::once(mask_hash.to_vec()))
        }
        Ok(counter)
    }
}
