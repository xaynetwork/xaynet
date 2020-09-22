//! Provides the logic and functionality for a participant of the PET protocol.
//!
//! See the [client module] documentation since this is a private module anyways.
//!
//! [client module]: ../index.html

use std::default::Default;

use xaynet_core::{
    crypto::{ByteObject, EncryptKeyPair, SigningKeyPair},
    mask::{
        Aggregation,
        BoundType,
        DataType,
        GroupType,
        MaskConfig,
        MaskObject,
        MaskSeed,
        Masker,
        Model,
        ModelType,
    },
    message::{Message, Sum, Sum2, Update},
    CoordinatorPublicKey,
    InitError,
    LocalSeedDict,
    ParticipantPublicKey,
    ParticipantSecretKey,
    ParticipantTaskSignature,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantEphemeralSecretKey,
    UpdateSeedDict,
};

use crate::PetError;

#[derive(Debug, PartialEq, Copy, Clone)]
/// Tasks of a participant.
pub enum Task {
    Sum,
    Update,
    None,
}

#[derive(Debug)]
/// A participant in the PET protocol layer.
pub struct Participant {
    // credentials
    pub pk: ParticipantPublicKey,                // 32 bytes
    pub sk: ParticipantSecretKey,                // 64 bytes
    ephm_pk: SumParticipantEphemeralPublicKey,   // 32 bytes
    ephm_sk: SumParticipantEphemeralSecretKey,   // 32 bytes
    pub sum_signature: ParticipantTaskSignature, // 64 bytes
    update_signature: ParticipantTaskSignature,  // 64 bytes

    // round parameters
    pub task: Task,
}

impl Default for Participant {
    fn default() -> Self {
        let pk = ParticipantPublicKey::zeroed();
        let sk = ParticipantSecretKey::zeroed();
        let ephm_pk = SumParticipantEphemeralPublicKey::zeroed();
        let ephm_sk = SumParticipantEphemeralSecretKey::zeroed();
        let sum_signature = ParticipantTaskSignature::zeroed();
        let update_signature = ParticipantTaskSignature::zeroed();
        let task = Task::None;
        Self {
            pk,
            sk,
            ephm_pk,
            ephm_sk,
            sum_signature,
            update_signature,
            task,
        }
    }
}

