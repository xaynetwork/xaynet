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
    encr_pk: box_::PublicKey,          // 32 bytes
    encr_sk: box_::SecretKey,          // 32 bytes
    sign_pk: sign::PublicKey,          // 32 bytes
    sign_sk: sign::SecretKey,          // 64 bytes
    ephm_pk: box_::PublicKey,          // 32 bytes
    ephm_sk: box_::SecretKey,          // 32 bytes
    certificate: Vec<u8>,              // 0 bytes (dummy)
    signature_sum: sign::Signature,    // 64 bytes
    signature_update: sign::Signature, // 64 bytes

    // round parameters
    task: Task,
}

impl Default for Participant {
    fn default() -> Self {
        let encr_pk = box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let encr_sk = box_::SecretKey([0_u8; box_::SECRETKEYBYTES]);
        let sign_pk = sign::PublicKey([0_u8; sign::PUBLICKEYBYTES]);
        let sign_sk = sign::SecretKey([0_u8; sign::SECRETKEYBYTES]);
        let ephm_pk = box_::PublicKey([0_u8; box_::PUBLICKEYBYTES]);
        let ephm_sk = box_::SecretKey([0_u8; box_::SECRETKEYBYTES]);
        let certificate = Vec::<u8>::new();
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

impl Participant {
    /// Create a participant. Fails if there is insufficient system entropy to generate secrets.
    pub fn new() -> Result<Self, PetError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(PetError::InsufficientSystemEntropy))?;
        let (encr_pk, encr_sk) = box_::gen_keypair();
        let (sign_pk, sign_sk) = sign::gen_keypair();
        Ok(Self {
            encr_pk,
            encr_sk,
            sign_pk,
            sign_sk,
            ..Default::default()
        })
    }

    /// Compute the sum and update signatures.
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

    /// Compose a sum message.
    pub fn compose_message_sum(&mut self, coord_encr_pk: &box_::PublicKey) -> Vec<u8> {
        self.gen_ephm_keypair();
        Message::new(
            RoundBox::new(&self.encr_pk, &self.sign_pk),
            SumBox::new(&self.certificate, &self.signature_sum, &self.ephm_pk),
        )
        .seal(coord_encr_pk, &self.encr_sk)
    }

