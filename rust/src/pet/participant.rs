#![allow(dead_code)] // temporary

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

pub struct SealedBoxBuffer(Vec<u8>);

impl SealedBoxBuffer {
    pub fn new(encr_pk: &box_::PublicKey, sign_pk: &sign::PublicKey) -> Self {
        Self(
            [
                &encr_pk.0[..], // 32 bytes
                &sign_pk.0[..], // 32 bytes
                b"round",       // 5 bytes
            ]
            .concat(),
        ) // 69 bytes in total
    }

    pub fn seal(&self, coord_encr_pk: &box_::PublicKey) -> Result<Vec<u8>, PetError> {
        init().or(Err(PetError::InvalidMessage))?;
        let sbox = sealedbox::seal(&self.0[..], coord_encr_pk); // 48 + 69 bytes
        Ok(sbox) // 117 bytes in total
    }
}

pub struct SumBoxBuffer(Vec<u8>);

impl SumBoxBuffer {
    pub fn new(certificate: &[u8], signature: &[u8], ephm_pk: box_::PublicKey) -> Self {
        Self(
            [
                certificate,    // 0 bytes (dummy)
                signature,      // 128 bytes
                b"sum",         // 3 bytes
                &ephm_pk.0[..], // 32 bytes
            ]
            .concat(),
        ) // 163 bytes in total
    }

    pub fn seal(
        &self,
        coord_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        init().or(Err(PetError::InvalidMessage))?;
        let nonce = box_::gen_nonce(); // 24 bytes
        let sumbox = box_::seal(&self.0[..], &nonce, coord_encr_pk, part_encr_sk); // 16 + 163 bytes
        Ok([nonce.0.to_vec(), sumbox].concat()) // 203 bytes in total
    }
}

pub struct UpdateBoxBuffer(Vec<u8>);

impl UpdateBoxBuffer {
    pub fn new(certificate: &[u8], signature: &[u8], model_url: &[u8], dict_seed: &[u8]) -> Self {
        Self(
            [
                certificate, // 0 bytes (dummy)
                signature,   // 128 bytes
                b"update",   // 6 bytes
                model_url,   // 32 bytes (dummy)
                dict_seed,   // 112 * dict_sum.len() bytes
            ]
            .concat(),
        ) // 166 + 112 * dict_sum.len() bytes in total
    }

    pub fn seal(
        &self,
        coord_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        init().or(Err(PetError::InvalidMessage))?;
        let nonce = box_::gen_nonce(); // 24 bytes
        let updatebox = box_::seal(&self.0[..], &nonce, coord_encr_pk, part_encr_sk); // 16 + 166 + 112 * dict_sum.len() bytes
        Ok([nonce.0.to_vec(), updatebox].concat()) // 206 + 112 * dict_sum.len() bytes in total
    }
}

pub struct Sum2BoxBuffer(Vec<u8>);

impl Sum2BoxBuffer {
    pub fn new(certificate: &[u8], signature: &[u8], mask_url: &[u8]) -> Self {
        Self(
            [
                certificate,  // 0 bytes (dummy)
                signature,    // 128 bytes
                &b"sum2"[..], // 4 bytes
                mask_url,     // 32 bytes (dummy)
            ]
            .concat(),
        ) // 164 bytes in total
    }

    pub fn seal(
        &self,
        coord_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        init().or(Err(PetError::InvalidMessage))?;
        let nonce = box_::gen_nonce(); // 24 bytes
        let sum2box = box_::seal(&self.0[..], &nonce, coord_encr_pk, part_encr_sk); // 16 + 164 bytes
        Ok([nonce.0.to_vec(), sum2box].concat()) // 204 bytes in total
    }
}

pub struct SumMessageBuffer(Vec<u8>);

impl SumMessageBuffer {
    pub fn new(sealedbox: &[u8], sumbox: &[u8]) -> Self {
        Self(
            [
                sealedbox, // 117 bytes
                sumbox,    // 203 bytes
            ]
            .concat(),
        ) // 320 bytes in total
    }
}

pub struct UpdateMessageBuffer(Vec<u8>);

impl UpdateMessageBuffer {
    pub fn new(sealedbox: &[u8], updatebox: &[u8]) -> Self {
        Self(
            [
                sealedbox, // 117 bytes
                updatebox, // 206 + 112 * dict_sum.len() bytes
            ]
            .concat(),
        ) // 323 + 112 * dict_sum.len() bytes in total
    }
}

pub struct Sum2MessageBuffer(Vec<u8>);

impl Sum2MessageBuffer {
    pub fn new(sealedbox: &[u8], sum2box: &[u8]) -> Self {
        Self(
            [
                sealedbox, // 117 bytes
                sum2box,   // 204 bytes
            ]
            .concat(),
        ) // 321 bytes in total
    }
}

