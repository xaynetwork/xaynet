#![allow(dead_code)] // temporary

use std::{collections::HashMap, default::Default};

use sodiumoxide::{
    self,
    crypto::{box_, hash::sha256, sealedbox, sign},
    randombytes::randombytes,
};

use super::{
    message::{round::RoundBox, sum::SumBox, sum2::Sum2Box, update::UpdateBox, Message},
    utils::is_eligible,
    PetError,
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
    pub encr_pk: box_::PublicKey,      // 32 bytes
    encr_sk: box_::SecretKey,          // 32 bytes
    pub sign_pk: sign::PublicKey,      // 32 bytes
    sign_sk: sign::SecretKey,          // 64 bytes
    pub ephm_pk: box_::PublicKey,      // 32 bytes
    ephm_sk: box_::SecretKey,          // 32 bytes
    certificate: Vec<u8>,              // 0 bytes (dummy)
    signature_sum: sign::Signature,    // 64 bytes
    signature_update: sign::Signature, // 64 bytes

    // other
    task: Task,
}

impl Participant {
    pub fn new() -> Result<Self, PetError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init()
            .and(Ok(Default::default()))
            .or(Err(PetError::InsufficientSystemEntropy))
    }

    /// Compute the "sum" and "update" signatures.
    pub fn compute_signatures(&mut self, round_seed: &[u8]) {
        self.signature_sum = sign::sign_detached(&[round_seed, b"sum"].concat(), &self.sign_sk);
        self.signature_update =
            sign::sign_detached(&[round_seed, b"update"].concat(), &self.sign_sk);
    }

    /// Check eligibility for a task.
    pub fn check_task(&mut self, round_sum: f64, round_update: f64) {
        if is_eligible(&self.signature_sum, round_sum) {
            self.task = Task::Sum;
        } else if is_eligible(&self.signature_update, round_update) {
            self.task = Task::Update;
        } else {
            self.task = Task::None;
        }
    }

    /// Generate an ephemeral key pair.
    fn gen_ephm_keypair(&mut self) {
        let (ephm_pk, ephm_sk) = box_::gen_keypair();
        self.ephm_pk = ephm_pk;
        self.ephm_sk = ephm_sk;
    }

    /// Compose a "sum" message.
    pub fn compose_sum_message(&mut self, coord_encr_pk: &box_::PublicKey) -> Vec<u8> {
        self.gen_ephm_keypair();
        Message::new(
            RoundBox::new(&self.encr_pk, &self.sign_pk),
            SumBox::new(&self.certificate, &self.signature_sum, &self.ephm_pk),
        )
        .seal(coord_encr_pk, &self.encr_sk)
    }

    /// Mask a local model (dummy). Returns the mask seed and the model url.
    fn mask_model() -> (Vec<u8>, Vec<u8>) {
        (randombytes(32), randombytes(32))
    }

    // Create the dictionary of encrypted masking seeds from a dictionary of sum participants and
    // the mask seed.
    fn create_dict_seed(
        dict_sum: &HashMap<box_::PublicKey, box_::PublicKey>,
        mask_seed: &[u8],
    ) -> HashMap<box_::PublicKey, Vec<u8>> {
        dict_sum
            .iter()
            .map(|(sum_encr_pk, sum_ephm_pk)| {
                (sum_encr_pk.clone(), sealedbox::seal(mask_seed, sum_ephm_pk))
            })
            .collect()
    }

    /// Compose an "update" message.
    pub fn compose_update_message(
        &self,
        coord_encr_pk: &box_::PublicKey,
        dict_sum: &HashMap<box_::PublicKey, box_::PublicKey>,
    ) -> Vec<u8> {
        let (mask_seed, model_url) = Self::mask_model();
        let dict_seed = Self::create_dict_seed(dict_sum, &mask_seed);
        Message::new(
            RoundBox::new(&self.encr_pk, &self.sign_pk),
            UpdateBox::new(
                &self.certificate,
                &self.signature_sum,
                &self.signature_update,
                &model_url,
                &dict_seed,
            ),
        )
        .seal(coord_encr_pk, &self.encr_sk)
    }

    /// Compute a global mask from local mask seeds (dummy). Returns the mask url.
    fn compute_global_mask(
        &self,
        dict_seed: &HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    ) -> Result<Vec<u8>, PetError> {
        let seeds = dict_seed
            .get(&self.encr_pk)
            .ok_or(PetError::InvalidMessage)?
            .values()
            .map(|seed| {
                sealedbox::open(seed, &self.ephm_pk, &self.ephm_sk)
                    .or(Err(PetError::InvalidMessage))
            })
            .collect::<Result<Vec<Vec<u8>>, PetError>>()?;
        let model_url = sha256::hash(&seeds.into_iter().flatten().collect::<Vec<u8>>());
        Ok(model_url.as_ref().to_vec())
    }

    /// Compose a "sum2" message.
    pub fn compose_sum2_message(
        &self,
        coord_encr_pk: &box_::PublicKey,
        dict_seed: &HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    ) -> Result<Vec<u8>, PetError> {
        let mask_url = self.compute_global_mask(dict_seed)?;
        Ok(Message::new(
            RoundBox::new(&self.encr_pk, &self.sign_pk),
            Sum2Box::new(&self.certificate, &self.signature_sum, &mask_url),
        )
        .seal(coord_encr_pk, &self.encr_sk))
    }
}

impl Default for Participant {
    fn default() -> Self {
        let (encr_pk, encr_sk) = box_::gen_keypair();
        let (sign_pk, sign_sk) = sign::gen_keypair();
        let ephm_pk = box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let ephm_sk = box_::SecretKey([0_u8; box_::SECRETKEYBYTES]);
        let certificate: Vec<u8> = Vec::new();
        let signature_sum = sign::Signature([0_u8; sign::SIGNATUREBYTES]);
        let signature_update = sign::Signature([0_u8; sign::SIGNATUREBYTES]);
        let task = Task::None;
        Self {
            encr_pk,
            encr_sk,
            sign_pk,
            sign_sk,
            ephm_pk,
            ephm_sk,
            certificate,
            signature_sum,
            signature_update,
            task,
        }
    }
}