    /// Compose an update message.
    pub fn compose_message_update(
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

    /// Compose a sum2 message.
    pub fn compose_message_sum2(
        &self,
        coord_encr_pk: &box_::PublicKey,
        dict_seed: &HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    ) -> Result<Vec<u8>, PetError> {
        let mask_seeds = self.get_seeds(dict_seed)?;
        let mask_url = self.compute_global_mask(mask_seeds);
        Ok(Message::new(
            RoundBox::new(&self.encr_pk, &self.sign_pk),
            Sum2Box::new(&self.certificate, &self.signature_sum, &mask_url),
        )
        .seal(coord_encr_pk, &self.encr_sk))
    }

    /// Generate an ephemeral key pair.
    fn gen_ephm_keypair(&mut self) {
        let (ephm_pk, ephm_sk) = box_::gen_keypair();
        self.ephm_pk = ephm_pk;
        self.ephm_sk = ephm_sk;
    }

    /// Mask a local model (dummy). Returns the mask seed and the model url.
    fn mask_model() -> (Vec<u8>, Vec<u8>) {
        (randombytes(32), randombytes(32))
    }

    // Create a mask seed dictionary from a sum dictionary.
    fn create_dict_seed(
        dict_sum: &HashMap<box_::PublicKey, box_::PublicKey>,
        mask_seed: &[u8],
    ) -> HashMap<box_::PublicKey, Vec<u8>> {
        dict_sum
            .iter()
            .map(|(encr_pk, ephm_pk)| (*encr_pk, sealedbox::seal(mask_seed, ephm_pk)))
            .collect()
    }

    /// Get the mask seeds from the seed dictionary.
    fn get_seeds(
        &self,
        dict_seed: &HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    ) -> Result<Vec<Vec<u8>>, PetError> {
        dict_seed
            .get(&self.encr_pk)
            .ok_or(PetError::InvalidMessage)?
            .values()
            .map(|seed| {
                sealedbox::open(seed, &self.ephm_pk, &self.ephm_sk)
                    .or(Err(PetError::InvalidMessage))
            })
            .collect()
    }

    /// Compute a global mask from local mask seeds (dummy). Returns the mask url.
    fn compute_global_mask(&self, mask_seeds: Vec<Vec<u8>>) -> Vec<u8> {
        sha256::hash(&mask_seeds.into_iter().flatten().collect::<Vec<u8>>())
            .as_ref()
            .to_vec()
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, iter};

    use sodiumoxide::randombytes::randombytes_uniform;

    use super::*;

    #[test]
    fn test_participant() {
        // new
        let mut part = Participant::new().unwrap();
        assert_eq!(part.encr_pk, part.encr_sk.public_key());
        assert_eq!(part.encr_sk.as_ref().len(), 32);
        assert_eq!(part.sign_pk, part.sign_sk.public_key());
        assert_eq!(part.sign_sk.as_ref().len(), 64);
        assert_eq!(part.ephm_pk, box_::PublicKey([0_u8; 32]));
        assert_eq!(part.ephm_sk, box_::SecretKey([0_u8; 32]));
        assert_eq!(part.certificate, Vec::<u8>::new());
        assert_eq!(part.signature_sum, sign::Signature([0_u8; 64]));
        assert_eq!(part.signature_update, sign::Signature([0_u8; 64]));
        assert_eq!(part.task, Task::None);

        // compute signature
        let round_seed = randombytes(32);
        part.compute_signatures(&round_seed);
        assert_eq!(
            part.signature_sum,
            sign::sign_detached(&[round_seed.as_slice(), b"sum"].concat(), &part.sign_sk)
        );
        assert_eq!(
            part.signature_update,
            sign::sign_detached(&[round_seed.as_slice(), b"update"].concat(), &part.sign_sk)
        );

        // check task
        let sign_ell = sign::Signature([
            229, 191, 74, 163, 113, 6, 242, 191, 255, 225, 40, 89, 210, 94, 25, 50, 44, 129, 155,
            241, 99, 64, 25, 212, 157, 235, 102, 95, 115, 18, 158, 115, 253, 136, 178, 223, 4, 47,
            54, 162, 236, 78, 126, 114, 205, 217, 250, 163, 223, 149, 31, 65, 179, 179, 60, 64, 34,
            1, 78, 245, 1, 50, 165, 47,
        ]);
        let sign_inell = sign::Signature([
            15, 107, 81, 84, 105, 246, 165, 81, 76, 125, 140, 172, 113, 85, 51, 173, 119, 123, 78,
            114, 249, 182, 135, 212, 134, 38, 125, 153, 120, 45, 179, 55, 116, 155, 205, 51, 247,
            37, 78, 147, 63, 231, 28, 61, 251, 41, 48, 239, 125, 0, 129, 126, 194, 123, 183, 11,
            215, 220, 1, 225, 248, 131, 64, 242,
        ]);
        part.signature_sum = sign_ell;
        part.signature_update = sign_inell;
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::Sum);
        part.signature_update = sign_ell;
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::Sum);
        part.signature_sum = sign_inell;
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::Update);
        part.signature_update = sign_inell;
        part.check_task(0.5_f64, 0.5_f64);
        assert_eq!(part.task, Task::None);

        // gen ephm keypair
        part.gen_ephm_keypair();
        assert_eq!(part.ephm_pk, part.ephm_sk.public_key());
        assert_eq!(part.ephm_sk.as_ref().len(), 32);

        // mask model
        let (mask_seed, _) = Participant::mask_model();
        assert_eq!(mask_seed.len(), 32);

        // create dict seed
        let dict_ephm = iter::repeat_with(|| box_::gen_keypair())
            .take(1 + randombytes_uniform(10) as usize)
            .collect::<HashMap<box_::PublicKey, box_::SecretKey>>();
        let dict_sum = dict_ephm
            .iter()
            .map(|(ephm_pk, _)| {
                (
                    box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
                    *ephm_pk,
                )
            })
            .collect();
        let dict_seed = Participant::create_dict_seed(&dict_sum, &mask_seed);
        assert_eq!(
            dict_seed.keys().collect::<HashSet<&box_::PublicKey>>(),
            dict_sum.keys().collect::<HashSet<&box_::PublicKey>>()
        );
        assert!(dict_seed.iter().all(|(encr_pk, sealed_mask_seed)| {
            let ephm_pk = dict_sum.get(encr_pk).unwrap();
            let ephm_sk = dict_ephm.get(ephm_pk).unwrap();
            mask_seed == sealedbox::open(sealed_mask_seed, ephm_pk, ephm_sk).unwrap()
        }));

        // get seeds
        let mask_seeds = iter::repeat_with(|| randombytes(32))
            .take(1 + randombytes_uniform(10) as usize)
            .collect::<Vec<Vec<u8>>>();
        let dict_seed = [(
            part.encr_pk,
            mask_seeds
                .iter()
                .map(|seed| {
                    (
                        box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
                        sealedbox::seal(seed, &part.ephm_pk),
                    )
                })
                .collect(),
        )]
        .iter()
        .cloned()
        .collect::<HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>>();
        assert_eq!(
            part.get_seeds(&dict_seed)
                .unwrap()
                .into_iter()
                .collect::<HashSet<Vec<u8>>>(),
            mask_seeds.into_iter().collect::<HashSet<Vec<u8>>>()
        );
        assert_eq!(
            part.get_seeds(&HashMap::new()).unwrap_err(),
            PetError::InvalidMessage
        );
    }
}
