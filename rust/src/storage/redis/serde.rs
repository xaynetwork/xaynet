use crate::{
    coordinator::MaskHash,
    storage::state::{
        CoordinatorState, CoordinatorState::*, CoordinatorStateRequest, MaskDictEntry,
        MaskDictResult, SeedDictEntry, SubSeedDictResult, SumDictEntry, SumDictResult,
    },
};
use counter::Counter;
use data_encoding::HEXUPPER;
use rmp_serde as rmps;
use rmps::Serializer;
use serde::Serialize;
use std::{collections::HashMap, iter};

pub fn serialize_coordinator_state(
    state: &CoordinatorState,
) -> Result<(&str, Vec<u8>), Box<dyn std::error::Error + 'static>> {
    let se_value = match state {
        EncrPk(enc_pk) => {
            let mut se_value = Vec::new();
            enc_pk.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }
        EncrSk(enc_sk) => {
            let mut se_value = Vec::new();
            enc_sk.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }
        SignPk(sign_pk) => {
            let mut se_value = Vec::new();
            sign_pk.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }

        SignSk(sign_sk) => {
            let mut se_value = Vec::new();
            sign_sk.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }

        Sum(sum) => {
            let mut se_value = Vec::new();
            sum.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }

        Update(update) => {
            let mut se_value = Vec::new();
            update.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }

        Seed(seed) => {
            let mut se_value = Vec::new();
            seed.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }

        MinSum(min_sum) => {
            let mut se_value = Vec::new();
            min_sum.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }

        MinUpdate(min_update) => {
            let mut se_value = Vec::new();
            min_update.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }

        Phase(phase) => {
            let mut se_value = Vec::new();
            phase.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }
        Round(round) => {
            let mut se_value = Vec::new();
            round.serialize(&mut Serializer::new(&mut se_value))?;
            se_value
        }
    };
    Ok((RedisKeys::from_coordinator_state(state), se_value))
}

pub fn deserialize_coordinator_state(
    state_type: &CoordinatorStateRequest,
    value: &Vec<u8>,
) -> Result<CoordinatorState, Box<dyn std::error::Error + 'static>> {
    let result = match state_type {
        CoordinatorStateRequest::EncrPk => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::EncrPk(de_value)
        }
        CoordinatorStateRequest::EncrSk => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::EncrSk(de_value)
        }
        CoordinatorStateRequest::SignPk => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::SignPk(de_value)
        }
        CoordinatorStateRequest::SignSk => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::SignSk(de_value)
        }
        CoordinatorStateRequest::Sum => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::Sum(de_value)
        }
        CoordinatorStateRequest::Update => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::Update(de_value)
        }
        CoordinatorStateRequest::Seed => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::Seed(de_value)
        }
        CoordinatorStateRequest::MinSum => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::MinSum(de_value)
        }
        CoordinatorStateRequest::MinUpdate => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::MinUpdate(de_value)
        }
        CoordinatorStateRequest::Phase => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::Phase(de_value)
        }
        CoordinatorStateRequest::Round => {
            let de_value = rmps::from_read_ref(value)?;
            CoordinatorState::Round(de_value)
        }
    };
    Ok(result)
}

pub fn serialize_sum_dict_entry(
    entry: &SumDictEntry,
) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error + 'static>> {
    let mut se_sum_pk = Vec::new();
    entry.0.serialize(&mut Serializer::new(&mut se_sum_pk))?;
    let mut se_sum_epk = Vec::new();
    entry.1.serialize(&mut Serializer::new(&mut se_sum_epk))?;
    Ok((se_sum_pk, se_sum_epk))
}

pub fn deserialize_sum_dict(
    de_sum_dict: &Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<SumDictResult, Box<dyn std::error::Error + 'static>> {
    let mut sum_dict: SumDictResult = HashMap::new();
    for (sum_pk, sum_epk) in de_sum_dict.iter() {
        sum_dict.insert(rmps::from_read_ref(sum_pk)?, rmps::from_read_ref(sum_epk)?);
    }
    Ok(sum_dict)
}

