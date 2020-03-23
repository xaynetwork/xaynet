use std::{collections::HashMap, iter::Iterator};

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{utils::is_eligible, PetError};

pub struct SumMessageBuffer(Vec<u8>);

impl SumMessageBuffer {
    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        if message.len() != 320 {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self(message))
    }

    pub fn open_sealedbox(
        &self,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        sealedbox::open(&self.0[0..117], coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))
    }

    pub fn open_box(
        &self,
        part_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        box_::open(
            &self.0[141..320],
            &box_::Nonce::from_slice(&self.0[117..141]).ok_or(PetError::InvalidMessage)?,
            part_encr_pk,
            coord_encr_sk,
        )
        .or(Err(PetError::InvalidMessage))
    }
}

pub struct UpdateMessageBuffer(Vec<u8>);

impl UpdateMessageBuffer {
    pub fn new(message: Vec<u8>, dict_sum_len: usize) -> Result<Self, PetError> {
        if message.len() != 323 + 112 * dict_sum_len {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self(message))
    }

    pub fn open_sealedbox(
        &self,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        sealedbox::open(&self.0[0..117], coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))
    }

    pub fn open_box(
        &self,
        part_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
        dict_sum_len: usize,
    ) -> Result<Vec<u8>, PetError> {
        box_::open(
            &self.0[141..323 + 112 * dict_sum_len],
            &box_::Nonce::from_slice(&self.0[117..141]).ok_or(PetError::InvalidMessage)?,
            part_encr_pk,
            coord_encr_sk,
        )
        .or(Err(PetError::InvalidMessage))
    }
}

pub struct Sum2MessageBuffer(Vec<u8>);

impl Sum2MessageBuffer {
    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        if message.len() != 321 {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self(message))
    }

    pub fn open_sealedbox(
        &self,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        sealedbox::open(&self.0[0..117], coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))
    }

    pub fn open_box(
        &self,
        part_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        box_::open(
            &self.0[141..321],
            &box_::Nonce::from_slice(&self.0[117..141]).ok_or(PetError::InvalidMessage)?,
            part_encr_pk,
            coord_encr_sk,
        )
        .or(Err(PetError::InvalidMessage))
    }
}

pub struct SealedBoxBuffer(Vec<u8>);

