use std::{default::Default, iter};

use counter::Counter;
use sodiumoxide::{self, crypto::hash::sha256, randombytes::randombytes};

use crate::{
    crypto::{generate_encrypt_key_pair, ByteObject, SigningKeySeed},
    mask::Mask,
    message::{sum::SumMessage, sum2::Sum2Message, update::UpdateMessage},
    utils::is_eligible,
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    LocalSeedDict,
    ParticipantPublicKey,
    ParticipantTaskSignature,
    PetError,
    SeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};

/// A 32-byte hash that identifies a model mask computed by a sum participant.
pub type MaskHash = sha256::Digest;

/// A dictionary created during the sum2 phase of the protocol. It counts the model masks
/// represented by their hashes.
pub type MaskDict = Counter<MaskHash>;

#[derive(Debug, PartialEq, Copy, Clone)]
/// Round phases of a coordinator.
pub enum Phase {
    Idle,
    Sum,
    Update,
    Sum2,
}

/// A coordinator in the PET protocol layer.
pub struct Coordinator {
    // credentials
    pk: CoordinatorPublicKey, // 32 bytes
    sk: CoordinatorSecretKey, // 32 bytes

    // round parameters
    sum: f64,
    update: f64,
    seed: Vec<u8>, // 32 bytes
    min_sum: usize,
    min_update: usize,
    phase: Phase,

    // round dictionaries
    /// Dictionary built during the sum phase.
    sum_dict: SumDict,
    /// Dictionary built during the update phase.
    seed_dict: SeedDict,
    /// Dictionary built during the sum2 phase.
    mask_dict: MaskDict,
}

impl Default for Coordinator {
    fn default() -> Self {
        let pk = CoordinatorPublicKey::zeroed();
        let sk = CoordinatorSecretKey::zeroed();
        let sum = 0.01_f64;
        let update = 0.1_f64;
        let seed = vec![0_u8; 32];
        let min_sum = 1_usize;
        let min_update = 3_usize;
        let phase = Phase::Idle;
        let sum_dict = SumDict::new();
        let seed_dict = SeedDict::new();
        let mask_dict = MaskDict::new();
        Self {
            pk,
            sk,
            sum,
            update,
            seed,
            min_sum,
            min_update,
            phase,
            sum_dict,
            seed_dict,
            mask_dict,
        }
    }
}

