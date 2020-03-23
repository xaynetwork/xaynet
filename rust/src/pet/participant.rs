use std::collections::HashMap;

use sodiumoxide::{
    crypto::{box_, sealedbox, sign},
    init,
    randombytes::randombytes,
};

use super::{utils::is_eligible, PetError};

pub enum Task {
    Sum,
    Update,
    None,
}

pub struct Message {
    round_sum: f64,
    round_update: f64,
    round_seed: Vec<u8>,
}

impl Message {
    pub fn new(round_sum: f64, round_update: f64, round_seed: Vec<u8>) -> Self {
        Self {
            round_sum,
            round_update,
            round_seed,
        }
    }

    pub fn check_task(&self, part_sign_sk: &sign::SecretKey) -> Result<(Vec<u8>, Task), PetError> {
        let signature = [
            &sign::sign_detached(&[&self.round_seed[..], &b"sum"[..]].concat(), part_sign_sk).0[..],
            &sign::sign_detached(
                &[&self.round_seed[..], &b"update"[..]].concat(),
                part_sign_sk,
            )
            .0[..],
        ]
        .concat();
        if is_eligible(&signature[0..64], self.round_sum).ok_or(PetError::InvalidMessage)? {
            return Ok((signature, Task::Sum));
        }
        if is_eligible(&signature[64..128], self.round_update).ok_or(PetError::InvalidMessage)? {
            return Ok((signature, Task::Update));
        }
        Ok((signature, Task::None))
    }

    /// Generate an ephemeral asymmetric key pair and encrypt the "sum" message parts.
    /// Eligibility for the "sum" task should be checked beforehand.
    pub fn compose_sum(
        coord_encr_pk: &box_::PublicKey,
        part_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
        part_sign_pk: &sign::PublicKey,
        certificate: &[u8],
        signature: &[u8],
    ) -> Result<(box_::PublicKey, box_::SecretKey, Vec<u8>), PetError> {
        // initialize csprng
        init().or(Err(PetError::InvalidMessage))?;

        // generate ephemeral key pair
        let (part_ephm_pk, part_ephm_sk) = box_::gen_keypair();

        // encrypt message parts
        let nonce = box_::gen_nonce();
        let message = [
            sealedbox::seal(
                // 48 bytes +
                &[
                    &part_encr_pk.0[..], // 32 bytes
                    &part_sign_pk.0[..], // 32 bytes
                    &b"round"[..],       // 5 bytes
                ]
                .concat(),
                coord_encr_pk,
            ),
            nonce.0.to_vec(), // 24 bytes
            box_::seal(
                // 16 bytes +
                &[
                    certificate,         // 0 bytes (dummy)
                    signature,           // 128 bytes
                    &b"sum"[..],         // 3 bytes
                    &part_ephm_pk.0[..], // 32 bytes
                ]
                .concat(),
                &nonce,
                coord_encr_pk,
                part_encr_sk,
            ),
        ]
        .concat(); // 320 bytes in total

        Ok((part_ephm_pk, part_ephm_sk, message))
    }

    /// Mask a trained local model, create a dictionary of encrypted masking seeds and
    /// encrypt the "update" message parts. Eligibility for the "update" task should be
    /// checked beforehand.
    pub fn compose_update(
        coord_encr_pk: &box_::PublicKey,
        part_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
        part_sign_pk: &sign::PublicKey,
        certificate: &[u8],
        signature: &[u8],
        dict_sum: &HashMap<box_::PublicKey, box_::PublicKey>,
    ) -> Result<Vec<u8>, PetError> {
        // initialize csprng
        init().or(Err(PetError::InvalidMessage))?;

        // mask the local model
        let mask_seed = randombytes(32_usize);
        let model_url = randombytes(32_usize); // dummy

        // create dictionary of encrypted masking seeds
        let mut dict_seed: Vec<u8> = Vec::new();
        for (sum_encr_pk, sum_ephm_pk) in dict_sum.iter() {
            dict_seed.extend(sum_encr_pk.0.to_vec()); // 32 bytes
            dict_seed.extend(sealedbox::seal(&mask_seed, sum_ephm_pk)); // 48 + 32 bytes
        } // 112 * dict_sum.len() bytes in total

        // encrypt message parts
        let nonce = box_::gen_nonce();
        let message = [
            sealedbox::seal(
                // 48 bytes +
                &[
                    &part_encr_pk.0[..], // 32 bytes
                    &part_sign_pk.0[..], // 32 bytes
                    &b"round"[..],       // 5 bytes
                ]
                .concat(),
                coord_encr_pk,
            ),
            nonce.0.to_vec(), // 24 bytes
            box_::seal(
                // 16 bytes +
                &[
                    certificate,    // 0 bytes (dummy)
                    signature,      // 128 bytes
                    &b"update"[..], // 6 bytes
                    &model_url,     // 32 bytes
                    &dict_seed,     // 112 * dict_sum.len() bytes
                ]
                .concat(),
                &nonce,
                coord_encr_pk,
                part_encr_sk,
            ),
        ]
        .concat(); // 323 + 112 * dict_sum.len() bytes in total

        Ok(message)
    }

    pub fn compose_sum2(
        coord_encr_pk: &box_::PublicKey,
        part_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
        part_sign_pk: &sign::PublicKey,
        certificate: &[u8],
        signature: &[u8],
        dict_seed: &HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    ) -> Result<Vec<u8>, PetError> {
        // initialize csprng
        init().or(Err(PetError::InvalidMessage))?;

        // compute global mask
        let mut seeds: Vec<Vec<u8>> = Vec::new();
        for seed in dict_seed
            .get(part_encr_pk)
            .ok_or(PetError::InvalidMessage)?
            .values()
        {
            seeds.append(&mut vec![sealedbox::open(seed, part_encr_pk, part_encr_sk)
                .or(Err(PetError::InvalidMessage))?]);
        }
        let mask_url = randombytes(32_usize); // dummy

        // encrypt message parts
        let nonce = box_::gen_nonce();

        let message = [
            sealedbox::seal(
                // 48 bytes +
                &[
                    &part_encr_pk.0[..], // 32 bytes
                    &part_sign_pk.0[..], // 32 bytes
                    &b"round"[..],       // 5 bytes
                ]
                .concat(),
                coord_encr_pk,
            ),
            nonce.0.to_vec(), // 24 bytes
            box_::seal(
                // 16 bytes +
                &[
                    certificate,  // 0 bytes (dummy)
                    signature,    // 128 bytes
                    &b"sum2"[..], // 4 bytes
                    &mask_url,    // 32 bytes
                ]
                .concat(),
                &nonce,
                coord_encr_pk,
                part_encr_sk,
            ),
        ]
        .concat(); // 321 bytes in total

        Ok(message)
    }
}
