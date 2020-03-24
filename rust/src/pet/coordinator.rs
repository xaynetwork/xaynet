#![allow(dead_code)] // temporary

use std::{collections::HashMap, iter::Iterator, ops::Range};

use sodiumoxide::crypto::{box_, sealedbox, sign};

use super::{utils::is_eligible, PetError};

/// A coordinator in the PET protocol layer.
pub struct Coordinator {
    // credentials
    encr_pk: box_::PublicKey,
    encr_sk: box_::SecretKey,

    // round parameters
    sum: f64,
    update: f64,
    seed: Vec<u8>,
}

/// Buffer and access an encrypted "sum" message.
pub struct SumMessageBuffer(Vec<u8>);

impl SumMessageBuffer {
    const SEALEDBOX_RANGE: Range<usize> = 0..117;
    const NONCE_RANGE: Range<usize> = 117..141;
    const BOX_RANGE: Range<usize> = 141..320;
    const MESSAGE_LENGTH: usize = 320;

    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH)
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    pub fn open_sealedbox(
        &self,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        sealedbox::open(&self.0[Self::SEALEDBOX_RANGE], coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))
    }

    pub fn open_box(
        &self,
        part_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        box_::open(
            &self.0[Self::BOX_RANGE],
            &box_::Nonce::from_slice(&self.0[Self::NONCE_RANGE]).ok_or(PetError::InvalidMessage)?,
            part_encr_pk,
            coord_encr_sk,
        )
        .or(Err(PetError::InvalidMessage))
    }
}

/// Buffer and access an encrypted "update" message.
pub struct UpdateMessageBuffer(Vec<u8>, Range<usize>);

impl UpdateMessageBuffer {
    const SEALEDBOX_RANGE: Range<usize> = 0..117;
    const NONCE_RANGE: Range<usize> = 117..141;
    const BOX_START: usize = 141;
    const BOX_END_WO_DICT_SEED: usize = 323;
    const DICT_SEED_ITEM_LENGTH: usize = 112;
    const MESSAGE_LENGTH_WO_DICT_SEED: usize = 323;

    pub fn new(message: Vec<u8>, dict_sum_length: usize) -> Result<Self, PetError> {
        let box_range = Self::BOX_START
            ..Self::BOX_END_WO_DICT_SEED + Self::DICT_SEED_ITEM_LENGTH * dict_sum_length;
        let message_length =
            Self::MESSAGE_LENGTH_WO_DICT_SEED + Self::DICT_SEED_ITEM_LENGTH * dict_sum_length;
        (message.len() == message_length)
            .then_some(Self(message, box_range))
            .ok_or(PetError::InvalidMessage)
    }

    pub fn open_sealedbox(
        &self,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        sealedbox::open(&self.0[Self::SEALEDBOX_RANGE], coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))
    }

    pub fn open_box(
        &self,
        part_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        box_::open(
            &self.0[self.1.clone()],
            &box_::Nonce::from_slice(&self.0[Self::NONCE_RANGE]).ok_or(PetError::InvalidMessage)?,
            part_encr_pk,
            coord_encr_sk,
        )
        .or(Err(PetError::InvalidMessage))
    }
}

/// Buffer and access an encrypted "sum2" message.
pub struct Sum2MessageBuffer(Vec<u8>);

impl Sum2MessageBuffer {
    const SEALEDBOX_RANGE: Range<usize> = 0..117;
    const NONCE_RANGE: Range<usize> = 117..141;
    const BOX_RANGE: Range<usize> = 141..321;
    const MESSAGE_LENGTH: usize = 321;

    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH)
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    pub fn open_sealedbox(
        &self,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        sealedbox::open(&self.0[Self::SEALEDBOX_RANGE], coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))
    }

    pub fn open_box(
        &self,
        part_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        box_::open(
            &self.0[Self::BOX_RANGE],
            &box_::Nonce::from_slice(&self.0[Self::NONCE_RANGE]).ok_or(PetError::InvalidMessage)?,
            part_encr_pk,
            coord_encr_sk,
        )
        .or(Err(PetError::InvalidMessage))
    }
}