pub struct SumMessage {
    part_ephm_pk: box_::PublicKey,
    part_ephm_sk: box_::SecretKey,
    message: SumMessageBuffer,
}

impl SumMessage {
    /// Generate an ephemeral asymmetric key pair and encrypt the "sum" message parts.
    /// Eligibility for the "sum" task should be checked beforehand.
    pub fn compose(
        coord_encr_pk: &box_::PublicKey,
        part_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
        part_sign_pk: &sign::PublicKey,
        certificate: &[u8],
        signature: &[u8],
    ) -> Result<Self, PetError> {
        // generate ephemeral key pair
        init().or(Err(PetError::InvalidMessage))?;
        let (part_ephm_pk, part_ephm_sk) = box_::gen_keypair();

        // encrypt message parts
        let sbox = SealedBoxBuffer::new(part_encr_pk, part_sign_pk).seal(coord_encr_pk)?;
        let sumbox = SumBoxBuffer::new(certificate, signature, part_ephm_pk)
            .seal(coord_encr_pk, part_encr_sk)?;
        let message = SumMessageBuffer::new(&sbox, &sumbox);
        Ok(Self {
            part_ephm_pk,
            part_ephm_sk,
            message,
        })
    }
}

pub struct UpdateMessage {
    mask_seed: Vec<u8>,
    message: UpdateMessageBuffer,
}

impl UpdateMessage {
    /// Mask a trained local model, create a dictionary of encrypted masking seeds and
    /// encrypt the "update" message parts. Eligibility for the "update" task should be
    /// checked beforehand.
    pub fn compose(
        coord_encr_pk: &box_::PublicKey,
        part_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
        part_sign_pk: &sign::PublicKey,
        certificate: &[u8],
        signature: &[u8],
        dict_sum: &HashMap<box_::PublicKey, box_::PublicKey>,
    ) -> Result<Self, PetError> {
        // mask the local model
        init().or(Err(PetError::InvalidMessage))?;
        let mask_seed = randombytes(32_usize);
        let model_url = randombytes(32_usize); // dummy

        // create dictionary of encrypted masking seeds
        let mut dict_seed: Vec<u8> = Vec::new();
        for (sum_encr_pk, sum_ephm_pk) in dict_sum.iter() {
            dict_seed.extend(sum_encr_pk.0.to_vec()); // 32 bytes
            dict_seed.extend(sealedbox::seal(&mask_seed, sum_ephm_pk)); // 48 + 32 bytes
        } // 112 * dict_sum.len() bytes in total

        // encrypt message parts
        let sbox = SealedBoxBuffer::new(part_encr_pk, part_sign_pk).seal(coord_encr_pk)?;
        let updatebox = UpdateBoxBuffer::new(certificate, signature, &model_url, &dict_seed)
            .seal(coord_encr_pk, part_encr_sk)?;
        let message = UpdateMessageBuffer::new(&sbox, &updatebox);
        Ok(Self { mask_seed, message })
    }
}

pub struct Sum2Message {
    mask_url: Vec<u8>,
    message: Sum2MessageBuffer,
}

impl Sum2Message {
    pub fn compose(
        coord_encr_pk: &box_::PublicKey,
        part_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
        part_sign_pk: &sign::PublicKey,
        certificate: &[u8],
        signature: &[u8],
        dict_seed: &HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    ) -> Result<Self, PetError> {
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
        let sbox = SealedBoxBuffer::new(part_encr_pk, part_sign_pk).seal(coord_encr_pk)?;
        let sum2box = Sum2BoxBuffer::new(certificate, signature, &mask_url)
            .seal(coord_encr_pk, part_encr_sk)?;
        let message = Sum2MessageBuffer::new(&sbox, &sum2box);
        Ok(Self { mask_url, message })
    }
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

    pub fn compute_signature(&self, part_sign_sk: &sign::SecretKey) -> Vec<u8> {
        let sum_data = [&self.round_seed[..], b"sum"].concat();
        let sum_signature = sign::sign_detached(&sum_data, part_sign_sk);

        let update_data = [&self.round_seed[..], b"update"].concat();
        let update_signature = sign::sign_detached(&update_data, part_sign_sk);

        [&sum_signature[..], &update_signature[..]].concat()
    }

    pub fn check_task(&self, part_sign_sk: &sign::SecretKey) -> Result<(Vec<u8>, Task), PetError> {
        let signature = self.compute_signature(part_sign_sk);

        if is_eligible(&signature[0..64], self.round_sum).ok_or(PetError::InvalidMessage)? {
            return Ok((signature, Task::Sum));
        }

        if is_eligible(&signature[64..128], self.round_update).ok_or(PetError::InvalidMessage)? {
            return Ok((signature, Task::Update));
        }

        Ok((signature, Task::None))
    }
}
