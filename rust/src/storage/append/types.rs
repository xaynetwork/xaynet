use crate::coordinator::Coordinator;
use data_encoding::HEXUPPER;
use redis::ToRedisArgs;
use sodiumoxide::crypto::box_;
use std::collections::HashMap;

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

pub struct SumDictEntry(box_::PublicKey, box_::PublicKey);

impl SumDictEntry {
    pub fn to_args(&self) -> (String, String) {
        (
            HEXUPPER.encode(&self.0.as_ref()),
            HEXUPPER.encode(&self.1.as_ref()),
        )
    }
}

pub struct SeedDictEntry(box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>);

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

pub struct MaskDictEntry(Vec<u8>);

impl MaskDictEntry {
    pub fn to_args(&self) -> String {
        HEXUPPER.encode(&self.0.as_ref())
    }
}
