#![allow(dead_code)] // temporary

use std::{collections::HashMap, default::Default};

use sodiumoxide::{
    self,
    crypto::{box_, sealedbox, sign},
    randombytes::randombytes,
};

use super::{utils::is_eligible, PetError};

/// Tasks of a participant.
enum Task {
    Sum,
    Update,
    None,
}

/// A participant in the PET protocol layer.
pub struct Participant {
    // credentials
    encr_pk: box_::PublicKey,
    encr_sk: box_::SecretKey,
    sign_pk: sign::PublicKey,
    sign_sk: sign::SecretKey,
    ephm_pk: box_::PublicKey,
    ephm_sk: box_::SecretKey,
    certificate: Vec<u8>,
    signature_sum: sign::Signature,
    signature_update: sign::Signature,

    // other
    task: Task,
}

impl Participant {
    pub fn new() -> Result<Self, PetError> {
        // crucial: must be called before anything else in this module
        sodiumoxide::init()
            .and(Ok(Default::default()))
            .or(Err(PetError::InvalidMessage))
    }

    /// Compute the "sum" and "update" signatures.
    pub fn compute_signature(&mut self, round_seed: &[u8]) {
        self.signature_sum = sign::sign_detached(&[round_seed, b"sum"].concat(), &self.sign_sk);
        self.signature_update =
            sign::sign_detached(&[round_seed, b"update"].concat(), &self.sign_sk);
    }