impl Coordinator {
    /// Create a coordinator. Fails if there is insufficient system entropy to generate secrets.
    pub fn new() -> Result<Self, PetError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(PetError::InsufficientSystemEntropy))?;
        let seed = randombytes(32);
        Ok(Self {
            seed,
            ..Default::default()
        })
    }

    /// Validate and handle a sum, update or sum2 message.
    pub fn handle_message(&mut self, bytes: &[u8]) -> Result<(), PetError> {
        match self.phase {
            Phase::Idle => Err(PetError::InvalidMessage),
            Phase::Sum => self.handle_sum_message(bytes),
            Phase::Update => self.handle_update_message(bytes),
            Phase::Sum2 => self.handle_sum2_message(bytes),
        }
    }

    /// Validate and handle a sum message.
    fn handle_sum_message(&mut self, bytes: &[u8]) -> Result<(), PetError> {
        let msg = SumMessage::open(bytes, &self.pk, &self.sk)?;
        msg.certificate().validate()?;
        self.validate_sum_task(msg.sum_signature(), msg.pk())?;
        self.add_sum_participant(msg.pk(), msg.ephm_pk());
        Ok(())
    }

    /// Validate and handle an update message.
    fn handle_update_message(&mut self, bytes: &[u8]) -> Result<(), PetError> {
        let msg = UpdateMessage::open(bytes, &self.pk, &self.sk)?;
        msg.certificate().validate()?;
        self.validate_update_task(msg.sum_signature(), msg.update_signature(), msg.pk())?;
        self.add_local_seed_dict(msg.pk(), msg.local_seed_dict())?;
        Ok(())
    }

    /// Validate and handle a sum2 message.
    fn handle_sum2_message(&mut self, bytes: &[u8]) -> Result<(), PetError> {
        let msg = Sum2Message::open(bytes, &self.pk, &self.sk)?;
        if !self.sum_dict.contains_key(msg.pk()) {
            return Err(PetError::InvalidMessage);
        }
        msg.certificate().validate()?;
        self.validate_sum_task(msg.sum_signature(), msg.pk())?;
        self.add_mask_hash(msg.mask());
        Ok(())
    }

    /// Validate a sum signature and its implied task.
    fn validate_sum_task(
        &self,
        sum_signature: &ParticipantTaskSignature,
        pk: &SumParticipantPublicKey,
    ) -> Result<(), PetError> {
        if pk.verify_detached(sum_signature, &[self.seed.as_slice(), b"sum"].concat())
            && is_eligible(sum_signature, self.sum)
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Validate an update signature and its implied task.
    fn validate_update_task(
        &self,
        sum_signature: &ParticipantTaskSignature,
        update_signature: &ParticipantTaskSignature,
        pk: &UpdateParticipantPublicKey,
    ) -> Result<(), PetError> {
        if pk.verify_detached(sum_signature, &[self.seed.as_slice(), b"sum"].concat())
            && pk.verify_detached(
                update_signature,
                &[self.seed.as_slice(), b"update"].concat(),
            )
            && !is_eligible(sum_signature, self.sum)
            && is_eligible(update_signature, self.update)
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Add a sum participant to the sum dictionary.
    fn add_sum_participant(
        &mut self,
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
    ) {
        self.sum_dict.insert(*pk, *ephm_pk);
    }

    /// Freeze the sum dictionary.
    fn freeze_sum_dict(&mut self) {
        self.seed_dict = self
            .sum_dict
            .keys()
            .map(|pk| (*pk, LocalSeedDict::new()))
            .collect();
    }

    /// Add a local seed dictionary to the seed dictionary. Fails if it contains invalid keys.
    fn add_local_seed_dict(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> Result<(), PetError> {
        if local_seed_dict.keys().len() == self.sum_dict.keys().len()
            && local_seed_dict
                .keys()
                .all(|pk| self.sum_dict.contains_key(pk))
        {
            for (sum_pk, seed) in local_seed_dict {
                // safe unwrap: existence of `sum_pk` is guaranteed by `freeze_sum_dict()`
                self.seed_dict
                    .get_mut(sum_pk)
                    .unwrap()
                    .insert(*pk, seed.clone());
            }
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Add a hashed mask to the mask dictionary.
    fn add_mask_hash(&mut self, mask: &Mask) {
        let mask_hash = sha256::hash(mask.as_ref());
        self.mask_dict.update(iter::once(mask_hash));
    }

    /// Freeze the mask dictionary.
    fn freeze_mask_dict(&self) -> Result<MaskHash, PetError> {
        let counts = self.mask_dict.most_common();
        if counts.len() == 1 || counts[0].1 > counts[1].1 {
            Ok(counts[0].0)
        } else {
            Err(PetError::AmbiguousMasks)
        }
    }

    /// Clear the round dictionaries.
    fn clear_round_dicts(&mut self) {
        self.sum_dict.clear();
        self.sum_dict.shrink_to_fit();
        self.seed_dict.clear();
        self.seed_dict.shrink_to_fit();
        self.mask_dict.clear();
        self.mask_dict.shrink_to_fit();
    }

    /// Generate fresh round credentials.
    fn gen_round_keypair(&mut self) {
        let (pk, sk) = generate_encrypt_key_pair();
        self.pk = pk;
        self.sk = sk;
    }

    /// Update the round threshold parameters (dummy).
    fn update_round_thresholds(&mut self) {}

    /// Update the seed round parameter.
    fn update_round_seed(&mut self) {
        // safe unwrap: `sk` and `seed` have same number of bytes
        let (_, sk) = SigningKeySeed::from_slice(self.sk.as_slice())
            .unwrap()
            .derive_signing_key_pair();
        let signature = sk.sign_detached(
            &[
                self.seed.as_slice(),
                &self.sum.to_le_bytes(),
                &self.update.to_le_bytes(),
            ]
            .concat(),
        );
        self.seed = sha256::hash(signature.as_slice()).as_ref().to_vec();
    }

    /// Transition to the next phase if the protocol conditions are satisfied.
    pub fn try_phase_transition(&mut self) {
        match self.phase {
            Phase::Idle => self.proceed_sum_phase(),
            Phase::Sum => {
                if self.has_enough_sums() {
                    self.proceed_update_phase();
                }
            }
            Phase::Update => {
                if self.has_enough_seeds() {
                    self.proceed_sum2_phase();
                }
            }
            Phase::Sum2 => {
                if self.has_enough_masks() {
                    self.proceed_idle_phase();
                }
            }
        }
    }

    /// Check whether enough sum participants submitted their ephemeral keys to start the update
    /// phase.
    fn has_enough_sums(&self) -> bool {
        self.sum_dict.len() >= self.min_sum
    }

    /// Check whether enough update participants submitted their models and seeds to start the sum2
    /// phase.
    fn has_enough_seeds(&self) -> bool {
        self.seed_dict
            .values()
            .next()
            .map(|dict| dict.len() >= self.min_update)
            .unwrap_or(false)
    }

    /// Check whether enough sum participants submitted their masks to start the idle phase.
    fn has_enough_masks(&self) -> bool {
        let mask_count = self
            .mask_dict
            .most_common()
            .iter()
            .map(|(_, count)| count)
            .sum::<usize>();
        mask_count >= self.min_sum
    }

    /// End the idle phase and proceed to the sum phase to start the round.
    fn proceed_sum_phase(&mut self) {
        self.gen_round_keypair();
        self.phase = Phase::Sum;
    }

    /// End the sum phase and proceed to the update phase.
    fn proceed_update_phase(&mut self) {
        self.freeze_sum_dict();
        self.phase = Phase::Update;
    }

    /// End the update phase and proceed to the sum2 phase.
    fn proceed_sum2_phase(&mut self) {
        self.phase = Phase::Sum2;
    }

    /// End the sum2 phase and proceed to the idle phase to end the round.
    fn proceed_idle_phase(&mut self) {
        match self.freeze_mask_dict() {
            Ok(_mask_hash) => {
                info!("round finished successfully");
            }
            Err(_) => {
                error!("round failed");
            }
        }
        self.clear_round_dicts();
        self.update_round_thresholds();
        self.update_round_seed();
        self.phase = Phase::Idle;
    }

    pub fn round_parameters(&self) -> RoundParameters {
        RoundParameters {
            pk: self.pk,
            sum: self.sum,
            update: self.update,
            seed: self.seed.clone(),
        }
    }
}

pub struct RoundParameters {
    /// The coordinator public key for encryption.
    pub pk: CoordinatorPublicKey,

    /// Fraction of participants to be selected for the sum task.
    pub sum: f64,

    /// Fraction of participants to be selected for the update task.
    pub update: f64,

    /// The random round seed.
    pub seed: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{crypto::*, mask::MaskSeed};

    #[test]
    fn test_coordinator() {
        let coord = Coordinator::new().unwrap();
        assert_eq!(coord.pk, PublicEncryptKey::zeroed());
        assert_eq!(coord.sk, SecretEncryptKey::zeroed());
        assert!(coord.sum >= 0. && coord.sum <= 1.);
        assert!(coord.update >= 0. && coord.update <= 1.);
        assert_eq!(coord.seed.len(), 32);
        assert!(coord.min_sum >= 1);
        assert!(coord.min_update >= 3);
        assert_eq!(coord.phase, Phase::Idle);
        assert_eq!(coord.sum_dict, SumDict::new());
        assert_eq!(coord.seed_dict, SeedDict::new());
        assert_eq!(coord.mask_dict, MaskDict::new());
    }

    #[test]
    fn test_validate_sum_task() {
        let mut coord = Coordinator::new().unwrap();
        coord.seed = vec![
            229, 16, 164, 40, 138, 161, 23, 161, 175, 102, 13, 103, 229, 229, 163, 56, 184, 250,
            190, 44, 91, 69, 246, 222, 64, 101, 139, 22, 126, 6, 103, 238,
        ];
        let sum_signature = Signature::from_slice_unchecked(&[
            106, 152, 91, 255, 122, 191, 159, 252, 180, 225, 105, 182, 30, 16, 99, 187, 220, 139,
            88, 105, 112, 224, 167, 249, 76, 12, 108, 182, 144, 208, 55, 80, 191, 47, 246, 87, 213,
            158, 237, 197, 199, 181, 91, 232, 197, 136, 230, 155, 56, 106, 217, 129, 200, 31, 113,
            254, 148, 234, 134, 152, 173, 69, 51, 13,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            130, 93, 138, 240, 229, 140, 60, 97, 160, 189, 208, 185, 248, 206, 146, 160, 53, 173,
            146, 163, 35, 233, 191, 177, 72, 121, 136, 23, 32, 241, 181, 165,
        ]);
        assert_eq!(coord.validate_sum_task(&sum_signature, &pk).unwrap(), ());
        let sum_signature = Signature::from_slice_unchecked(&[
            237, 143, 229, 127, 38, 65, 45, 145, 131, 233, 178, 250, 81, 211, 224, 103, 236, 91,
            82, 56, 19, 186, 236, 134, 19, 124, 16, 54, 148, 121, 206, 31, 71, 2, 11, 90, 41, 183,
            56, 58, 216, 3, 199, 181, 195, 118, 43, 185, 173, 25, 62, 186, 146, 14, 147, 24, 14,
            191, 118, 202, 185, 124, 125, 9,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            121, 99, 230, 84, 169, 21, 227, 76, 114, 4, 61, 21, 68, 153, 79, 43, 111, 201, 28, 152,
            111, 145, 208, 17, 156, 93, 67, 74, 56, 40, 202, 149,
        ]);
        assert_eq!(
            coord.validate_sum_task(&sum_signature, &pk).unwrap_err(),
            PetError::InvalidMessage,
        );
    }

    #[test]
    fn test_validate_update_task() {
        let mut coord = Coordinator::new().unwrap();
        coord.seed = vec![
            229, 16, 164, 40, 138, 161, 23, 161, 175, 102, 13, 103, 229, 229, 163, 56, 184, 250,
            190, 44, 91, 69, 246, 222, 64, 101, 139, 22, 126, 6, 103, 238,
        ];
        let sum_signature = Signature::from_slice_unchecked(&[
            184, 138, 175, 209, 149, 211, 214, 237, 125, 97, 56, 97, 206, 13, 111, 107, 227, 146,
            40, 41, 210, 179, 5, 83, 113, 185, 6, 3, 221, 135, 128, 74, 20, 120, 102, 182, 16, 138,
            58, 94, 7, 128, 151, 50, 10, 107, 253, 73, 126, 36, 244, 141, 254, 34, 113, 71, 196,
            127, 18, 96, 223, 176, 67, 10,
        ]);
        let update_signature = Signature::from_slice_unchecked(&[
            71, 51, 166, 220, 84, 170, 245, 60, 139, 79, 238, 74, 172, 122, 130, 47, 188, 168, 114,
            237, 210, 210, 234, 7, 123, 88, 73, 173, 174, 187, 82, 140, 41, 6, 44, 202, 255, 180,
            36, 186, 170, 97, 164, 155, 93, 21, 136, 114, 208, 246, 158, 254, 242, 12, 217, 148,
            27, 206, 44, 52, 204, 55, 4, 13,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            106, 233, 139, 112, 104, 250, 253, 242, 74, 19, 188, 176, 211, 198, 17, 98, 132, 9,
            220, 253, 191, 119, 159, 138, 134, 250, 244, 193, 58, 244, 218, 231,
        ]);
        assert_eq!(
            coord
                .validate_update_task(&sum_signature, &update_signature, &pk)
                .unwrap(),
            (),
        );
        let sum_signature = Signature::from_slice_unchecked(&[
            136, 94, 175, 83, 39, 171, 196, 102, 225, 111, 39, 28, 104, 51, 34, 117, 112, 178, 165,
            134, 128, 184, 131, 67, 73, 244, 98, 0, 133, 12, 111, 60, 215, 19, 237, 197, 96, 110,
            27, 196, 205, 3, 201, 112, 30, 24, 109, 145, 30, 62, 169, 130, 113, 35, 253, 194, 148,
            111, 151, 203, 238, 109, 223, 13,
        ]);
        let update_signature = Signature::from_slice_unchecked(&[
            189, 170, 55, 119, 59, 71, 14, 211, 117, 167, 110, 79, 44, 160, 171, 199, 43, 77, 147,
            65, 121, 172, 77, 248, 81, 62, 66, 111, 235, 209, 131, 188, 5, 117, 123, 81, 204, 136,
            205, 213, 28, 248, 46, 39, 83, 80, 66, 3, 77, 224, 60, 248, 231, 216, 241, 224, 87,
            170, 120, 214, 43, 106, 188, 13,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            221, 242, 188, 27, 163, 226, 152, 164, 43, 89, 154, 78, 26, 54, 35, 233, 129, 245, 131,
            251, 251, 154, 171, 121, 207, 58, 134, 201, 185, 31, 80, 181,
        ]);
        assert_eq!(
            coord
                .validate_update_task(&sum_signature, &update_signature, &pk)
                .unwrap_err(),
            PetError::InvalidMessage,
        );
        let sum_signature = Signature::from_slice_unchecked(&[
            70, 46, 99, 192, 150, 169, 206, 133, 91, 206, 219, 205, 228, 255, 57, 96, 186, 64, 63,
            79, 109, 112, 192, 225, 238, 41, 5, 27, 213, 91, 83, 60, 219, 81, 227, 101, 30, 12, 36,
            87, 37, 57, 64, 184, 146, 129, 217, 215, 212, 43, 77, 255, 202, 93, 150, 25, 147, 50,
            63, 93, 8, 83, 33, 14,
        ]);
        let update_signature = Signature::from_slice_unchecked(&[
            222, 204, 229, 157, 200, 187, 57, 66, 40, 158, 76, 184, 105, 1, 221, 122, 119, 110,
            115, 98, 119, 189, 130, 222, 8, 83, 69, 80, 107, 230, 18, 58, 180, 198, 160, 115, 111,
            173, 147, 182, 89, 197, 14, 138, 199, 64, 28, 34, 51, 98, 32, 219, 138, 252, 133, 139,
            219, 212, 207, 133, 61, 79, 200, 7,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            63, 238, 181, 248, 155, 69, 222, 175, 198, 46, 148, 78, 39, 51, 249, 250, 45, 157, 92,
            1, 18, 43, 24, 199, 144, 235, 245, 85, 63, 225, 151, 120,
        ]);
        assert_eq!(
            coord
                .validate_update_task(&sum_signature, &update_signature, &pk)
                .unwrap_err(),
            PetError::InvalidMessage,
        );
        let sum_signature = Signature::from_slice_unchecked(&[
            186, 136, 94, 177, 248, 84, 83, 97, 83, 183, 242, 20, 93, 90, 21, 159, 238, 90, 82,
            254, 87, 74, 53, 23, 199, 27, 224, 156, 113, 252, 66, 90, 167, 109, 166, 89, 80, 96,
            216, 227, 177, 218, 216, 59, 239, 169, 132, 33, 91, 108, 26, 163, 159, 233, 34, 208, 7,
            19, 106, 175, 193, 253, 47, 14,
        ]);
        let update_signature = Signature::from_slice_unchecked(&[
            146, 127, 108, 132, 170, 89, 77, 240, 50, 81, 109, 30, 120, 212, 65, 155, 132, 147,
            199, 86, 136, 204, 184, 14, 162, 107, 45, 215, 73, 129, 214, 79, 160, 249, 118, 47,
            116, 140, 91, 200, 226, 203, 166, 35, 54, 24, 148, 124, 113, 154, 131, 141, 122, 25,
            26, 224, 175, 60, 221, 27, 252, 234, 245, 15,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            147, 43, 34, 245, 84, 183, 114, 36, 243, 153, 91, 4, 75, 52, 247, 250, 86, 96, 127,
            106, 222, 191, 119, 72, 208, 88, 242, 40, 178, 151, 8, 7,
        ]);
        assert_eq!(
            coord
                .validate_update_task(&sum_signature, &update_signature, &pk)
                .unwrap_err(),
            PetError::InvalidMessage,
        );
    }

    fn auxiliary_sum(min_sum: usize) -> SumDict {
        iter::repeat_with(|| {
            (
                PublicSigningKey::from_slice(&randombytes(32)).unwrap(),
                PublicEncryptKey::from_slice(&randombytes(32)).unwrap(),
            )
        })
        .take(min_sum)
        .collect()
    }

    #[test]
    fn test_sum_dict() {
        let mut coord = Coordinator::new().unwrap();
        coord.min_sum = 3;
        coord.min_update = 3;

        // start the sum phase
        coord.try_phase_transition();
        assert_eq!(coord.phase, Phase::Sum);
        assert!(coord.sum_dict.is_empty());

        // Artifically add just enough sum participants
        let sum_dict = auxiliary_sum(coord.min_sum);
        for (pk, ephm_pk) in sum_dict.iter() {
            assert!(!coord.has_enough_sums());
            coord.add_sum_participant(pk, ephm_pk);
        }
        assert_eq!(coord.sum_dict, sum_dict);
        assert!(coord.seed_dict.is_empty());
        assert!(coord.has_enough_sums());

        // finish the sum phase
        coord.try_phase_transition();
        assert_eq!(coord.phase, Phase::Update);
        assert_eq!(
            coord.seed_dict,
            sum_dict
                .iter()
                .map(|(pk, _)| (*pk, LocalSeedDict::new()))
                .collect(),
        );
    }

    fn generate_update(sum_dict: &SumDict) -> (UpdateParticipantPublicKey, LocalSeedDict) {
        let seed = MaskSeed::new();
        let pk = PublicSigningKey::from_slice(&randombytes(32)).unwrap();
        let local_seed_dict = sum_dict
            .iter()
            .map(|(sum_pk, sum_ephm_pk)| (*sum_pk, seed.seal(sum_ephm_pk)))
            .collect::<LocalSeedDict>();
        (pk, local_seed_dict)
    }

    fn auxiliary_update(
        min_sum: usize,
        min_update: usize,
    ) -> (
        SumDict,
        Vec<(UpdateParticipantPublicKey, LocalSeedDict)>,
        SeedDict,
    ) {
        let sum_dict = auxiliary_sum(min_sum);
        let updates = iter::repeat_with(|| generate_update(&sum_dict))
            .take(min_update)
            .collect::<Vec<(UpdateParticipantPublicKey, LocalSeedDict)>>();
        let mut seed_dict = SeedDict::new();
        for sum_pk in sum_dict.keys() {
            // Dictionary of all the encrypted seeds for that participant
            let sum_participant_seeds = updates
                .iter()
                .map(|(upd_pk, local_seed_dict)| {
                    (*upd_pk, local_seed_dict.get(sum_pk).unwrap().clone())
                })
                .collect();
            seed_dict.insert(*sum_pk, sum_participant_seeds);
        }
        (sum_dict, updates, seed_dict)
    }

    #[test]
    fn test_seed_dict() {
        let mut coord = Coordinator::new().unwrap();
        coord.min_sum = 3;
        coord.min_update = 3;
        coord.try_phase_transition(); // start the sum phase

        // artificially populate the sum dictionary
        let (sum_dict, updates, seed_dict) = auxiliary_update(coord.min_sum, coord.min_update);
        coord.sum_dict = sum_dict;

        coord.try_phase_transition(); // start the update phase
        assert_eq!(coord.phase, Phase::Update);
        assert!(!coord.has_enough_seeds());

        // simulate update participants sending their seeds dictionary
        for (pk, local_seed_dict) in updates.iter() {
            assert!(!coord.has_enough_seeds());
            coord.add_local_seed_dict(pk, local_seed_dict).unwrap();
        }
        assert_eq!(coord.seed_dict, seed_dict);
        assert!(coord.has_enough_seeds());

        coord.try_phase_transition(); // finish the update phase
        assert_eq!(coord.phase, Phase::Sum2);
    }

    fn auxiliary_mask(min_sum: usize) -> (Vec<Mask>, MaskDict) {
        // this doesn't work for `min_sum == 0` and `min_sum == 2`
        let masks = [
            vec![Mask::from(randombytes(32)); min_sum - 1],
            vec![Mask::from(randombytes(32)); 1],
        ]
        .concat();
        let mask_dict = masks
            .iter()
            .map(|mask| sha256::hash(mask.as_ref()))
            .collect::<MaskDict>();
        (masks, mask_dict)
    }

    #[test]
    fn test_mask_dict() {
        let mut coord = Coordinator::new().unwrap();
        coord.min_sum = 3;
        coord.min_update = 3;
        coord.phase = Phase::Sum2;

        // Pretend we received enough masks
        let (masks, mask_dict) = auxiliary_mask(coord.min_sum);
        for mask in masks.iter() {
            coord.add_mask_hash(mask);
        }
        assert_eq!(coord.mask_dict, mask_dict);
        assert!(coord.has_enough_masks());
        assert_eq!(
            coord.freeze_mask_dict().unwrap(),
            mask_dict.most_common()[0].0,
        );
    }

    #[test]
    fn test_mask_dict_fail() {
        let mut coord = Coordinator::new().unwrap();
        coord.min_sum = 3;
        coord.min_update = 3;
        coord.phase = Phase::Sum2;

        coord.mask_dict = iter::repeat_with(|| sha256::hash(&randombytes(32)))
            .take(coord.min_sum)
            .collect::<MaskDict>();
        assert_eq!(
            coord.freeze_mask_dict().unwrap_err(),
            PetError::AmbiguousMasks,
        );
    }

    #[test]
    fn test_clear_round_dicts() {
        let mut coord = Coordinator::new().unwrap();
        coord.clear_round_dicts();
        assert!(coord.sum_dict.is_empty());
        assert!(coord.seed_dict.is_empty());
        assert!(coord.mask_dict.is_empty());
    }

    #[test]
    fn test_gen_round_keypair() {
        let mut coord = Coordinator::new().unwrap();
        coord.gen_round_keypair();
        assert_eq!(coord.pk, coord.sk.public_key());
        assert_eq!(coord.sk.as_slice().len(), 32);
    }

    #[test]
    fn test_update_round_seed() {
        let mut coord = Coordinator::new().unwrap();
        coord.seed = vec![
            229, 16, 164, 40, 138, 161, 23, 161, 175, 102, 13, 103, 229, 229, 163, 56, 184, 250,
            190, 44, 91, 69, 246, 222, 64, 101, 139, 22, 126, 6, 103, 238,
        ];
        coord.sk = SecretEncryptKey::from_slice_unchecked(&[
            39, 177, 238, 71, 112, 48, 60, 73, 246, 28, 143, 222, 211, 114, 29, 34, 174, 28, 77,
            51, 146, 27, 155, 224, 20, 169, 254, 164, 231, 141, 190, 31,
        ]);
        coord.update_round_seed();
        assert_eq!(
            coord.seed,
            vec![
                90, 35, 97, 78, 70, 149, 40, 131, 149, 211, 30, 236, 194, 175, 156, 76, 85, 43,
                138, 159, 180, 166, 25, 205, 156, 176, 3, 203, 27, 128, 231, 38
            ],
        );
    }

    #[test]
    fn test_transitions() {
        let mut coord = Coordinator::new().unwrap();
        coord.min_sum = 3;
        coord.min_update = 3;

        let (sum_dict, _, seed_dict) = auxiliary_update(coord.min_sum, coord.min_update);
        let (_, mask_dict) = auxiliary_mask(coord.min_sum);
        assert_eq!(coord.phase, Phase::Idle);

        coord.try_phase_transition();
        assert_eq!(coord.phase, Phase::Sum);
        assert_ne!(coord.pk, PublicEncryptKey::zeroed());
        assert_ne!(coord.sk, SecretEncryptKey::zeroed());

        coord.try_phase_transition();
        // We didn't add any participant so the state should remain
        // unchanged
        assert_eq!(coord.phase, Phase::Sum);

        // Pretend we have enough participants, and transition
        // again. This time, the state should change.
        coord.sum_dict = sum_dict;
        coord.try_phase_transition();
        assert_eq!(coord.phase, Phase::Update);

        // We didn't add any update so the state should remain
        // unchanged
        coord.try_phase_transition();
        assert_eq!(coord.phase, Phase::Update);

        // Pretend we received enough updates and transition. This
        // time the state should change.
        coord.seed_dict = seed_dict;
        coord.try_phase_transition();
        assert_eq!(coord.phase, Phase::Sum2);

        // We didn't add any mask so the state should remain unchanged
        coord.try_phase_transition();
        assert_eq!(coord.phase, Phase::Sum2);

        // Pretend we received enough masks and transition. This time
        // the state should change and we should be back to the
        // beginning: Phase::Idle.
        coord.mask_dict = mask_dict;
        let seed = coord.seed.clone();
        coord.try_phase_transition();
        assert_eq!(coord.phase, Phase::Idle);
        assert!(coord.sum_dict.is_empty());
        assert!(coord.seed_dict.is_empty());
        assert!(coord.mask_dict.is_empty());
        assert_ne!(coord.seed, seed);
    }
}
