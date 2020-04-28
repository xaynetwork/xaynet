use std::{convert::TryFrom, default::Default};

use sodiumoxide;

use crate::{
    certificate::Certificate,
    crypto::{generate_encrypt_key_pair, generate_signing_key_pair, ByteObject},
    mask::{
        config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
        seed::MaskSeed,
        Mask,
        Model,
    },
    message::{sum::SumMessage, sum2::Sum2Message, update::UpdateMessage},
    utils::is_eligible,
    CoordinatorPublicKey,
    InitError,
    LocalSeedDict,
    ParticipantPublicKey,
    ParticipantSecretKey,
    ParticipantTaskSignature,
    PetError,
    SeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantEphemeralSecretKey,
};

#[derive(Debug, PartialEq)]
/// Tasks of a participant.
enum Task {
    Sum,
    Update,
    None,
}

/// A participant in the PET protocol layer.
pub struct Participant {
    // credentials
    pk: ParticipantPublicKey,                   // 32 bytes
    sk: ParticipantSecretKey,                   // 64 bytes
    ephm_pk: SumParticipantEphemeralPublicKey,  // 32 bytes
    ephm_sk: SumParticipantEphemeralSecretKey,  // 32 bytes
    certificate: Certificate,                   // 0 bytes (dummy)
    sum_signature: ParticipantTaskSignature,    // 64 bytes
    update_signature: ParticipantTaskSignature, // 64 bytes

    // round parameters
    task: Task,
}

impl Default for Participant {
    fn default() -> Self {
        let pk = ParticipantPublicKey::zeroed();
        let sk = ParticipantSecretKey::zeroed();
        let ephm_pk = SumParticipantEphemeralPublicKey::zeroed();
        let ephm_sk = SumParticipantEphemeralSecretKey::zeroed();
        let certificate = Certificate::zeroed();
        let sum_signature = ParticipantTaskSignature::zeroed();
        let update_signature = ParticipantTaskSignature::zeroed();
        let task = Task::None;
        Self {
            pk,
            sk,
            ephm_pk,
            ephm_sk,
            certificate,
            sum_signature,
            update_signature,
            task,
        }
    }
}

