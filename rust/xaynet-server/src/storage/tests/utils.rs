use num::{bigint::BigUint, traits::identities::Zero};

use crate::{
    state_machine::tests::utils::mask_settings,
    storage::{CoordinatorStorage, LocalSeedDictAdd},
};
use xaynet_core::{
    crypto::{ByteObject, EncryptKeyPair, SigningKeyPair},
    mask::{EncryptedMaskSeed, MaskConfig, MaskObject},
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
    sum_pks: &[SumParticipantPublicKey],
) -> Vec<(UpdateParticipantPublicKey, LocalSeedDict)> {
    let mut entries = Vec::new();

    for _ in 0..sum_pks.len() {
        let SigningKeyPair {
            public: update_pk, ..
        } = SigningKeyPair::generate();

        let mut local_seed_dict = LocalSeedDict::new();
        for sum_pk in sum_pks {
            let seed = EncryptedMaskSeed::zeroed();
            local_seed_dict.insert(*sum_pk, seed);
        }
        entries.push((update_pk, local_seed_dict))
    }

    entries
}

pub fn create_mask_zeroed(byte_size: usize) -> MaskObject {
    MaskObject::new(
        MaskConfig::from(mask_settings()).into(),
        vec![BigUint::zero(); byte_size],
        BigUint::zero(),
    )
    .unwrap()
}

pub fn create_mask(byte_size: usize, number: u32) -> MaskObject {
    MaskObject::new(
        MaskConfig::from(mask_settings()).into(),
        vec![BigUint::from(number); byte_size],
        BigUint::zero(),
    )
    .unwrap()
}

pub fn create_seed_dict(
    sum_dict: SumDict,
    seed_updates: &[(UpdateParticipantPublicKey, LocalSeedDict)],
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

pub async fn create_and_add_sum_participant_entries(
    client: &mut impl CoordinatorStorage,
    n: u32,
) -> Vec<SumParticipantPublicKey> {
    let mut sum_pks = Vec::new();
    for _ in 0..n {
        let (pk, ephm_pk) = create_sum_participant_entry();

        let _ = client.add_sum_participant(&pk, &ephm_pk).await.unwrap();
        sum_pks.push(pk);
    }

    sum_pks
}

pub async fn add_local_seed_entries(
    client: &mut impl CoordinatorStorage,
    local_seed_entries: &[(UpdateParticipantPublicKey, LocalSeedDict)],
) -> Vec<LocalSeedDictAdd> {
    let mut update_result = Vec::new();

    for (update_pk, local_seed_dict) in local_seed_entries {
        let res = client
            .add_local_seed_dict(&update_pk, &local_seed_dict)
            .await;
        assert!(res.is_ok());
        update_result.push(res.unwrap())
    }

    update_result
}

#[cfg(feature = "model-persistence")]
use xaynet_core::mask::{FromPrimitives, Model};

#[cfg(feature = "model-persistence")]
pub fn create_global_model(nb_elements: usize) -> Model {
    Model::from_primitives(vec![0; nb_elements].into_iter()).unwrap()
}
