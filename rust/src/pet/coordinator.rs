use std::{collections::HashMap, iter::Iterator, ops::Range};

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{utils::is_eligible, PetError};

pub struct SumMessageBuffer(Vec<u8>);
pub struct UpdateMessageBuffer(Vec<u8>);
pub struct SealedBoxBuffer(Vec<u8>);
pub struct SumBoxBuffer(Vec<u8>);
pub struct UpdateBoxBuffer(Vec<u8>);

pub struct SumMessage {
    sum_encr_pk: box_::PublicKey,
    sum_ephm_pk: box_::PublicKey,
}
pub struct UpdateMessage {
    model_url: Vec<u8>,
    dict_seed: HashMap<box_::PublicKey, Vec<u8>>,
}

impl SumMessageBuffer {
    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        if message.len() != 320 {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self(message))
    }

    pub fn get_sealedbox(&self) -> &[u8] {
        &self.0[0..117]
    }

    pub fn get_nonce(&self) -> Result<box_::Nonce, PetError> {
        box_::Nonce::from_slice(&self.0[117..141]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_box(&self) -> &[u8] {
        &self.0[141..320]
    }
}

impl UpdateMessageBuffer {
    pub fn new(message: Vec<u8>, dict_sum_len: usize) -> Result<Self, PetError> {
        if message.len() != 323 + 112 * dict_sum_len {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self(message))
    }

    pub fn get_sealedbox(&self) -> &[u8] {
        &self.0[0..117]
    }

    pub fn get_nonce(&self) -> Result<box_::Nonce, PetError> {
        box_::Nonce::from_slice(&self.0[117..141]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_box(&self, dict_sum_len: usize) -> &[u8] {
        &self.0[141..323 + 112 * dict_sum_len]
    }
}

impl SealedBoxBuffer {
    pub fn new(message: Result<Vec<u8>, ()>) -> Result<Self, PetError> {
        if let Ok(msg) = message {
            if msg.len() != 69 {
                return Err(PetError::InvalidMessage);
            }
            if &msg[64..69] != b"round" {
                return Err(PetError::InvalidMessage);
            }
            Ok(Self(msg))
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    pub fn get_part_encr_pk(&self) -> Result<box_::PublicKey, PetError> {
        box_::PublicKey::from_slice(&self.0[0..32]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_part_sign_pk(&self) -> Result<sign::PublicKey, PetError> {
        sign::PublicKey::from_slice(&self.0[32..64]).ok_or(PetError::InvalidMessage)
    }
}

impl SumBoxBuffer {
    pub fn new(message: Result<Vec<u8>, ()>) -> Result<Self, PetError> {
        if let Ok(msg) = message {
            if msg.len() != 163 {
                return Err(PetError::InvalidMessage);
            }
            if &msg[128..131] != b"sum" {
                return Err(PetError::InvalidMessage);
            }
            Ok(Self(msg))
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    // dummy
    pub fn get_certificate(&self) -> &[u8] {
        &self.0[0..0]
    }

    pub fn get_signature_sum(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[0..64]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_part_ephm_pk(&self) -> Result<box_::PublicKey, PetError> {
        box_::PublicKey::from_slice(&self.0[131..163]).ok_or(PetError::InvalidMessage)
    }
}

impl UpdateBoxBuffer {
    pub fn new(message: Result<Vec<u8>, ()>, dict_sum_len: usize) -> Result<Self, PetError> {
        if let Ok(msg) = message {
            if msg.len() != 166 + 112 * dict_sum_len {
                return Err(PetError::InvalidMessage);
            }
            if &msg[128..134] != b"update" {
                return Err(PetError::InvalidMessage);
            }
            Ok(Self(msg))
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    // dummy
    pub fn get_certificate(&self) -> &[u8] {
        &self.0[0..0]
    }

    pub fn get_signature_sum(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[0..64]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_signature_update(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[64..128]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_model_url(&self) -> Vec<u8> {
        self.0[134..166].to_vec()
    }

    pub fn get_dict_seed(
        &self,
        dict_sum_len: usize,
    ) -> Result<HashMap<box_::PublicKey, Vec<u8>>, PetError> {
        let mut dict_seed: HashMap<box_::PublicKey, Vec<u8>> = HashMap::new();
        for i in (166..166 + 112 * dict_sum_len).step_by(112) {
            dict_seed.insert(
                box_::PublicKey::from_slice(&self.0[i..i + 32]).ok_or(PetError::InvalidMessage)?,
                self.0[i + 32..i + 112].to_vec(),
            );
        }
        if dict_seed.len() != dict_sum_len {
            return Err(PetError::InvalidMessage);
        }
        Ok(dict_seed)
    }
}

impl SumMessage {
    /// Decrypt and validate the message from a "sum" participant to get an item for the
    /// dictionary of "sum" participants.
    pub fn validate(
        message: Vec<u8>,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
        round_seed: &[u8],
        round_sum: f64,
    ) -> Result<Self, PetError> {
        let msg = SumMessageBuffer::new(message)?;

        // get public keys
        let sealedbox = SealedBoxBuffer::new(sealedbox::open(
            msg.get_sealedbox(),
            coord_encr_pk,
            coord_encr_sk,
        ))?;
        let sum_encr_pk = sealedbox.get_part_encr_pk()?;

        // get ephemeral key
        let nonce = msg.get_nonce()?;
        let sumbox = SumBoxBuffer::new(box_::open(
            msg.get_box(),
            &nonce,
            &sum_encr_pk,
            coord_encr_sk,
        ))?;
        Self::validate_certificate(sumbox.get_certificate())?;
        Self::validate_signature(
            &sumbox.get_signature_sum()?,
            &sealedbox.get_part_sign_pk()?,
            round_seed,
            round_sum,
        )?;
        let sum_ephm_pk = sumbox.get_part_ephm_pk()?;

        Ok(Self {
            sum_encr_pk,
            sum_ephm_pk,
        })
    }

    // dummy
    fn validate_certificate(certificate: &[u8]) -> Result<(), PetError> {
        if certificate != b"" {
            return Err(PetError::InvalidMessage);
        }
        Ok(())
    }

    fn validate_signature(
        sig: &sign::Signature,
        sign_pk: &sign::PublicKey,
        seed: &[u8],
        sum: f64,
    ) -> Result<(), PetError> {
        if sign::verify_detached(sig, &[seed, &b"sum"[..]].concat(), sign_pk)
            && is_eligible(&sig.0[..], sum).ok_or(PetError::InvalidMessage)?
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

impl UpdateMessage {
    /// Decrypt and validate the message parts from an "update" participant to get the
    /// url to the masked local model and items for the dictionary of encrypted seeds.
    pub fn validate(
        message: Vec<u8>,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
        round_seed: &[u8],
        round_sum: f64,
        round_update: f64,
        dict_sum_len: usize,
    ) -> Result<Self, PetError> {
        let msg = UpdateMessageBuffer::new(message, dict_sum_len)?;

        // get public keys
        let sealedbox = SealedBoxBuffer::new(sealedbox::open(
            msg.get_sealedbox(),
            coord_encr_pk,
            coord_encr_sk,
        ))?;
        let sum_encr_pk = sealedbox.get_part_encr_pk()?;

        // get model url and dictionary of encrypted seeds
        let nonce = msg.get_nonce()?;
        let updatebox = UpdateBoxBuffer::new(
            box_::open(
                msg.get_box(dict_sum_len),
                &nonce,
                &sum_encr_pk,
                coord_encr_sk,
            ),
            dict_sum_len,
        )?;
        Self::validate_certificate(updatebox.get_certificate())?;
        Self::validate_signature(
            &updatebox.get_signature_sum()?,
            &updatebox.get_signature_update()?,
            &sealedbox.get_part_sign_pk()?,
            round_seed,
            round_sum,
            round_update,
        )?;
        let model_url = updatebox.get_model_url();
        let dict_seed = updatebox.get_dict_seed(dict_sum_len)?;

        Ok(Self {
            model_url,
            dict_seed,
        })
    }

    // dummy
    fn validate_certificate(certificate: &[u8]) -> Result<(), PetError> {
        if certificate != b"" {
            return Err(PetError::InvalidMessage);
        }
        Ok(())
    }

    fn validate_signature(
        sig_sum: &sign::Signature,
        sig_update: &sign::Signature,
        sign_pk: &sign::PublicKey,
        seed: &[u8],
        sum: f64,
        update: f64,
    ) -> Result<(), PetError> {
        if sign::verify_detached(sig_sum, &[seed, &b"sum"[..]].concat(), sign_pk)
            && sign::verify_detached(sig_update, &[seed, &b"update"[..]].concat(), sign_pk)
            && !is_eligible(&sig_sum.0[..], sum).ok_or(PetError::InvalidMessage)?
            && is_eligible(&sig_update.0[..], update).ok_or(PetError::InvalidMessage)?
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}