    /// Check eligibility for a task.
    pub fn check_task(&mut self, round_sum: f64, round_update: f64) -> Result<(), PetError> {
        if is_eligible(&self.signature_sum, round_sum).ok_or(PetError::InvalidMessage)? {
            self.task = Task::Sum;
            Ok(())
        } else if is_eligible(&self.signature_update, round_update)
            .ok_or(PetError::InvalidMessage)?
        {
            self.task = Task::Update;
            Ok(())
        } else {
            self.task = Task::None;
            Ok(())
        }
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

/// Buffer and wrap the asymmetrically encrypted part of a "sum/update/sum2" message.
pub struct SealedBoxBuffer(Vec<u8>);

impl SealedBoxBuffer {
    pub fn new(encr_pk: &box_::PublicKey, sign_pk: &sign::PublicKey) -> Self {
        Self(
            [
                b"round",       // 5 bytes
                &encr_pk.0[..], // 32 bytes
                &sign_pk.0[..], // 32 bytes
            ]
            .concat(),
        ) // 69 bytes in total
    }

    pub fn seal(&self, coord_encr_pk: &box_::PublicKey) -> Vec<u8> {
        let sbox = sealedbox::seal(&self.0[..], coord_encr_pk); // 48 + 69 bytes
        sbox // 117 bytes in total
    }
}

/// Buffer and wrap the symmetrically encrypted part of a "sum" message.
pub struct SumBoxBuffer(Vec<u8>);

impl SumBoxBuffer {
    pub fn new(
        certificate: &[u8],
        signature_sum: &sign::Signature,
        signature_update: &sign::Signature,
        ephm_pk: &box_::PublicKey,
    ) -> Self {
        Self(
            [
                b"sum",                  // 3 bytes
                certificate,             // 0 bytes (dummy)
                &signature_sum.0[..],    // 64 bytes
                &signature_update.0[..], // 64 bytes
                &ephm_pk.0[..],          // 32 bytes
            ]
            .concat(),
        ) // 163 bytes in total
    }

    pub fn seal(&self, coord_encr_pk: &box_::PublicKey, part_encr_sk: &box_::SecretKey) -> Vec<u8> {
        let nonce = box_::gen_nonce(); // 24 bytes
        let sumbox = box_::seal(&self.0[..], &nonce, coord_encr_pk, part_encr_sk); // 16 + 163 bytes
        [nonce.0.to_vec(), sumbox].concat() // 203 bytes in total
    }
}

/// Buffer and wrap the symmetrically encrypted part of an "update" message.
pub struct UpdateBoxBuffer(Vec<u8>);

impl UpdateBoxBuffer {
    pub fn new(
        certificate: &[u8],
        signature_sum: &sign::Signature,
        signature_update: &sign::Signature,
        model_url: &[u8],
        dict_seed: &[u8],
    ) -> Self {
        Self(
            [
                b"update",               // 6 bytes
                certificate,             // 0 bytes (dummy)
                &signature_sum.0[..],    // 64 bytes
                &signature_update.0[..], // 64 bytes
                model_url,               // 32 bytes (dummy)
                dict_seed,               // 112 * dict_sum.len() bytes
            ]
            .concat(),
        ) // 166 + 112 * dict_sum.len() bytes in total
    }

    pub fn seal(&self, coord_encr_pk: &box_::PublicKey, part_encr_sk: &box_::SecretKey) -> Vec<u8> {
        let nonce = box_::gen_nonce(); // 24 bytes
        let updatebox = box_::seal(&self.0[..], &nonce, coord_encr_pk, part_encr_sk); // 16 + 166 + 112 * dict_sum.len() bytes
        [nonce.0.to_vec(), updatebox].concat() // 206 + 112 * dict_sum.len() bytes in total
    }
}

/// Buffer and wrap the symmetrically encrypted part of a "sum2" message.
pub struct Sum2BoxBuffer(Vec<u8>);

impl Sum2BoxBuffer {
    pub fn new(
        certificate: &[u8],
        signature_sum: &sign::Signature,
        signature_update: &sign::Signature,
        mask_url: &[u8],
    ) -> Self {
        Self(
            [
                &b"sum2"[..],            // 4 bytes
                certificate,             // 0 bytes (dummy)
                &signature_sum.0[..],    // 64 bytes
                &signature_update.0[..], // 64 bytes
                mask_url,                // 32 bytes (dummy)
            ]
            .concat(),
        ) // 164 bytes in total
    }

    pub fn seal(&self, coord_encr_pk: &box_::PublicKey, part_encr_sk: &box_::SecretKey) -> Vec<u8> {
        let nonce = box_::gen_nonce(); // 24 bytes
        let sum2box = box_::seal(&self.0[..], &nonce, coord_encr_pk, part_encr_sk); // 16 + 164 bytes
        [nonce.0.to_vec(), sum2box].concat() // 204 bytes in total
    }
}

/// Buffer and wrap an encrypted "sum" message.
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

/// Buffer and wrap an encrypted "update" message.
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

/// Buffer and wrap an encrypted "sum2" message.
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

/// Compose and encrypt a "sum" message. Get an ephemeral asymmetric key pair.
pub struct SumMessage {
    message: SumMessageBuffer,
    part_ephm_pk: box_::PublicKey,
    part_ephm_sk: box_::SecretKey,
}

impl SumMessage {
    pub fn compose(part: &Participant, coord_encr_pk: &box_::PublicKey) -> Self {
        // generate ephemeral key pair
        let (part_ephm_pk, part_ephm_sk) = box_::gen_keypair();

        // encrypt message parts
        let sbox = SealedBoxBuffer::new(&part.encr_pk, &part.sign_pk).seal(coord_encr_pk);
        let sumbox = SumBoxBuffer::new(
            &part.certificate,
            &part.signature_sum,
            &part.signature_update,
            &part_ephm_pk,
        )
        .seal(coord_encr_pk, &part.encr_sk);
        let message = SumMessageBuffer::new(&sbox, &sumbox);

        Self {
            message,
            part_ephm_pk,
            part_ephm_sk,
        }
    }
}

/// Compose and encrypt an "update" message. Get a seed of a local model mask.
pub struct UpdateMessage {
    message: UpdateMessageBuffer,
    mask_seed: Vec<u8>,
}

impl UpdateMessage {
    pub fn compose(
        part: &Participant,
        coord_encr_pk: &box_::PublicKey,
        dict_sum: &HashMap<box_::PublicKey, box_::PublicKey>,
    ) -> Self {
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
        let sbox = SealedBoxBuffer::new(&part.encr_pk, &part.sign_pk).seal(coord_encr_pk);
        let updatebox = UpdateBoxBuffer::new(
            &part.certificate,
            &part.signature_sum,
            &part.signature_update,
            &model_url,
            &dict_seed,
        )
        .seal(coord_encr_pk, &part.encr_sk);
        let message = UpdateMessageBuffer::new(&sbox, &updatebox);

        Self { message, mask_seed }
    }
}

/// Compose and encrypt a "sum" message. Get an url of a global mask.
pub struct Sum2Message {
    message: Sum2MessageBuffer,
    mask_url: Vec<u8>,
}

impl Sum2Message {
    pub fn compose(
        part: &Participant,
        coord_encr_pk: &box_::PublicKey,
        dict_seed: &HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
    ) -> Result<Self, PetError> {
        // compute global mask
        let mut seeds: Vec<Vec<u8>> = Vec::new();
        for seed in dict_seed
            .get(&part.encr_pk)
            .ok_or(PetError::InvalidMessage)?
            .values()
        {
            seeds.append(&mut vec![sealedbox::open(
                seed,
                &part.encr_pk,
                &part.encr_sk,
            )
            .or(Err(PetError::InvalidMessage))?]);
        }
        let mask_url = randombytes(32_usize); // dummy

        // encrypt message parts
        let sbox = SealedBoxBuffer::new(&part.encr_pk, &part.sign_pk).seal(coord_encr_pk);
        let sum2box = Sum2BoxBuffer::new(
            &part.certificate,
            &part.signature_sum,
            &part.signature_update,
            &mask_url,
        )
        .seal(coord_encr_pk, &part.encr_sk);
        let message = Sum2MessageBuffer::new(&sbox, &sum2box);

        Ok(Self { message, mask_url })
    }
}
