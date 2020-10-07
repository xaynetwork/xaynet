use crate::storage::{impls::SeedDictUpdate, redis::Client};
use num::{bigint::BigUint, traits::identities::Zero};
use xaynet_core::{
    crypto::{ByteObject, EncryptKeyPair, SigningKeyPair},
    mask::{BoundType, DataType, EncryptedMaskSeed, GroupType, MaskConfig, MaskObject, ModelType},
    LocalSeedDict,
    SeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

pub fn create_sum_participant_entry() -> (SumParticipantPublicKey, SumParticipantEphemeralPublicKey)
{
    let SigningKeyPair { public: pk, .. } = SigningKeyPair::generate();
    let EncryptKeyPair {
        public: ephm_pk, ..
    } = EncryptKeyPair::generate();
    (pk, ephm_pk)
}

pub fn create_local_seed_entries(
    sum_pks: &Vec<SumParticipantPublicKey>,
) -> Vec<(UpdateParticipantPublicKey, LocalSeedDict)> {
    let mut entries = Vec::new();

    for _ in 0..sum_pks.len() {
        let SigningKeyPair {
            public: update_pk, ..
        } = SigningKeyPair::generate();

        let mut local_seed_dict = LocalSeedDict::new();
        for sum_pk in sum_pks {
            let seed = EncryptedMaskSeed::zeroed();
            local_seed_dict.insert(sum_pk.clone(), seed);
        }
        entries.push((update_pk, local_seed_dict))
    }

    entries
}

pub fn create_mask(byte_size: usize) -> MaskObject {
    let config = MaskConfig {
        group_type: GroupType::Prime,
        data_type: DataType::F32,
        bound_type: BoundType::B0,
        model_type: ModelType::M3,
    };

    MaskObject::new_checked(
        config.clone(),
        vec![BigUint::zero(); byte_size],
        config,
        BigUint::zero(),
    )
    .unwrap()
}

pub fn create_seed_dict(
    sum_dict: SumDict,
    seed_updates: &Vec<(UpdateParticipantPublicKey, LocalSeedDict)>,
) -> SeedDict {
    let mut seed_dict: SeedDict = sum_dict
        .keys()
        .map(|pk| (*pk, LocalSeedDict::new()))
        .collect();

    for (pk, local_seed_dict) in seed_updates {
        for (sum_pk, seed) in local_seed_dict {
            seed_dict.get_mut(sum_pk).unwrap().insert(*pk, seed.clone());
        }
    }

    seed_dict
}

pub async fn create_and_write_sum_participant_entries(
    client: &Client,
    n: u32,
) -> Vec<SumParticipantPublicKey> {
    let mut sum_pks = Vec::new();
    for _ in 0..n {
        let (pk, ephm_pk) = create_sum_participant_entry();

        let _ = client
            .connection()
            .await
            .add_sum_participant(&pk, &ephm_pk)
            .await
            .unwrap();
        // assert_eq!(add_new_key, AddSumParticipant::Ok);
        sum_pks.push(pk);
    }

    sum_pks
}

pub async fn write_local_seed_entries(
    client: &Client,
    local_seed_entries: &Vec<(UpdateParticipantPublicKey, LocalSeedDict)>,
) -> Vec<SeedDictUpdate> {
    let mut update_result = Vec::new();

    for (update_pk, local_seed_dict) in local_seed_entries {
        let res = client
            .connection()
            .await
            .update_seed_dict(&update_pk, &local_seed_dict)
            .await;
        assert!(res.is_ok());
        update_result.push(res.unwrap())
    }

    update_result
}
