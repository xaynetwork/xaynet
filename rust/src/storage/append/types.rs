use crate::coordinator::{
    Coordinator, EncryptedMaskingSeed, SumDict, SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
};
use data_encoding::HEXUPPER;
use redis::ToRedisArgs;
use sodiumoxide::crypto::box_;
use std::{collections::HashMap, convert::TryFrom, str::FromStr};

#[derive(Debug)]
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

#[derive(Debug)]
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

impl TryFrom<CoordinatorPartialStateResult> for CoordinatorPartialState {
    type Error = Box<dyn std::error::Error + 'static>;
    fn try_from(result: CoordinatorPartialStateResult) -> Result<Self, Self::Error> {
        Ok(Self {
            encr_pk: (result.0).0.ok_or(StoreError::Read)?,
            encr_sk: (result.0).1.ok_or(StoreError::Read)?,
            sign_pk: (result.0).2.ok_or(StoreError::Read)?,
            sign_sk: (result.0).3.ok_or(StoreError::Read)?,
            sum: f64::from_str(&(result.0).4.ok_or(StoreError::Read)?)?,
            update: f64::from_str(&(result.0).5.ok_or(StoreError::Read)?)?,
            seed: (result.0).6.ok_or(StoreError::Read)?,
            min_sum: (result.0).7.ok_or(StoreError::Read)?,
            min_update: (result.0).8.ok_or(StoreError::Read)?,
            phase: (result.0).9.ok_or(StoreError::Read)?,
        })
    }
}

pub struct SumDictEntry(pub box_::PublicKey, pub box_::PublicKey);

impl SumDictEntry {
    pub fn to_args(&self) -> (String, String) {
        (
            HEXUPPER.encode(&self.0.as_ref()),
            HEXUPPER.encode(&self.1.as_ref()),
        )
    }
}

pub struct SumDictResult(pub Vec<(String, String)>);

impl TryFrom<SumDictResult> for SumDict {
    type Error = Box<dyn std::error::Error + 'static>;
    fn try_from(result: SumDictResult) -> Result<Self, Self::Error> {
        let mut sum_dict: SumDict = HashMap::new();
        for (k, v) in result.0.iter() {
            let new_k = box_::PublicKey::from_slice(HEXUPPER.decode(&k.as_bytes())?.as_slice())
                .ok_or(StoreError::Read)?;
            let new_v = box_::PublicKey::from_slice(HEXUPPER.decode(&v.as_bytes())?.as_slice())
                .ok_or(StoreError::Read)?;
            sum_dict.insert(new_k, new_v);
        }
        Ok(sum_dict)
    }
}

pub struct SeedDictEntry(pub box_::PublicKey, pub HashMap<box_::PublicKey, Vec<u8>>);

impl SeedDictEntry {
    pub fn to_args(&self) -> (String, Vec<(String, String)>) {
        let sub_dict = &self
            .1
            .iter()
            .map(|(k, v)| (HEXUPPER.encode(&k.as_ref()), HEXUPPER.encode(&v)))
            .collect::<Vec<(String, String)>>();
        (HEXUPPER.encode(&self.0.as_ref()), sub_dict.to_vec())
    }
}

pub struct SeedDictEntryResult(pub Vec<(String, String)>);

impl TryFrom<SeedDictEntryResult> for HashMap<SumParticipantPublicKey, EncryptedMaskingSeed> {
    type Error = Box<dyn std::error::Error + 'static>;
    fn try_from(result: SeedDictEntryResult) -> Result<Self, Self::Error> {
        let mut sub_dict: HashMap<SumParticipantPublicKey, EncryptedMaskingSeed> = HashMap::new();
        for (k, v) in result.0.iter() {
            let new_k = box_::PublicKey::from_slice(HEXUPPER.decode(&k.as_bytes())?.as_slice())
                .ok_or(StoreError::Read)?;
            let new_v = HEXUPPER.decode(&v.as_bytes())?;
            sub_dict.insert(new_k, new_v);
        }
        Ok(sub_dict)
    }
}

pub struct SeedDictKeyResult(pub String);

impl TryFrom<SeedDictKeyResult> for SumParticipantPublicKey {
    type Error = Box<dyn std::error::Error + 'static>;
    fn try_from(result: SeedDictKeyResult) -> Result<Self, Self::Error> {
        let key = box_::PublicKey::from_slice(HEXUPPER.decode(&result.0.as_bytes())?.as_slice())
            .ok_or(StoreError::Read)?;
        Ok(key)
    }
}

pub struct MaskDictEntry(pub Vec<u8>);

impl MaskDictEntry {
    pub fn to_args(&self) -> String {
        HEXUPPER.encode(&self.0.as_ref())
    }
}

use std::{convert::From, error::Error, fmt};

#[derive(Debug)] // Allow the use of "{:?}" format specifier
enum StoreError {
    Read,
}

// Allow the use of "{}" format specifier
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StoreError::Read => write!(f, "Read Error!",),
        }
    }
}

// Allow this type to be treated like an error
impl Error for StoreError {
    fn description(&self) -> &str {
        match *self {
            StoreError::Read => "Read failed!",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            StoreError::Read => None,
        }
    }
}