impl SealedBoxBuffer {
    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        if message.len() != 69 {
            return Err(PetError::InvalidMessage);
        }
        if &message[64..69] != b"round" {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self(message))
    }

    pub fn get_part_encr_pk(&self) -> Result<box_::PublicKey, PetError> {
        box_::PublicKey::from_slice(&self.0[0..32]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_part_sign_pk(&self) -> Result<sign::PublicKey, PetError> {
        sign::PublicKey::from_slice(&self.0[32..64]).ok_or(PetError::InvalidMessage)
    }
}

pub struct SumBoxBuffer(Vec<u8>);

impl SumBoxBuffer {
    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        if message.len() != 163 {
            return Err(PetError::InvalidMessage);
        }
        if &message[128..131] != b"sum" {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self(message))
    }

    // dummy
    pub fn get_certificate(&self) -> Vec<u8> {
        self.0[0..0].to_vec()
    }

    pub fn get_signature_sum(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[0..64]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_part_ephm_pk(&self) -> Result<box_::PublicKey, PetError> {
        box_::PublicKey::from_slice(&self.0[131..163]).ok_or(PetError::InvalidMessage)
    }
}

pub struct UpdateBoxBuffer(Vec<u8>);

impl UpdateBoxBuffer {
    pub fn new(message: Vec<u8>, dict_sum_len: usize) -> Result<Self, PetError> {
        if message.len() != 166 + 112 * dict_sum_len {
            return Err(PetError::InvalidMessage);
        }
        if &message[128..134] != b"update" {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self(message))
    }

    // dummy
    pub fn get_certificate(&self) -> Vec<u8> {
        self.0[0..0].to_vec()
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

pub struct Sum2BoxBuffer(Vec<u8>);

impl Sum2BoxBuffer {
    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        if message.len() != 164 {
            return Err(PetError::InvalidMessage);
        }
        if &message[128..132] != b"sum2" {
            return Err(PetError::InvalidMessage);
        }
        Ok(Self(message))
    }

    // dummy
    pub fn get_certificate(&self) -> Vec<u8> {
        self.0[0..0].to_vec()
    }

    pub fn get_signature_sum(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[0..64]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_mask_url(&self) -> Vec<u8> {
        self.0[132..164].to_vec()
    }
}

#[allow(dead_code)] // temporary
pub struct SumMessage {
    sum_encr_pk: box_::PublicKey,
    sum_ephm_pk: box_::PublicKey,
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
        let sbox = SealedBoxBuffer::new(msg.open_sealedbox(coord_encr_pk, coord_encr_sk)?)?;
        let sum_encr_pk = sbox.get_part_encr_pk()?;

        // get ephemeral key
        let sumbox = SumBoxBuffer::new(msg.open_box(&sum_encr_pk, coord_encr_sk)?)?;
        Self::validate_certificate(&sumbox.get_certificate())?;
        Self::validate_signature(
            &sumbox.get_signature_sum()?,
            &sbox.get_part_sign_pk()?,
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
        sig_sum: &sign::Signature,
        sign_pk: &sign::PublicKey,
        seed: &[u8],
        sum: f64,
    ) -> Result<(), PetError> {
        if sign::verify_detached(sig_sum, &[seed, &b"sum"[..]].concat(), sign_pk)
            && is_eligible(&sig_sum.0[..], sum).ok_or(PetError::InvalidMessage)?
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

#[allow(dead_code)] // temporary
pub struct UpdateMessage {
    model_url: Vec<u8>,
    dict_seed: HashMap<box_::PublicKey, Vec<u8>>,
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
        let sbox = SealedBoxBuffer::new(msg.open_sealedbox(coord_encr_pk, coord_encr_sk)?)?;
        let sum_encr_pk = sbox.get_part_encr_pk()?;

        // get model url and dictionary of encrypted seeds
        let updatebox = UpdateBoxBuffer::new(
            msg.open_box(&sum_encr_pk, coord_encr_sk, dict_sum_len)?,
            dict_sum_len,
        )?;
        Self::validate_certificate(&updatebox.get_certificate())?;
        Self::validate_signature(
            &updatebox.get_signature_sum()?,
            &updatebox.get_signature_update()?,
            &sbox.get_part_sign_pk()?,
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

#[allow(dead_code)] // temporary
pub struct Sum2Message {
    mask_url: Vec<u8>,
}

impl Sum2Message {
    /// Decrypt and validate the message parts from a "sum" participant to get the url
    /// to the global mask.
    pub fn validate(
        message: Vec<u8>,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
        round_seed: &[u8],
        round_sum: f64,
    ) -> Result<Self, PetError> {
        let msg = Sum2MessageBuffer::new(message)?;

        // get public keys
        let sbox = SealedBoxBuffer::new(msg.open_sealedbox(coord_encr_pk, coord_encr_sk)?)?;
        let sum_encr_pk = sbox.get_part_encr_pk()?;

        // get ephemeral key
        let sumbox = Sum2BoxBuffer::new(msg.open_box(&sum_encr_pk, coord_encr_sk)?)?;
        Self::validate_certificate(&sumbox.get_certificate())?;
        Self::validate_signature(
            &sumbox.get_signature_sum()?,
            &sbox.get_part_sign_pk()?,
            round_seed,
            round_sum,
        )?;
        let mask_url = sumbox.get_mask_url();

        Ok(Self { mask_url })
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
        sign_pk: &sign::PublicKey,
        seed: &[u8],
        sum: f64,
    ) -> Result<(), PetError> {
        if sign::verify_detached(sig_sum, &[seed, &b"sum"[..]].concat(), sign_pk)
            && is_eligible(&sig_sum.0[..], sum).ok_or(PetError::InvalidMessage)?
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}