impl Participant {
    /// Create a participant. Fails if there is insufficient system entropy to generate secrets.
    pub fn new() -> Result<Self, InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;
        let (pk, sk) = generate_signing_key_pair();
        Ok(Self {
            pk,
            sk,
            ..Default::default()
        })
    }

    /// Compute the sum and update signatures.
    pub fn compute_signatures(&mut self, round_seed: &[u8]) {
        self.sum_signature = self.sk.sign_detached(&[round_seed, b"sum"].concat());
        self.update_signature = self.sk.sign_detached(&[round_seed, b"update"].concat());
    }

    /// Check eligibility for a task.
    pub fn check_task(&mut self, round_sum: f64, round_update: f64) {
        if is_eligible(&self.sum_signature, round_sum) {
            self.task = Task::Sum;
        } else if is_eligible(&self.update_signature, round_update) {
            self.task = Task::Update;
        } else {
            self.task = Task::None;
        }
    }

    /// Compose a sum message.
    pub fn compose_sum_message(&mut self, pk: &CoordinatorPublicKey) -> Vec<u8> {
        self.gen_ephm_keypair();
        SumMessage::from_parts(
            &self.pk,
            &self.sum_signature,
            &self.ephm_pk,
            &self.certificate,
        )
        .seal(&self.sk, pk)
    }

    /// Compose an update message.
    pub fn compose_update_message(&self, pk: &CoordinatorPublicKey, sum_dict: &SumDict) -> Vec<u8> {
        let model = Model::try_from(vec![0_f32, 0.5, -0.5]).unwrap(); // dummy
        let scalar = 0.5_f64; // dummy
        let mask_config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        let (mask_seed, masked_model) = model.mask(scalar, &mask_config);
        let local_seed_dict = Self::create_local_seed_dict(sum_dict, &mask_seed);
        UpdateMessage::from_parts(
            &self.pk,
            &self.sum_signature,
            &self.update_signature,
            &self.certificate,
            &masked_model,
            &local_seed_dict,
        )
        .seal(&self.sk, pk)
    }

    /// Compose a sum2 message.
    pub fn compose_sum2_message(
        &self,
        pk: &CoordinatorPublicKey,
        seed_dict: &SeedDict,
    ) -> Result<Vec<u8>, PetError> {
        let mask_seeds = self.get_seeds(seed_dict)?;
        let mask = self.compute_global_mask(mask_seeds);
        Ok(
            Sum2Message::from_parts(&self.pk, &self.sum_signature, &self.certificate, &mask)
                .seal(&self.sk, pk),
        )
    }

    /// Generate an ephemeral encryption key pair.
    fn gen_ephm_keypair(&mut self) {
        let (ephm_pk, ephm_sk) = generate_encrypt_key_pair();
        self.ephm_pk = ephm_pk;
        self.ephm_sk = ephm_sk;
    }

    // Create a local seed dictionary from a sum dictionary.
    fn create_local_seed_dict(sum_dict: &SumDict, mask_seed: &MaskSeed) -> LocalSeedDict {
        sum_dict
            .iter()
            .map(|(pk, ephm_pk)| (*pk, mask_seed.encrypt(ephm_pk)))
            .collect()
    }

    /// Get the mask seeds from the seed dictionary.
    fn get_seeds(&self, seed_dict: &SeedDict) -> Result<Vec<MaskSeed>, PetError> {
        seed_dict
            .get(&self.pk)
            .ok_or(PetError::InvalidMessage)?
            .values()
            .map(|seed| seed.decrypt(&self.ephm_pk, &self.ephm_sk))
            .collect()
    }

    /// Compute a global mask from local mask seeds (dummy).
    fn compute_global_mask(&self, _mask_seeds: Vec<MaskSeed>) -> Mask {
        Mask::deserialize(Vec::<u8>::new().as_slice()).unwrap() // dummy
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
    use crate::{crypto::Signature, SumParticipantPublicKey, UpdateParticipantPublicKey};

    #[test]
    fn test_participant() {
        let part = Participant::new().unwrap();
        assert_eq!(part.pk, part.sk.public_key());
        assert_eq!(part.sk.as_slice().len(), 64);
        assert_eq!(part.ephm_pk, SumParticipantEphemeralPublicKey::zeroed());
        assert_eq!(part.ephm_sk, SumParticipantEphemeralSecretKey::zeroed());
        assert_eq!(part.certificate, Certificate::zeroed());
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
        let ephm_dict = iter::repeat_with(|| generate_encrypt_key_pair())
            .take(1 + randombytes_uniform(10) as usize)
            .collect::<HashMap<SumParticipantEphemeralPublicKey, SumParticipantEphemeralSecretKey>>(
            );
        let sum_dict = ephm_dict
            .iter()
            .map(|(ephm_pk, _)| {
                (
                    SumParticipantPublicKey::from_slice_unchecked(&randombytes(32)),
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
        let mask_seeds = iter::repeat_with(|| MaskSeed::generate())
            .take(1 + randombytes_uniform(10) as usize)
            .collect::<Vec<_>>();
        let seed_dict = [(
            part.pk,
            mask_seeds
                .iter()
                .map(|seed| {
                    (
                        UpdateParticipantPublicKey::from_slice_unchecked(&randombytes(32)),
                        seed.encrypt(&part.ephm_pk),
                    )
                })
                .collect(),
        )]
        .iter()
        .cloned()
        .collect();
        assert_eq!(
            part.get_seeds(&seed_dict)
                .unwrap()
                .into_iter()
                .map(|seed| seed.as_array())
                .collect::<HashSet<_>>(),
            mask_seeds
                .into_iter()
                .map(|seed| seed.as_array())
                .collect::<HashSet<_>>(),
        );
        assert_eq!(
            part.get_seeds(&SeedDict::new()).unwrap_err(),
            PetError::InvalidMessage,
        );
    }
}