pub fn serialize_seed_dict_entry(
    entry: &SeedDictEntry,
) -> Result<(Vec<u8>, Vec<(Vec<u8>, Vec<u8>)>), Box<dyn std::error::Error + 'static>> {
    let mut se_sum_pk = Vec::new();
    entry.0.serialize(&mut Serializer::new(&mut se_sum_pk))?;

    let mut se_sub_dict = Vec::new();
    for (update_pk, seed) in entry.1.iter() {
        let mut se_update_pk = Vec::new();
        update_pk.serialize(&mut Serializer::new(&mut se_update_pk))?;
        let mut se_seed = Vec::new();
        seed.serialize(&mut Serializer::new(&mut se_seed))?;
        se_sub_dict.push((se_update_pk, se_seed));
    }

    Ok((se_sum_pk, se_sub_dict))
}

pub fn deserialize_seed_dict_entry(
    se_sub_seed_dict: &Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<SubSeedDictResult, Box<dyn std::error::Error + 'static>> {
    let mut sub_seed_dict: SubSeedDictResult = HashMap::new();
    for (update_pk, seed) in se_sub_seed_dict.iter() {
        sub_seed_dict.insert(rmps::from_read_ref(update_pk)?, rmps::from_read_ref(seed)?);
    }
    Ok(sub_seed_dict)
}

pub fn deserialize_seed_dict_key(
    se_seed_dict_key: &Vec<u8>,
) -> Result<sodiumoxide::crypto::box_::PublicKey, Box<dyn std::error::Error + 'static>> {
    Ok(rmps::from_read_ref(se_seed_dict_key)?)
}

pub fn serialize_mask_dict_entry(
    entry: &MaskDictEntry,
) -> Result<Vec<u8>, Box<dyn std::error::Error + 'static>> {
    let mut se_mask_hash = Vec::new();
    entry.0.serialize(&mut Serializer::new(&mut se_mask_hash))?;

    Ok(se_mask_hash)
}

pub fn deserialize_mask_dict(
    se_mask_dict: &Vec<Vec<u8>>,
) -> Result<MaskDictResult, Box<dyn std::error::Error + 'static>> {
    let mut counter: Counter<MaskHash> = Counter::new();
    for mask_hash in se_mask_dict.iter() {
        let mask_hash = rmps::from_read_ref(mask_hash)?;
        counter.update(iter::once(mask_hash));
    }
    Ok(counter)
}

pub struct RedisKeys;

impl RedisKeys {
    pub fn sum_dict() -> &'static str {
        "sum_dict"
    }
    pub fn seed_dict() -> &'static str {
        "seed_dict"
    }

    pub fn sub_seed_dict_key(key: &Vec<u8>) -> String {
        format!("{}:{}", RedisKeys::seed_dict(), HEXUPPER.encode(key))
    }

    pub fn mask_dict() -> &'static str {
        "mask_dict"
    }

    pub fn from_coordinator_state(state: &CoordinatorState) -> &'static str {
        match state {
            CoordinatorState::EncrPk(_) => "enc_pk",
            CoordinatorState::EncrSk(_) => "enc_sk",
            CoordinatorState::SignPk(_) => "sign_pk",
            CoordinatorState::SignSk(_) => "sign_sk",
            CoordinatorState::Sum(_) => "sum",
            CoordinatorState::Update(_) => "update",
            CoordinatorState::Seed(_) => "seed",
            CoordinatorState::MinSum(_) => "min_sum",
            CoordinatorState::MinUpdate(_) => "min_update",
            CoordinatorState::Phase(_) => "phase",
            CoordinatorState::Round(_) => "round",
        }
    }

    pub fn from_coordinator_state_request(request: &CoordinatorStateRequest) -> &'static str {
        match request {
            CoordinatorStateRequest::EncrPk => "enc_pk",
            CoordinatorStateRequest::EncrSk => "enc_sk",
            CoordinatorStateRequest::SignPk => "sign_pk",
            CoordinatorStateRequest::SignSk => "sign_sk",
            CoordinatorStateRequest::Sum => "sum",
            CoordinatorStateRequest::Update => "update",
            CoordinatorStateRequest::Seed => "seed",
            CoordinatorStateRequest::MinSum => "min_sum",
            CoordinatorStateRequest::MinUpdate => "min_update",
            CoordinatorStateRequest::Phase => "phase",
            CoordinatorStateRequest::Round => "round",
        }
    }
}
