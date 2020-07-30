use super::{Participant, ParticipantState};
use crate::{
    mask::{masking::Aggregation, object::MaskObject, seed::MaskSeed},
    message::{message::MessageOwned, payload::sum2::Sum2Owned},
    CoordinatorPublicKey,
    ParticipantPublicKey,
    ParticipantTaskSignature,
    PetError,
    SumParticipantEphemeralPublicKey,
    SumParticipantEphemeralSecretKey,
    UpdateSeedDict,
};
pub struct Sum2 {
    ephm_pk: SumParticipantEphemeralPublicKey,
    ephm_sk: SumParticipantEphemeralSecretKey,
    sum_signature: ParticipantTaskSignature,
}

impl Participant<Sum2> {
    pub fn new(
        state: ParticipantState,
        sum_signature: ParticipantTaskSignature,
        ephm_pk: SumParticipantEphemeralPublicKey,
        ephm_sk: SumParticipantEphemeralSecretKey,
    ) -> Self {
        Self {
            inner: Sum2 {
                sum_signature,
                ephm_pk,
                ephm_sk,
            },
            state,
        }
    }

    /// Compose a sum2 message given the coordinator public key, seed dictionary
    /// and mask length.
    ///
    /// # Errors
    ///
    /// Returns a [`PetError`] if there is a problem extracting the
    /// seed dictionary, or computing the global mask.
    pub fn compose_sum2_message(
        &self,
        pk: CoordinatorPublicKey,
        seed_dict: &UpdateSeedDict,
        mask_len: usize,
    ) -> Result<MessageOwned, PetError> {
        let mask_seeds = self.get_seeds(seed_dict)?;
        let mask = self.compute_global_mask(mask_seeds, mask_len)?;
        let payload = Sum2Owned {
            mask,
            sum_signature: self.inner.sum_signature,
        };

        Ok(MessageOwned::new_sum2(pk, self.state.keys.public, payload))
    }

    pub fn get_participant_pk(&self) -> ParticipantPublicKey {
        self.state.keys.public
    }

    /// Get the mask seeds from the local seed dictionary.
    fn get_seeds(&self, seed_dict: &UpdateSeedDict) -> Result<Vec<MaskSeed>, PetError> {
        seed_dict
            .values()
            .map(|seed| seed.decrypt(&self.inner.ephm_pk, &self.inner.ephm_sk))
            .collect()
    }

    /// Compute a global mask from local mask seeds.
    fn compute_global_mask(
        &self,
        mask_seeds: Vec<MaskSeed>,
        mask_len: usize,
    ) -> Result<MaskObject, PetError> {
        if mask_seeds.is_empty() {
            return Err(PetError::InvalidMask);
        }

        let mut aggregation = Aggregation::new(self.state.mask_config, mask_len);
        for seed in mask_seeds.into_iter() {
            let mask = seed.derive_mask(mask_len, self.state.mask_config);
            aggregation
                .validate_aggregation(&mask)
                .map_err(|_| PetError::InvalidMask)?;
            aggregation.aggregate(mask);
        }
        Ok(aggregation.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        certificate::Certificate,
        crypto::{encrypt::EncryptKeyPair, sign::SigningKeyPair, ByteObject, Signature},
        mask::config::{BoundType, DataType, GroupType, MaskConfig, ModelType},
        UpdateParticipantPublicKey,
    };
    use sodiumoxide::randombytes::{randombytes, randombytes_uniform};
    use std::{collections::HashSet, iter};

    fn participant_state() -> ParticipantState {
        sodiumoxide::init().unwrap();

        let certificate = Certificate::new();
        let mask_config = MaskConfig {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };

        ParticipantState {
            keys: SigningKeyPair::generate(),
            certificate,
            mask_config,
        }
    }

    #[test]
    fn test_get_seeds() {
        let EncryptKeyPair { public, secret } = EncryptKeyPair::generate();
        let part =
            Participant::<Sum2>::new(participant_state(), Signature::zeroed(), public, secret);
        let mask_seeds: Vec<MaskSeed> = iter::repeat_with(MaskSeed::generate)
            .take(1 + randombytes_uniform(10) as usize)
            .collect::<Vec<_>>();
        let upd_seed_dict = mask_seeds
            .iter()
            .map(|seed| {
                (
                    UpdateParticipantPublicKey::from_slice(&randombytes(32)).unwrap(),
                    seed.encrypt(&part.inner.ephm_pk),
                )
            })
            .collect();
        assert_eq!(
            part.get_seeds(&upd_seed_dict)
                .unwrap()
                .into_iter()
                .map(|seed| seed.as_array())
                .collect::<HashSet<_>>(),
            mask_seeds
                .into_iter()
                .map(|seed| seed.as_array())
                .collect::<HashSet<_>>(),
        );
    }
}
