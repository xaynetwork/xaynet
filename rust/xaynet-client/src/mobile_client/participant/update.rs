use super::{Participant, ParticipantState};
use xaynet_core::{
    mask::{MaskObject, MaskSeed, Masker, Model},
    message::{Message, Update as UpdateMessage},
    CoordinatorPublicKey,
    LocalSeedDict,
    ParticipantTaskSignature,
    SumDict,
};
#[derive(Serialize, Deserialize, Clone)]
pub struct Update {
    sum_signature: ParticipantTaskSignature,
    update_signature: ParticipantTaskSignature,
}

impl Participant<Update> {
    pub fn new(
        state: ParticipantState,
        sum_signature: ParticipantTaskSignature,
        update_signature: ParticipantTaskSignature,
    ) -> Self {
        Self {
            inner: Update {
                sum_signature,
                update_signature,
            },
            state,
        }
    }

    /// Compose an update message given the coordinator public key, sum
    /// dictionary, model scalar and local model update.
    pub fn compose_update_message(
        &self,
        coordinator_pk: CoordinatorPublicKey,
        sum_dict: &SumDict,

        local_model: Model,
    ) -> Message {
        let (mask_seed, masked_model, masked_scalar) = self.mask_model(local_model);
        let local_seed_dict = Self::create_local_seed_dict(sum_dict, &mask_seed);
        let payload = UpdateMessage {
            sum_signature: self.inner.sum_signature,
            update_signature: self.inner.update_signature,
            masked_model,
            masked_scalar,
            local_seed_dict,
        };
        Message::new_update(self.state.keys.public, coordinator_pk, payload)
    }

    /// Generate a mask seed and mask a local model.
    fn mask_model(&self, local_model: Model) -> (MaskSeed, MaskObject, MaskObject) {
        Masker::new(self.state.aggregation_config.mask)
            .mask(self.state.aggregation_config.scalar, local_model)
    }

    // Create a local seed dictionary from a sum dictionary.
    fn create_local_seed_dict(sum_dict: &SumDict, mask_seed: &MaskSeed) -> LocalSeedDict {
        sum_dict
            .iter()
            .map(|(pk, ephm_pk)| (*pk, mask_seed.encrypt(ephm_pk)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sodiumoxide::randombytes::{randombytes, randombytes_uniform};
    use std::{collections::HashMap, iter};
    use xaynet_core::{
        crypto::{ByteObject, EncryptKeyPair},
        SumParticipantEphemeralPublicKey,
        SumParticipantEphemeralSecretKey,
        SumParticipantPublicKey,
    };

    #[test]
    fn test_create_local_seed_dict() {
        let mask_seed = MaskSeed::generate();
        let ephm_dict = iter::repeat_with(|| {
            let EncryptKeyPair { public, secret } = EncryptKeyPair::generate();
            (public, secret)
        })
        .take(1 + randombytes_uniform(10) as usize)
        .collect::<HashMap<SumParticipantEphemeralPublicKey, SumParticipantEphemeralSecretKey>>();
        let sum_dict = ephm_dict
            .iter()
            .map(|(ephm_pk, _)| {
                (
                    SumParticipantPublicKey::from_slice(&randombytes(32)).unwrap(),
                    *ephm_pk,
                )
            })
            .collect();
        let seed_dict = Participant::create_local_seed_dict(&sum_dict, &mask_seed);
        assert_eq!(seed_dict.keys().len(), sum_dict.keys().len());
        assert!(seed_dict.keys().all(|pk| sum_dict.contains_key(pk)));
        assert!(seed_dict.iter().all(|(pk, seed)| {
            let ephm_pk = sum_dict.get(pk).unwrap();
            let ephm_sk = ephm_dict.get(ephm_pk).unwrap();
            mask_seed == seed.decrypt(ephm_pk, ephm_sk).unwrap()
        }));
    }
}
