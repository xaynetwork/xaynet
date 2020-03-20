use super::{utils::is_eligible, PetError};
use sodiumoxide::{
    crypto::{box_, sealedbox, sign},
    init,
};

pub enum Task {
    Sum,
    Update,
    None,
}

pub struct Message {
    sum: f64,
    update: f64,
    seed: Vec<u8>,
}

impl Message {
    pub fn new(sum: f64, update: f64, seed: Vec<u8>) -> Self {
        Self { sum, update, seed }
    }

    pub fn check_task(&self, part_sign_sk: &sign::SecretKey) -> Result<(Vec<u8>, Task), PetError> {
        let signature = [
            sign::sign_detached(&[&self.seed[..], &b"sum"[..]].concat(), part_sign_sk)
                .0
                .to_vec(),
            sign::sign_detached(&[&self.seed[..], &b"update"[..]].concat(), part_sign_sk)
                .0
                .to_vec(),
        ]
        .concat();
        if is_eligible(&signature[0..64], self.sum).ok_or(PetError::InvalidMessage)? {
            return Ok((signature, Task::Sum));
        }
        if is_eligible(&signature[64..128], self.update).ok_or(PetError::InvalidMessage)? {
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
}
