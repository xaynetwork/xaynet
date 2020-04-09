#![allow(dead_code)] // temporary

use std::{collections::HashMap, default::Default};

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

#[derive(Debug, PartialEq)]
/// Round phases of a coordinator.
enum Phase {
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

    // dictionaries
    dict_sum: HashMap<box_::PublicKey, box_::PublicKey>,
    dict_seed: HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    masks: Counter<Vec<u8>>,
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
        let masks = Counter::new();
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
            masks,
        }
    }
}

impl Coordinator {
    /// Create a coordinator. Fails if there is insufficient system entropy to generate secrets.
    pub fn new() -> Result<Self, PetError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(PetError::InsufficientSystemEntropy))?;
        let (encr_pk, encr_sk) = box_::gen_keypair();
        let (sign_pk, sign_sk) = sign::gen_keypair();
        let seed = randombytes(32);
        Ok(Self {
            encr_pk,
            encr_sk,
            sign_pk,
            sign_sk,
            seed,
            ..Default::default()
        })
    }

    /// Validate and handle a sum message.
    pub fn validate_sum_message(&mut self, message: &[u8]) -> Result<(), PetError> {
        if self.phase == Phase::Sum {
            return Err(PetError::InvalidMessage);
        }
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
    pub fn validate_update_message(&mut self, message: &[u8]) -> Result<(), PetError> {
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
    pub fn validate_sum2_message(&mut self, message: &[u8]) -> Result<(), PetError> {
        let msg = Sum2Message::open(
            message,
            &self.encr_pk,
            &self.encr_sk,
            Sum2Message::exp_len(None),
        )?;
        Self::validate_certificate(msg.certificate())?;
        self.validate_task_sum(msg.signature_sum(), msg.sign_pk())?;
        self.update_masks(msg.mask_url());
        Ok(())
    }

    /// Validate a certificate (dummy).
    fn validate_certificate(certificate: &[u8]) -> Result<(), PetError> {
        if certificate == b"" {
            return Ok(());
        }
        Err(PetError::InvalidMessage)
    }

    /// Validate a sum signature and its implied task.
    fn validate_task_sum(
        &self,
        signature_sum: &sign::Signature,
        sign_pk: &sign::PublicKey,
    ) -> Result<(), PetError> {
        if !sign::verify_detached(
            signature_sum,
            &[self.seed.as_slice(), b"sum"].concat(),
            sign_pk,
        ) {
            return Err(PetError::InvalidMessage);
        }

        if !is_eligible(signature_sum, self.sum) {
            return Err(PetError::InvalidMessage);
        }
        Ok(())
    }

    /// Validate an update signature and its implied task.
    fn validate_task_update(
        &self,
        signature_sum: &sign::Signature,
        signature_update: &sign::Signature,
        sign_pk: &sign::PublicKey,
    ) -> Result<(), PetError> {
        if !sign::verify_detached(
            signature_sum,
            &[self.seed.as_slice(), b"sum"].concat(),
            sign_pk,
        ) {
            return Err(PetError::InvalidMessage);
        }

        if !sign::verify_detached(
            signature_update,
            &[self.seed.as_slice(), b"update"].concat(),
            sign_pk,
        ) {
            return Err(PetError::InvalidMessage);
        }

        if is_eligible(signature_sum, self.sum) || !is_eligible(signature_update, self.update) {
            return Err(PetError::InvalidMessage);
        }
        Ok(())
    }

    /// Update the sum dictionary.
    fn update_dict_sum(&mut self, encr_pk: &box_::PublicKey, ephm_pk: &box_::PublicKey) {
        self.dict_sum.insert(*encr_pk, *ephm_pk);
    }

    #[allow(clippy::unit_arg)]
    /// Freeze the sum dictionary.
    fn freeze_dict_sum(&mut self) -> Result<(), PetError> {
        if self.dict_sum.len() >= self.min_sum {
            self.dict_seed = self
                .dict_sum
                .keys()
                .map(|pk| (*pk, HashMap::new()))
                .collect();
            return Ok(());
        }
        Err(PetError::InsufficientParticipants)
    }

    #[allow(clippy::unit_arg)]
    /// Update the seed dictionary.
    fn update_dict_seed(
        &mut self,
        encr_pk: &box_::PublicKey,
        dict_seed: &HashMap<box_::PublicKey, Vec<u8>>,
    ) -> Result<(), PetError> {
        if dict_seed.len() != self.dict_sum.len() {
            return Err(PetError::InvalidMessage);
        }

        if dict_seed.keys().all(|pk| self.dict_sum.contains_key(pk)) {
            return Err(PetError::InvalidMessage);
        }

        for (pk, seed) in dict_seed.iter() {
            self.dict_seed
                .get_mut(pk)
                // UNWRAP_SAFE: It is fine to unwrap because:
                //
                //   - we checked just above that self.dict_sum has
                //     all the keys
                //   - self.dict_seed has the same keys than
                //     self.dict_sum, since free_dict_sum()
                //     populated it.
                .unwrap()
                .insert(*encr_pk, seed.clone());
        }

        Ok(())
    }

    /// Freeze the seed dictionary.
    fn freeze_dict_seed(&mut self) -> Result<(), PetError> {
        if self
            .dict_seed
            .values()
            .all(|dict| dict.len() >= self.min_update)
        {
            return Ok(());
        }
        Err(PetError::InsufficientParticipants)
    }

    /// Update the mask dictionary.
    fn update_masks(&mut self, mask_url: &[u8]) {
        self.masks += mask_url
            .chunks_exact(mask_url.len())
            .map(|mask| mask.to_vec());
    }

    /// Freeze the mask dictionary.
    fn freeze_masks(&self) -> Result<Vec<u8>, PetError> {
        let counts = self.masks.most_common();
        let sum = counts.iter().map(|(_, count)| count).sum::<usize>();
        if sum >= self.min_sum && (counts.len() == 1 || counts[0].1 > counts[1].1) {
            return Ok(counts[0].0.clone());
        }
        Err(PetError::InsufficientParticipants)
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

    /// Clear the round dictionaries.
    fn clear_round_dicts(&mut self) {
        self.dict_sum = HashMap::new();
        self.dict_seed = HashMap::new();
        self.masks = Counter::new();
    }

    /// Generate fresh round credentials.
    fn gen_round_keypairs(&mut self) {
        let (encr_pk, encr_sk) = box_::gen_keypair();
        self.encr_pk = encr_pk;
        self.encr_sk = encr_sk;
        let (sign_pk, sign_sk) = sign::gen_keypair();
        self.sign_pk = sign_pk;
        self.sign_sk = sign_sk;
    }

    /// Update the coordinator to start a round and proceed to the sum phase.
    pub fn start_round(&mut self) {
        self.update_round_sum();
        self.update_round_update();
        self.update_round_seed();
        self.clear_round_dicts();
        self.gen_round_keypairs();
        self.phase = Phase::Sum;
    }

    /// End the sum phase and proceed to the update phase.
    pub fn end_phase_sum(&mut self) -> Result<(), PetError> {
        self.freeze_dict_sum()?;
        self.phase = Phase::Update;
        Ok(())
    }

    /// End the update phase and proceed to the sum2 phase.
    pub fn end_phase_update(&mut self) -> Result<(), PetError> {
        self.freeze_dict_seed()?;
        self.phase = Phase::Sum2;
        Ok(())
    }

    /// Freeze the globals masks to end the sum2 phase and proceed to the idle phase to end the
    /// round. Returns the unique global mask url.
    pub fn end_phase_sum2(&mut self) -> Result<Vec<u8>, PetError> {
        let mask_url = self.freeze_masks()?;
        self.phase = Phase::Idle;
        Ok(mask_url)
    }
}