/// Buffer and access the asymmetrically decrypted part of a "sum/update/sum2" message.
pub struct SealedBoxBuffer(Vec<u8>);

impl SealedBoxBuffer {
    const ROUND_RANGE: Range<usize> = 0..5;
    const ENCR_PK_RANGE: Range<usize> = 5..37;
    const SIGN_PK_RANGE: Range<usize> = 37..69;
    const MESSAGE_LENGTH: usize = 69;

    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH && &message[Self::ROUND_RANGE] == b"round")
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    pub fn get_part_encr_pk(&self) -> Result<box_::PublicKey, PetError> {
        box_::PublicKey::from_slice(&self.0[Self::ENCR_PK_RANGE]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_part_sign_pk(&self) -> Result<sign::PublicKey, PetError> {
        sign::PublicKey::from_slice(&self.0[Self::SIGN_PK_RANGE]).ok_or(PetError::InvalidMessage)
    }
}

/// Buffer and access the symmetrically decrypted part of a "sum" message.
pub struct SumBoxBuffer(Vec<u8>);

impl SumBoxBuffer {
    const SUM_RANGE: Range<usize> = 0..3;
    const CERTIFICATE_RANGE: Range<usize> = 3..3;
    const SIGN_SUM_RANGE: Range<usize> = 3..67;
    const SIGN_UPDATE_RANGE: Range<usize> = 67..131;
    const EPHM_PK_RANGE: Range<usize> = 131..163;
    const MESSAGE_LENGTH: usize = 163;

    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH && &message[Self::SUM_RANGE] == b"sum")
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    // dummy
    pub fn get_certificate(&self) -> Vec<u8> {
        self.0[Self::CERTIFICATE_RANGE].to_vec()
    }

    pub fn get_signature_sum(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[Self::SIGN_SUM_RANGE]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_part_ephm_pk(&self) -> Result<box_::PublicKey, PetError> {
        box_::PublicKey::from_slice(&self.0[Self::EPHM_PK_RANGE]).ok_or(PetError::InvalidMessage)
    }
}

/// Buffer and access the symmetrically decrypted part of an "update" message.
pub struct UpdateBoxBuffer(Vec<u8>, usize, Range<usize>);

impl UpdateBoxBuffer {
    const UPDATE_RANGE: Range<usize> = 0..6;
    const CERTIFICATE_RANGE: Range<usize> = 6..6;
    const SIGN_SUM_RANGE: Range<usize> = 6..70;
    const SIGN_UPDATE_RANGE: Range<usize> = 70..134;
    const MODEL_URL_RANGE: Range<usize> = 134..166;
    const DICT_SEED_START: usize = 166;
    const DICT_SEED_KEY_LENGTH: usize = 32;
    const DICT_SEED_ITEM_LENGTH: usize = 112;
    const MESSAGE_LENGTH_WO_DICT_SEED: usize = 166;

    pub fn new(message: Vec<u8>, dict_sum_length: usize) -> Result<Self, PetError> {
        let dict_seed_range = Self::DICT_SEED_START
            ..Self::DICT_SEED_START + Self::DICT_SEED_ITEM_LENGTH * dict_sum_length;
        let message_length =
            Self::MESSAGE_LENGTH_WO_DICT_SEED + Self::DICT_SEED_ITEM_LENGTH * dict_sum_length;
        (message.len() == message_length && &message[Self::UPDATE_RANGE] == b"update")
            .then_some(Self(message, dict_sum_length, dict_seed_range))
            .ok_or(PetError::InvalidMessage)
    }

    // dummy
    pub fn get_certificate(&self) -> Vec<u8> {
        self.0[Self::CERTIFICATE_RANGE].to_vec()
    }

    pub fn get_signature_sum(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[Self::SIGN_SUM_RANGE]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_signature_update(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[Self::SIGN_UPDATE_RANGE])
            .ok_or(PetError::InvalidMessage)
    }

    pub fn get_model_url(&self) -> Vec<u8> {
        self.0[Self::MODEL_URL_RANGE].to_vec()
    }

    pub fn get_dict_seed(&self) -> Result<HashMap<box_::PublicKey, Vec<u8>>, PetError> {
        // map "sum" participants to encrypted seeds
        let mut dict_seed: HashMap<box_::PublicKey, Vec<u8>> = HashMap::new();
        for i in (self.2.clone()).step_by(Self::DICT_SEED_ITEM_LENGTH) {
            dict_seed.insert(
                box_::PublicKey::from_slice(&self.0[i..i + Self::DICT_SEED_KEY_LENGTH])
                    .ok_or(PetError::InvalidMessage)?,
                self.0[i + Self::DICT_SEED_KEY_LENGTH..i + Self::DICT_SEED_ITEM_LENGTH].to_vec(),
            );
        }
        (dict_seed.len() == self.1)
            .then_some(dict_seed)
            .ok_or(PetError::InvalidMessage)
    }
}

/// Buffer and access the symmetrically decrypted part of a "sum2" message.
pub struct Sum2BoxBuffer(Vec<u8>);

impl Sum2BoxBuffer {
    const SUM2_RANGE: Range<usize> = 0..4;
    const CERTIFICATE_RANGE: Range<usize> = 4..4;
    const SIGN_SUM_RANGE: Range<usize> = 4..68;
    const SIGN_UPDATE_RANGE: Range<usize> = 68..132;
    const MASK_URL_RANGE: Range<usize> = 132..164;
    const MESSAGE_LENGTH: usize = 164;

    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH && &message[Self::SUM2_RANGE] == b"sum2")
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    // dummy
    pub fn get_certificate(&self) -> Vec<u8> {
        self.0[Self::CERTIFICATE_RANGE].to_vec()
    }

    pub fn get_signature_sum(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[Self::SIGN_SUM_RANGE]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_mask_url(&self) -> Vec<u8> {
        self.0[Self::MASK_URL_RANGE].to_vec()
    }
}

/// Decrypt and validate a "sum" message. Get an item for the dictionary of "sum"
/// participants.
pub struct SumMessage {
    part_encr_pk: box_::PublicKey,
    part_ephm_pk: box_::PublicKey,
}

impl SumMessage {
    pub fn validate(message: Vec<u8>, coord: &Coordinator) -> Result<Self, PetError> {
        let msg = SumMessageBuffer::new(message)?;

        // get public keys
        let sbox = SealedBoxBuffer::new(msg.open_sealedbox(&coord.encr_pk, &coord.encr_sk)?)?;
        let part_encr_pk = sbox.get_part_encr_pk()?;
        let part_sign_pk = sbox.get_part_sign_pk()?;

        // get ephemeral key
        let sumbox = SumBoxBuffer::new(msg.open_box(&part_encr_pk, &coord.encr_sk)?)?;
        Self::validate_certificate(&sumbox.get_certificate())?;
        Self::validate_signature(
            &sumbox.get_signature_sum()?,
            &part_sign_pk,
            &coord.seed,
            coord.sum,
        )?;
        let part_ephm_pk = sumbox.get_part_ephm_pk()?;

        Ok(Self {
            part_encr_pk,
            part_ephm_pk,
        })
    }

    // dummy
    fn validate_certificate(certificate: &[u8]) -> Result<(), PetError> {
        (certificate == b"")
            .then_some(())
            .ok_or(PetError::InvalidMessage)
    }

    fn validate_signature(
        sign_sum: &sign::Signature,
        sign_pk: &sign::PublicKey,
        seed: &[u8],
        sum: f64,
    ) -> Result<(), PetError> {
        (sign::verify_detached(sign_sum, &[seed, b"sum"].concat(), sign_pk)
            && is_eligible(sign_sum, sum).ok_or(PetError::InvalidMessage)?)
        .then_some(())
        .ok_or(PetError::InvalidMessage)
    }
}

/// Decrypt and validate an "update" message. Get a url to a masked local model and
/// items for the dictionary of encrypted seeds.
pub struct UpdateMessage {
    model_url: Vec<u8>,
    dict_seed: HashMap<box_::PublicKey, Vec<u8>>,
}

impl UpdateMessage {
    pub fn validate(
        message: Vec<u8>,
        dict_sum_len: usize,
        coord: &Coordinator,
    ) -> Result<Self, PetError> {
        let msg = UpdateMessageBuffer::new(message, dict_sum_len)?;

        // get public keys
        let sbox = SealedBoxBuffer::new(msg.open_sealedbox(&coord.encr_pk, &coord.encr_sk)?)?;
        let part_encr_pk = sbox.get_part_encr_pk()?;
        let part_sign_pk = sbox.get_part_sign_pk()?;

        // get model url and dictionary of encrypted seeds
        let updatebox =
            UpdateBoxBuffer::new(msg.open_box(&part_encr_pk, &coord.encr_sk)?, dict_sum_len)?;
        Self::validate_certificate(&updatebox.get_certificate())?;
        Self::validate_signature(
            &updatebox.get_signature_sum()?,
            &updatebox.get_signature_update()?,
            &part_sign_pk,
            &coord.seed,
            coord.sum,
            coord.update,
        )?;
        let model_url = updatebox.get_model_url();
        let dict_seed = updatebox.get_dict_seed()?;

        Ok(Self {
            model_url,
            dict_seed,
        })
    }

    // dummy
    fn validate_certificate(certificate: &[u8]) -> Result<(), PetError> {
        (certificate == b"")
            .then_some(())
            .ok_or(PetError::InvalidMessage)
    }

    fn validate_signature(
        sign_sum: &sign::Signature,
        sign_update: &sign::Signature,
        sign_pk: &sign::PublicKey,
        seed: &[u8],
        sum: f64,
        update: f64,
    ) -> Result<(), PetError> {
        (sign::verify_detached(sign_sum, &[seed, b"sum"].concat(), sign_pk)
            && sign::verify_detached(sign_update, &[seed, b"update"].concat(), sign_pk)
            && !is_eligible(sign_sum, sum).ok_or(PetError::InvalidMessage)?
            && is_eligible(sign_update, update).ok_or(PetError::InvalidMessage)?)
        .then_some(())
        .ok_or(PetError::InvalidMessage)
    }
}

/// Decrypt and validate a "sum" message. Get an url to a global mask.
pub struct Sum2Message {
    mask_url: Vec<u8>,
}

impl Sum2Message {
    pub fn validate(message: Vec<u8>, coord: &Coordinator) -> Result<Self, PetError> {
        let msg = Sum2MessageBuffer::new(message)?;

        // get public keys
        let sbox = SealedBoxBuffer::new(msg.open_sealedbox(&coord.encr_pk, &coord.encr_sk)?)?;
        let part_encr_pk = sbox.get_part_encr_pk()?;
        let part_sign_pk = sbox.get_part_sign_pk()?;

        // get ephemeral key
        let sumbox = Sum2BoxBuffer::new(msg.open_box(&part_encr_pk, &coord.encr_sk)?)?;
        Self::validate_certificate(&sumbox.get_certificate())?;
        Self::validate_signature(
            &sumbox.get_signature_sum()?,
            &part_sign_pk,
            &coord.seed,
            coord.sum,
        )?;
        let mask_url = sumbox.get_mask_url();

        Ok(Self { mask_url })
    }

    // dummy
    fn validate_certificate(certificate: &[u8]) -> Result<(), PetError> {
        (certificate == b"")
            .then_some(())
            .ok_or(PetError::InvalidMessage)
    }

    fn validate_signature(
        sign_sum: &sign::Signature,
        sign_pk: &sign::PublicKey,
        seed: &[u8],
        sum: f64,
    ) -> Result<(), PetError> {
        (sign::verify_detached(sign_sum, &[seed, b"sum"].concat(), sign_pk)
            && is_eligible(sign_sum, sum).ok_or(PetError::InvalidMessage)?)
        .then_some(())
        .ok_or(PetError::InvalidMessage)
    }
}
