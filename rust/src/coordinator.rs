#![allow(dead_code)] // temporary

use std::{
    collections::{HashMap, HashSet},
    default::Default,
};

use counter::Counter;
use sodiumoxide::{
    self,
    crypto::{box_, hash::sha256, sign},
    randombytes::randombytes,
};

use super::{
    message::{Sum2Message, SumMessage, UpdateMessage},
    utils::is_eligible,
    PetError,
};

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
    encr_pk: box_::PublicKey, // 32 bytes
    encr_sk: box_::SecretKey, // 32 bytes
    sign_pk: sign::PublicKey, // 32 bytes
    sign_sk: sign::SecretKey, // 64 bytes

    // round parameters
    sum: f64,
    update: f64,
    seed: Vec<u8>, // 32 bytes
    min_sum: usize,
    min_update: usize,
    phase: Phase,

    // round dictionaries
    dict_sum: HashMap<box_::PublicKey, box_::PublicKey>,
    dict_seed: HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    dict_mask: Counter<Vec<u8>>,
}

impl Default for Coordinator {
    fn default() -> Self {
        let encr_pk = box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let encr_sk = box_::SecretKey([0_u8; box_::SECRETKEYBYTES]);
        let sign_pk = sign::PublicKey([0_u8; sign::PUBLICKEYBYTES]);
        let sign_sk = sign::SecretKey([0_u8; sign::SECRETKEYBYTES]);
        let sum = 0.01_f64;
        let update = 0.1_f64;
        let seed = vec![0_u8; 32];
        let min_sum = 1_usize;
        let min_update = 3_usize;
        let phase = Phase::Idle;
        let dict_sum = HashMap::new();
        let dict_seed = HashMap::new();
        let dict_mask = Counter::new();
        Self {
            encr_pk,
            encr_sk,
            sign_pk,
            sign_sk,
            sum,
            update,
            seed,
            min_sum,
            min_update,
            phase,
            dict_sum,
            dict_seed,
            dict_mask,
        }
    }
}