impl Participant {
    /// Create a participant.
    ///
    /// # Errors
    /// Fails if there is insufficient system entropy to generate secrets.
    pub fn new() -> Result<Self, InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;
        let SigningKeyPair {
            public: pk,
            secret: sk,
        } = SigningKeyPair::generate();
        Ok(Self {
            pk,
            sk,
            ..Default::default()
        })
    }

    /// Compute the sum and update signatures for the given round seed.
    pub fn compute_signatures(&mut self, round_seed: &[u8]) {
        self.sum_signature = self.sk.sign_detached(&[round_seed, b"sum"].concat());
        self.update_signature = self.sk.sign_detached(&[round_seed, b"update"].concat());
    }

    /// Check eligibility for a task given probabilities for `Sum` and `Update`
    /// selection in this round.
    ///
    /// Returns the [`Task`] selected for this round.
    pub fn check_task(&mut self, round_sum: f64, round_update: f64) -> Task {
        if self.sum_signature.is_eligible(round_sum) {
            self.task = Task::Sum;
        } else if self.update_signature.is_eligible(round_update) {
            self.task = Task::Update;
        } else {
            self.task = Task::None;
        }
        self.task
    }

    /// Compose a sum message that embeds the given coordinator public
    /// key.
    pub fn compose_sum_message(&mut self, coordinator_pk: CoordinatorPublicKey) -> Message {
        self.gen_ephm_keypair();
        let payload = Sum {
            sum_signature: self.sum_signature,
            ephm_pk: self.ephm_pk,
        };
        Message::new_sum(self.pk, coordinator_pk, payload)
    }

    /// Compose an update message given the coordinator public key, sum
    /// dictionary, model scalar and local model update.
    pub fn compose_update_message(
        &self,
        coordinator_pk: CoordinatorPublicKey,
        sum_dict: &SumDict,
        scalar: f64,
        local_model: Model,
    ) -> Message {
        let (mask_seed, masked_model, masked_scalar) = Self::mask_model(scalar, local_model);
        let local_seed_dict = Self::create_local_seed_dict(sum_dict, &mask_seed);
        let payload = Update {
            sum_signature: self.sum_signature,
            update_signature: self.update_signature,
            masked_model,
            masked_scalar,
            local_seed_dict,
        };
        Message::new_update(self.pk, coordinator_pk, payload)
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
        coordinator_pk: CoordinatorPublicKey,
        seed_dict: &UpdateSeedDict,
        mask_len: usize,
    ) -> Result<Message, PetError> {
        let mask_seeds = self.get_seeds(seed_dict)?;
        let (model_mask, scalar_mask) =
            self.compute_global_mask(mask_seeds, mask_len, dummy_config())?;
        let payload = Sum2 {
            sum_signature: self.sum_signature,
            model_mask,
            scalar_mask,
        };
        Ok(Message::new_sum2(self.pk, coordinator_pk, payload))
    }

    /// Sign the given message with the participant secret key, and
    /// encrypt the signed message with the given public key.
    pub fn seal_message(&self, pk: &CoordinatorPublicKey, message: &Message) -> Vec<u8> {
        let mut buf = vec![0; message.buffer_length()];
        message.to_bytes(&mut buf, &self.sk);
        pk.encrypt(&buf[..])
    }

    /// Generate an ephemeral encryption key pair.
    fn gen_ephm_keypair(&mut self) {
        let EncryptKeyPair { public, secret } = EncryptKeyPair::generate();
        self.ephm_pk = public;
        self.ephm_sk = secret;
    }

    /// Generate a mask seed and mask a local model.
    fn mask_model(scalar: f64, local_model: Model) -> (MaskSeed, MaskObject, MaskObject) {
        // TODO: use proper config
        Masker::new(dummy_config()).mask(scalar, local_model)
    }

    // Create a local seed dictionary from a sum dictionary.
    fn create_local_seed_dict(sum_dict: &SumDict, mask_seed: &MaskSeed) -> LocalSeedDict {
        sum_dict
            .iter()
            .map(|(pk, ephm_pk)| (*pk, mask_seed.encrypt(ephm_pk)))
            .collect()
    }

    /// Get the mask seeds from the local seed dictionary.
    fn get_seeds(&self, seed_dict: &UpdateSeedDict) -> Result<Vec<MaskSeed>, PetError> {
        seed_dict
            .values()
            .map(|seed| {
                seed.decrypt(&self.ephm_pk, &self.ephm_sk)
                    .map_err(|_| PetError::InvalidMask)
            })
            .collect()
    }

    /// Compute a global mask from local mask seeds.
    fn compute_global_mask(
        &self,
        mask_seeds: Vec<MaskSeed>,
        mask_len: usize,
        mask_config: MaskConfig,
    ) -> Result<(MaskObject, MaskObject), PetError> {
        if mask_seeds.is_empty() {
            return Err(PetError::InvalidMask);
        }

        let mut model_mask_agg = Aggregation::new(mask_config, mask_len);
        let mut scalar_mask_agg = Aggregation::new(mask_config, 1);
        for seed in mask_seeds.into_iter() {
            let (model_mask, scalar_mask) = seed.derive_mask(mask_len, mask_config);

            model_mask_agg
                .validate_aggregation(&model_mask)
                .map_err(|_| PetError::InvalidMask)?;
            scalar_mask_agg
                .validate_aggregation(&scalar_mask)
                .map_err(|_| PetError::InvalidMask)?;

            model_mask_agg.aggregate(model_mask);
            scalar_mask_agg.aggregate(scalar_mask);
        }
        Ok((model_mask_agg.into(), scalar_mask_agg.into()))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        iter,
    };

    use sodiumoxide::randombytes::{randombytes, randombytes_uniform};

    use super::*;
    use xaynet_core::{crypto::Signature, SumParticipantPublicKey, UpdateParticipantPublicKey};

    #[test]
    fn test_participant() {
        let part = Participant::new().unwrap();
        assert_eq!(part.pk, part.sk.public_key());
        assert_eq!(part.sk.as_slice().len(), 64);
        assert_eq!(part.ephm_pk, SumParticipantEphemeralPublicKey::zeroed());
        assert_eq!(part.ephm_sk, SumParticipantEphemeralSecretKey::zeroed());
        assert_eq!(part.sum_signature, ParticipantTaskSignature::zeroed());
        assert_eq!(part.update_signature, ParticipantTaskSignature::zeroed());
        assert_eq!(part.task, Task::None);
    }

    #[test]
    fn test_compute_signature() {
        let mut part = Participant::new().unwrap();
        let round_seed = randombytes(32);
        part.compute_signatures(&round_seed);
        assert!(part.pk.verify_detached(
            &part.sum_signature,
            &[round_seed.as_slice(), b"sum"].concat(),
        ));
        assert!(part.pk.verify_detached(
            &part.update_signature,
            &[round_seed.as_slice(), b"update"].concat(),
        ));
    }

    #[test]
    fn test_check_task() {
        let mut part = Participant::new().unwrap();
        let eligible_signature = Signature::from_slice_unchecked(&[
            172, 29, 85, 219, 118, 44, 107, 32, 219, 253, 25, 242, 53, 45, 111, 62, 102, 130, 24,
            8, 222, 199, 34, 120, 166, 163, 223, 229, 100, 50, 252, 244, 250, 88, 196, 151, 136,
            48, 39, 198, 166, 86, 29, 151, 13, 81, 69, 198, 40, 148, 134, 126, 7, 202, 1, 56, 174,
            43, 89, 28, 242, 194, 4, 214,
        ]);
        let ineligible_signature = Signature::from_slice_unchecked(&[
            119, 2, 197, 174, 52, 165, 229, 22, 218, 210, 240, 188, 220, 232, 149, 129, 211, 13,
            61, 217, 186, 79, 102, 15, 109, 237, 83, 193, 12, 117, 210, 66, 99, 230, 30, 131, 63,
            108, 28, 222, 48, 92, 153, 71, 159, 220, 115, 181, 183, 155, 146, 182, 205, 89, 140,
            234, 100, 40, 199, 248, 23, 147, 172, 248,
        ]);
        part.sum_signature = eligible_signature;
        part.update_signature = ineligible_signature;
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::Sum);
        part.update_signature = eligible_signature;
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::Sum);
        part.sum_signature = ineligible_signature;
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::Update);
        part.update_signature = ineligible_signature;
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::None);
    }

    #[test]
    fn test_gen_ephm_keypair() {
        let mut part = Participant::new().unwrap();
        part.gen_ephm_keypair();
        assert_eq!(part.ephm_pk, part.ephm_sk.public_key());
        assert_eq!(part.ephm_sk.as_slice().len(), 32);
    }

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

    #[test]
    fn test_get_seeds() {
        let mut part = Participant::new().unwrap();
        part.gen_ephm_keypair();
        let mask_seeds: Vec<MaskSeed> = iter::repeat_with(MaskSeed::generate)
            .take(1 + randombytes_uniform(10) as usize)
            .collect::<Vec<_>>();
        let upd_seed_dict = mask_seeds
            .iter()
            .map(|seed| {
                (
                    UpdateParticipantPublicKey::from_slice(&randombytes(32)).unwrap(),
                    seed.encrypt(&part.ephm_pk),
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

fn dummy_config() -> MaskConfig {
    MaskConfig {
        group_type: GroupType::Prime,
        data_type: DataType::F32,
        bound_type: BoundType::B0,
        model_type: ModelType::M3,
    }
}