impl Coordinator {
    /// Create a coordinator. Fails if there is insufficient system entropy to generate secrets.
    pub fn new() -> Result<Self, PetError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(PetError::InsufficientSystemEntropy))?;
        let (sign_pk, sign_sk) = sign::gen_keypair();
        let seed = randombytes(32);
        Ok(Self {
            sign_pk,
            sign_sk,
            seed,
            ..Default::default()
        })
    }

    /// Validate and handle a sum, update or sum2 message.
    pub fn validate_message(&mut self, message: &[u8]) -> Result<(), PetError> {
        match self.phase {
            Phase::Idle => Err(PetError::InvalidMessage),
            Phase::Sum => self.validate_message_sum(message),
            Phase::Update => self.validate_message_update(message),
            Phase::Sum2 => self.validate_message_sum2(message),
        }
    }

    /// Validate and handle a sum message.
    fn validate_message_sum(&mut self, message: &[u8]) -> Result<(), PetError> {
        let msg = SumMessage::open(
            message,
            &self.encr_pk,
            &self.encr_sk,
            SumMessage::exp_len(None),
        )?;

        Self::validate_certificate(msg.certificate())?;
        self.validate_task_sum(msg.signature_sum(), msg.sign_pk())?;
        self.update_dict_sum(msg.encr_pk(), msg.ephm_pk());
        Ok(())
    }

    /// Validate and handle an update message.
    fn validate_message_update(&mut self, message: &[u8]) -> Result<(), PetError> {
        let msg = UpdateMessage::open(
            message,
            &self.encr_pk,
            &self.encr_sk,
            UpdateMessage::exp_len(Some(self.dict_sum.len())),
        )?;
        Self::validate_certificate(msg.certificate())?;
        self.validate_task_update(msg.signature_sum(), msg.signature_update(), msg.sign_pk())?;
        self.update_dict_seed(msg.encr_pk(), msg.dict_seed())?;
        Ok(())
    }

    /// Validate and handle a sum2 message.
    fn validate_message_sum2(&mut self, message: &[u8]) -> Result<(), PetError> {
        let msg = Sum2Message::open(
            message,
            &self.encr_pk,
            &self.encr_sk,
            Sum2Message::exp_len(None),
        )?;
        Self::validate_certificate(msg.certificate())?;
        self.validate_task_sum(msg.signature_sum(), msg.sign_pk())?;
        self.update_dict_mask(msg.mask_url());
        Ok(())
    }

    /// Validate a certificate (dummy).
    fn validate_certificate(certificate: &[u8]) -> Result<(), PetError> {
        (certificate == b"")
            .then_some(())
            .ok_or(PetError::InvalidMessage)
    }

    /// Validate a sum signature and its implied task.
    fn validate_task_sum(
        &self,
        signature_sum: &sign::Signature,
        sign_pk: &sign::PublicKey,
    ) -> Result<(), PetError> {
        (sign::verify_detached(
            signature_sum,
            &[self.seed.as_slice(), b"sum"].concat(),
            sign_pk,
        ) && is_eligible(signature_sum, self.sum))
        .then_some(())
        .ok_or(PetError::InvalidMessage)
    }

    /// Validate an update signature and its implied task.
    fn validate_task_update(
        &self,
        signature_sum: &sign::Signature,
        signature_update: &sign::Signature,
        sign_pk: &sign::PublicKey,
    ) -> Result<(), PetError> {
        (sign::verify_detached(
            signature_sum,
            &[self.seed.as_slice(), b"sum"].concat(),
            sign_pk,
        ) && sign::verify_detached(
            signature_update,
            &[self.seed.as_slice(), b"update"].concat(),
            sign_pk,
        ) && !is_eligible(signature_sum, self.sum)
            && is_eligible(signature_update, self.update))
        .then_some(())
        .ok_or(PetError::InvalidMessage)
    }

    /// Update the sum dictionary.
    fn update_dict_sum(&mut self, encr_pk: &box_::PublicKey, ephm_pk: &box_::PublicKey) {
        self.dict_sum.insert(*encr_pk, *ephm_pk);
    }

    /// Freeze the sum dictionary. Fails due to insufficient sum participants.
    fn freeze_dict_sum(&mut self) -> Result<(), PetError> {
        (self.dict_sum.len() >= self.min_sum)
            .then(|| {
                self.dict_seed = self
                    .dict_sum
                    .keys()
                    .map(|pk| (*pk, HashMap::<box_::PublicKey, Vec<u8>>::new()))
                    .collect();
            })
            .ok_or(PetError::InsufficientParticipants)
    }

    /// Update the seed dictionary.
    fn update_dict_seed(
        &mut self,
        encr_pk: &box_::PublicKey,
        dict_seed: &HashMap<box_::PublicKey, Vec<u8>>,
    ) -> Result<(), PetError> {
        (dict_seed.keys().collect::<HashSet<&box_::PublicKey>>()
            == self.dict_sum.keys().collect::<HashSet<&box_::PublicKey>>())
        .then(|| {
            dict_seed.iter().for_each(|(pk, seed)| {
                self.dict_seed
                    .get_mut(pk)
                    .unwrap()
                    .insert(*encr_pk, seed.clone());
            });
        })
        .ok_or(PetError::InvalidMessage)
    }

    /// Freeze the seed dictionary. Fails due to insufficient update participants.
    fn freeze_dict_seed(&mut self) -> Result<(), PetError> {
        (self
            .dict_seed
            .values()
            .all(|dict| dict.len() >= self.min_update))
        .then_some(())
        .ok_or(PetError::InsufficientParticipants)
    }

    /// Update the mask dictionary.
    fn update_dict_mask(&mut self, mask_url: &[u8]) {
        self.dict_mask += mask_url
            .chunks_exact(mask_url.len())
            .map(|mask| mask.to_vec());
    }

    /// Freeze the mask dictionary. Returns a unique mask. Fails due to insufficient sum
    /// participants.
    fn freeze_dict_mask(&self) -> Result<Vec<u8>, PetError> {
        let counts = self.dict_mask.most_common();
        (counts.iter().map(|(_, count)| count).sum::<usize>() >= self.min_sum
            && (counts.len() == 1 || counts[0].1 > counts[1].1))
            .then(|| counts[0].0.clone())
            .ok_or(PetError::InsufficientParticipants)
    }

    /// Clear the round dictionaries.
    fn clear_round_dicts(&mut self) {
        self.dict_sum.clear();
        self.dict_sum.shrink_to_fit();
        self.dict_seed.clear();
        self.dict_seed.shrink_to_fit();
        self.dict_mask.clear();
        self.dict_mask.shrink_to_fit();
    }

    /// Generate fresh round credentials.
    fn gen_round_keypair(&mut self) {
        let (encr_pk, encr_sk) = box_::gen_keypair();
        self.encr_pk = encr_pk;
        self.encr_sk = encr_sk;
    }

    /// Update the sum round parameter (dummy).
    fn update_round_sum(&mut self) {}

    /// Update the update round parameter (dummy).
    fn update_round_update(&mut self) {}

    /// Update the seed round parameter.
    fn update_round_seed(&mut self) {
        self.seed = sha256::hash(
            sign::sign_detached(
                &[
                    self.seed.as_slice(),
                    &self.sum.to_le_bytes(),
                    &self.update.to_le_bytes(),
                ]
                .concat(),
                &self.sign_sk,
            )
            .as_ref(),
        )
        .as_ref()
        .to_vec();
    }

    /// Proceed to the next phase. Fails due to insufficient participants.
    pub fn proceed_phase(&mut self) -> Result<(), PetError> {
        match self.phase {
            Phase::Idle => {
                self.proceed_phase_sum();
                Ok(())
            }
            Phase::Sum => self.proceed_phase_update(),
            Phase::Update => self.proceed_phase_sum2(),
            Phase::Sum2 => self.proceed_phase_idle(),
        }
    }

    /// End the idle phase and proceed to the sum phase to start the round.
    fn proceed_phase_sum(&mut self) {
        self.gen_round_keypair();
        self.phase = Phase::Sum;
    }

    /// End the sum phase and proceed to the update phase. Fails due to insufficient sum
    /// participants.
    fn proceed_phase_update(&mut self) -> Result<(), PetError> {
        self.freeze_dict_sum()?;
        self.phase = Phase::Update;
        Ok(())
    }

    /// End the update phase and proceed to the sum2 phase. Fails due to insufficient update
    /// participants.
    fn proceed_phase_sum2(&mut self) -> Result<(), PetError> {
        self.freeze_dict_seed()?;
        self.phase = Phase::Sum2;
        Ok(())
    }

    /// End the sum2 phase and proceed to the idle phase to end the round. Fails due to insufficient
    /// sum participants.
    fn proceed_phase_idle(&mut self) -> Result<(), PetError> {
        let _mask_url = self.freeze_dict_mask()?;
        self.clear_round_dicts();
        self.update_round_sum();
        self.update_round_update();
        self.update_round_seed();
        self.phase = Phase::Idle;
        Ok(())
    }

    pub fn round_parameters(&self) -> RoundParameters {
        RoundParameters {
            sum: self.sum,
            update: self.update,
            seed: self.seed.clone(),
            encr_pk: self.encr_pk,
            sign_pk: self.sign_pk,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use sodiumoxide::{crypto::sealedbox, randombytes::randombytes_uniform};

    use super::*;

    #[test]
    fn test_coordinator() {
        let coord = Coordinator::new().unwrap();
        assert_eq!(coord.encr_pk, box_::PublicKey([0_u8; 32]));
        assert_eq!(coord.encr_sk, box_::SecretKey([0_u8; 32]));
        assert_eq!(coord.sign_pk, coord.sign_sk.public_key());
        assert_eq!(coord.sign_sk.as_ref().len(), 64);
        assert!(coord.sum >= 0. && coord.sum <= 1.);
        assert!(coord.update >= 0. && coord.update <= 1.);
        assert_eq!(coord.seed.len(), 32);
        assert!(coord.min_sum >= 1);
        assert!(coord.min_update >= 3);
        assert_eq!(coord.phase, Phase::Idle);
        assert_eq!(coord.dict_sum, HashMap::new());
        assert_eq!(coord.dict_seed, HashMap::new());
        assert_eq!(coord.dict_mask, Counter::new());
    }

    #[test]
    fn test_validate_task_sum() {
        let mut coord = Coordinator::new().unwrap();
        coord.seed = vec![
            229, 16, 164, 40, 138, 161, 23, 161, 175, 102, 13, 103, 229, 229, 163, 56, 184, 250,
            190, 44, 91, 69, 246, 222, 64, 101, 139, 22, 126, 6, 103, 238,
        ];
        let signature_sum = sign::Signature([
            106, 152, 91, 255, 122, 191, 159, 252, 180, 225, 105, 182, 30, 16, 99, 187, 220, 139,
            88, 105, 112, 224, 167, 249, 76, 12, 108, 182, 144, 208, 55, 80, 191, 47, 246, 87, 213,
            158, 237, 197, 199, 181, 91, 232, 197, 136, 230, 155, 56, 106, 217, 129, 200, 31, 113,
            254, 148, 234, 134, 152, 173, 69, 51, 13,
        ]);
        let sign_pk = sign::PublicKey([
            130, 93, 138, 240, 229, 140, 60, 97, 160, 189, 208, 185, 248, 206, 146, 160, 53, 173,
            146, 163, 35, 233, 191, 177, 72, 121, 136, 23, 32, 241, 181, 165,
        ]);
        assert_eq!(coord.validate_task_sum(&signature_sum, &sign_pk), Ok(()));
        let signature_sum = sign::Signature([
            237, 143, 229, 127, 38, 65, 45, 145, 131, 233, 178, 250, 81, 211, 224, 103, 236, 91,
            82, 56, 19, 186, 236, 134, 19, 124, 16, 54, 148, 121, 206, 31, 71, 2, 11, 90, 41, 183,
            56, 58, 216, 3, 199, 181, 195, 118, 43, 185, 173, 25, 62, 186, 146, 14, 147, 24, 14,
            191, 118, 202, 185, 124, 125, 9,
        ]);
        let sign_pk = sign::PublicKey([
            121, 99, 230, 84, 169, 21, 227, 76, 114, 4, 61, 21, 68, 153, 79, 43, 111, 201, 28, 152,
            111, 145, 208, 17, 156, 93, 67, 74, 56, 40, 202, 149,
        ]);
        assert_eq!(
            coord.validate_task_sum(&signature_sum, &sign_pk),
            Err(PetError::InvalidMessage),
        );
    }

    #[test]
    fn test_validate_task_update() {
        let mut coord = Coordinator::new().unwrap();
        coord.seed = vec![
            229, 16, 164, 40, 138, 161, 23, 161, 175, 102, 13, 103, 229, 229, 163, 56, 184, 250,
            190, 44, 91, 69, 246, 222, 64, 101, 139, 22, 126, 6, 103, 238,
        ];
        let signature_sum = sign::Signature([
            184, 138, 175, 209, 149, 211, 214, 237, 125, 97, 56, 97, 206, 13, 111, 107, 227, 146,
            40, 41, 210, 179, 5, 83, 113, 185, 6, 3, 221, 135, 128, 74, 20, 120, 102, 182, 16, 138,
            58, 94, 7, 128, 151, 50, 10, 107, 253, 73, 126, 36, 244, 141, 254, 34, 113, 71, 196,
            127, 18, 96, 223, 176, 67, 10,
        ]);
        let signature_update = sign::Signature([
            71, 51, 166, 220, 84, 170, 245, 60, 139, 79, 238, 74, 172, 122, 130, 47, 188, 168, 114,
            237, 210, 210, 234, 7, 123, 88, 73, 173, 174, 187, 82, 140, 41, 6, 44, 202, 255, 180,
            36, 186, 170, 97, 164, 155, 93, 21, 136, 114, 208, 246, 158, 254, 242, 12, 217, 148,
            27, 206, 44, 52, 204, 55, 4, 13,
        ]);
        let sign_pk = sign::PublicKey([
            106, 233, 139, 112, 104, 250, 253, 242, 74, 19, 188, 176, 211, 198, 17, 98, 132, 9,
            220, 253, 191, 119, 159, 138, 134, 250, 244, 193, 58, 244, 218, 231,
        ]);
        assert_eq!(
            coord.validate_task_update(&signature_sum, &signature_update, &sign_pk),
            Ok(()),
        );
        let signature_sum = sign::Signature([
            136, 94, 175, 83, 39, 171, 196, 102, 225, 111, 39, 28, 104, 51, 34, 117, 112, 178, 165,
            134, 128, 184, 131, 67, 73, 244, 98, 0, 133, 12, 111, 60, 215, 19, 237, 197, 96, 110,
            27, 196, 205, 3, 201, 112, 30, 24, 109, 145, 30, 62, 169, 130, 113, 35, 253, 194, 148,
            111, 151, 203, 238, 109, 223, 13,
        ]);
        let signature_update = sign::Signature([
            189, 170, 55, 119, 59, 71, 14, 211, 117, 167, 110, 79, 44, 160, 171, 199, 43, 77, 147,
            65, 121, 172, 77, 248, 81, 62, 66, 111, 235, 209, 131, 188, 5, 117, 123, 81, 204, 136,
            205, 213, 28, 248, 46, 39, 83, 80, 66, 3, 77, 224, 60, 248, 231, 216, 241, 224, 87,
            170, 120, 214, 43, 106, 188, 13,
        ]);
        let sign_pk = sign::PublicKey([
            221, 242, 188, 27, 163, 226, 152, 164, 43, 89, 154, 78, 26, 54, 35, 233, 129, 245, 131,
            251, 251, 154, 171, 121, 207, 58, 134, 201, 185, 31, 80, 181,
        ]);
        assert_eq!(
            coord.validate_task_update(&signature_sum, &signature_update, &sign_pk),
            Err(PetError::InvalidMessage),
        );
        let signature_sum = sign::Signature([
            70, 46, 99, 192, 150, 169, 206, 133, 91, 206, 219, 205, 228, 255, 57, 96, 186, 64, 63,
            79, 109, 112, 192, 225, 238, 41, 5, 27, 213, 91, 83, 60, 219, 81, 227, 101, 30, 12, 36,
            87, 37, 57, 64, 184, 146, 129, 217, 215, 212, 43, 77, 255, 202, 93, 150, 25, 147, 50,
            63, 93, 8, 83, 33, 14,
        ]);
        let signature_update = sign::Signature([
            222, 204, 229, 157, 200, 187, 57, 66, 40, 158, 76, 184, 105, 1, 221, 122, 119, 110,
            115, 98, 119, 189, 130, 222, 8, 83, 69, 80, 107, 230, 18, 58, 180, 198, 160, 115, 111,
            173, 147, 182, 89, 197, 14, 138, 199, 64, 28, 34, 51, 98, 32, 219, 138, 252, 133, 139,
            219, 212, 207, 133, 61, 79, 200, 7,
        ]);
        let sign_pk = sign::PublicKey([
            63, 238, 181, 248, 155, 69, 222, 175, 198, 46, 148, 78, 39, 51, 249, 250, 45, 157, 92,
            1, 18, 43, 24, 199, 144, 235, 245, 85, 63, 225, 151, 120,
        ]);
        assert_eq!(
            coord.validate_task_update(&signature_sum, &signature_update, &sign_pk),
            Err(PetError::InvalidMessage),
        );
        let signature_sum = sign::Signature([
            186, 136, 94, 177, 248, 84, 83, 97, 83, 183, 242, 20, 93, 90, 21, 159, 238, 90, 82,
            254, 87, 74, 53, 23, 199, 27, 224, 156, 113, 252, 66, 90, 167, 109, 166, 89, 80, 96,
            216, 227, 177, 218, 216, 59, 239, 169, 132, 33, 91, 108, 26, 163, 159, 233, 34, 208, 7,
            19, 106, 175, 193, 253, 47, 14,
        ]);
        let signature_update = sign::Signature([
            146, 127, 108, 132, 170, 89, 77, 240, 50, 81, 109, 30, 120, 212, 65, 155, 132, 147,
            199, 86, 136, 204, 184, 14, 162, 107, 45, 215, 73, 129, 214, 79, 160, 249, 118, 47,
            116, 140, 91, 200, 226, 203, 166, 35, 54, 24, 148, 124, 113, 154, 131, 141, 122, 25,
            26, 224, 175, 60, 221, 27, 252, 234, 245, 15,
        ]);
        let sign_pk = sign::PublicKey([
            147, 43, 34, 245, 84, 183, 114, 36, 243, 153, 91, 4, 75, 52, 247, 250, 86, 96, 127,
            106, 222, 191, 119, 72, 208, 88, 242, 40, 178, 151, 8, 7,
        ]);
        assert_eq!(
            coord.validate_task_update(&signature_sum, &signature_update, &sign_pk),
            Err(PetError::InvalidMessage),
        );
    }

    fn auxiliary_sum(min_sum: usize) -> HashMap<box_::PublicKey, box_::PublicKey> {
        iter::repeat_with(|| {
            (
                box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
                box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
            )
        })
        .take(min_sum + randombytes_uniform(10) as usize)
        .collect()
    }

    #[test]
    fn test_dict_sum() {
        // update
        let mut coord = Coordinator::new().unwrap();
        assert!(coord.dict_sum.is_empty());
        assert_eq!(
            coord.freeze_dict_sum(),
            Err(PetError::InsufficientParticipants),
        );
        let dict_sum = auxiliary_sum(coord.min_sum);
        for (encr_pk, ephm_pk) in dict_sum.iter() {
            coord.update_dict_sum(encr_pk, ephm_pk);
        }
        assert_eq!(coord.dict_sum, dict_sum);

        // freeze
        assert!(coord.dict_seed.is_empty());
        assert_eq!(coord.freeze_dict_sum(), Ok(()));
        assert_eq!(
            coord.dict_seed,
            dict_sum
                .iter()
                .map(|(encr_pk, _)| (*encr_pk, HashMap::new()))
                .collect(),
        );
    }

    fn auxiliary_update(
        min_sum: usize,
        min_update: usize,
    ) -> (
        HashMap<box_::PublicKey, box_::PublicKey>,
        Vec<(box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>)>,
        HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    ) {
        let dict_sum = auxiliary_sum(min_sum);
        let updates = iter::repeat_with(|| {
            let seed = randombytes(32);
            let upd_encr_pk = box_::PublicKey::from_slice(&randombytes(32)).unwrap();
            let upd_dict_seed = dict_sum
                .iter()
                .map(|(sum_encr_pk, sum_ephm_pk)| {
                    (*sum_encr_pk, sealedbox::seal(&seed, sum_ephm_pk))
                })
                .collect::<HashMap<box_::PublicKey, Vec<u8>>>();
            (upd_encr_pk, upd_dict_seed)
        })
        .take(min_update + randombytes_uniform(10) as usize)
        .collect::<Vec<(box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>)>>();
        let dict_seed = dict_sum
            .iter()
            .map(|(sum_encr_pk, _)| {
                (
                    *sum_encr_pk,
                    updates
                        .iter()
                        .map(|(upd_encr_pk, dict_seed)| {
                            (*upd_encr_pk, dict_seed.get(sum_encr_pk).unwrap().clone())
                        })
                        .collect::<HashMap<box_::PublicKey, Vec<u8>>>(),
                )
            })
            .collect::<HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>>();
        (dict_sum, updates, dict_seed)
    }

    #[test]
    fn test_dict_seed() {
        // update
        let mut coord = Coordinator::new().unwrap();
        let (dict_sum, updates, dict_seed) = auxiliary_update(coord.min_sum, coord.min_update);
        coord.dict_sum = dict_sum;
        coord.freeze_dict_sum().unwrap();
        assert_eq!(
            coord.freeze_dict_seed(),
            Err(PetError::InsufficientParticipants),
        );
        for (encr_pk, dict_seed) in updates.iter() {
            coord.update_dict_seed(encr_pk, dict_seed).unwrap();
        }
        assert_eq!(coord.dict_seed, dict_seed);

        // freeze
        assert_eq!(coord.freeze_dict_seed(), Ok(()));
    }

    fn auxiliary_mask(min_sum: usize) -> (Vec<Vec<u8>>, Counter<Vec<u8>>) {
        let masks = match min_sum + randombytes_uniform(10) as usize {
            len @ 0..=2 => vec![randombytes(32); len],
            len => [vec![randombytes(32); len - 1], vec![randombytes(32); 1]].concat(),
        };
        let dict_mask = masks.iter().cloned().collect::<Counter<Vec<u8>>>();
        (masks, dict_mask)
    }

    #[test]
    fn test_dict_mask() {
        // update
        let mut coord = Coordinator::new().unwrap();
        assert_eq!(
            coord.freeze_dict_mask().unwrap_err(),
            PetError::InsufficientParticipants,
        );
        let (masks, dict_mask) = auxiliary_mask(coord.min_sum);
        for mask in masks.iter() {
            coord.update_dict_mask(mask);
        }
        assert_eq!(coord.dict_mask, dict_mask);

        // freeze
        assert_eq!(
            coord.freeze_dict_mask().unwrap(),
            dict_mask.most_common()[0].0,
        );

        // not unique
        coord.dict_mask = iter::repeat_with(|| randombytes(32))
            .take((coord.min_sum + randombytes_uniform(10) as usize).max(2))
            .collect::<Counter<Vec<u8>>>();
        assert_eq!(
            coord.freeze_dict_mask().unwrap_err(),
            PetError::InsufficientParticipants,
        );
    }

    #[test]
    fn test_clear_round_dicts() {
        let mut coord = Coordinator::new().unwrap();
        coord.clear_round_dicts();
        assert!(coord.dict_sum.is_empty());
        assert!(coord.dict_seed.is_empty());
        assert!(coord.dict_mask.is_empty());
    }

    #[test]
    fn test_gen_round_keypair() {
        let mut coord = Coordinator::new().unwrap();
        coord.gen_round_keypair();
        assert_eq!(coord.encr_pk, coord.encr_sk.public_key());
        assert_eq!(coord.encr_sk.as_ref().len(), 32);
    }

    #[test]
    fn test_update_round_seed() {
        let mut coord = Coordinator::new().unwrap();
        coord.seed = vec![
            229, 16, 164, 40, 138, 161, 23, 161, 175, 102, 13, 103, 229, 229, 163, 56, 184, 250,
            190, 44, 91, 69, 246, 222, 64, 101, 139, 22, 126, 6, 103, 238,
        ];
        coord.sign_sk = sign::SecretKey([
            72, 252, 162, 60, 90, 28, 214, 96, 4, 116, 71, 105, 97, 164, 192, 175, 210, 83, 50, 92,
            173, 243, 60, 238, 50, 162, 252, 216, 74, 15, 123, 76, 251, 186, 123, 178, 160, 3, 175,
            105, 175, 22, 238, 84, 120, 212, 110, 176, 51, 184, 143, 13, 55, 12, 87, 249, 142, 121,
            243, 62, 250, 97, 137, 153,
        ]);
        coord.update_round_seed();
        assert_eq!(
            coord.seed,
            vec![
                5, 13, 221, 236, 217, 108, 126, 186, 152, 180, 111, 173, 45, 124, 140, 79, 1, 239,
                176, 115, 38, 118, 221, 130, 246, 133, 212, 254, 46, 248, 222, 71,
            ],
        );
    }

    #[test]
    fn test_proceed_phase() {
        let mut coord = Coordinator::new().unwrap();
        let (dict_sum, _, dict_seed) = auxiliary_update(coord.min_sum, coord.min_update);
        let (_, dict_mask) = auxiliary_mask(coord.min_sum);
        assert_eq!(coord.phase, Phase::Idle);

        // proceed phase sum
        assert_eq!(coord.proceed_phase().unwrap(), ());
        assert_ne!(coord.encr_pk, box_::PublicKey([0_u8; 32]));
        assert_ne!(coord.encr_sk, box_::SecretKey([0_u8; 32]));
        assert_eq!(coord.phase, Phase::Sum);
        assert_eq!(
            coord.proceed_phase().unwrap_err(),
            PetError::InsufficientParticipants,
        );
        coord.dict_sum = dict_sum;

        // proceed phase update
        assert_eq!(coord.proceed_phase().unwrap(), ());
        assert_eq!(coord.phase, Phase::Update);
        assert_eq!(
            coord.proceed_phase().unwrap_err(),
            PetError::InsufficientParticipants,
        );
        coord.dict_seed = dict_seed;

        // proceed phase sum2
        assert_eq!(coord.proceed_phase().unwrap(), ());
        assert_eq!(coord.phase, Phase::Sum2);
        assert_eq!(
            coord.proceed_phase().unwrap_err(),
            PetError::InsufficientParticipants,
        );
        coord.dict_mask = dict_mask;

        // proceed phase idle
        let seed = coord.seed.clone();
        assert_eq!(coord.proceed_phase().unwrap(), ());
        assert!(coord.dict_sum.is_empty());
        assert!(coord.dict_seed.is_empty());
        assert!(coord.dict_mask.is_empty());
        assert_ne!(coord.seed, seed);
        assert_eq!(coord.phase, Phase::Idle);
    }
}

pub struct RoundParameters {
    /// Fraction of participants to be selected for the sum task
    pub sum: f64,

    /// Fraction of participants to be selected for the update task
    pub update: f64,

    /// The coordinator public key for encryption
    pub encr_pk: box_::PublicKey,

    /// The coordinator public key for signing
    pub sign_pk: sign::PublicKey,

    /// The random seed
    pub seed: Vec<u8>,
}
