#![allow(dead_code)] // temporary

use std::{collections::HashMap, default::Default, iter::Iterator, ops::Range};

use sodiumoxide::{
    self,
    crypto::{box_, sealedbox, sign},
    randombytes::randombytes,
};

use super::{utils::is_eligible, PetError};

/// A coordinator in the PET protocol layer.
pub struct Coordinator {
    // credentials
    pub encr_pk: box_::PublicKey, // 32 bytes
    encr_sk: box_::SecretKey,     // 32 bytes

    // round parameters
    sum: f64,
    update: f64,
    seed: Vec<u8>, // 32 bytes

    // dictionaries
    dict_sum: HashMap<box_::PublicKey, box_::PublicKey>,
    dict_seed: HashMap<box_::PublicKey, HashMap<box_::PublicKey, Vec<u8>>>,
}

impl Coordinator {
    pub fn new() -> Result<Self, PetError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init()
            .and(Ok(Default::default()))
            .or(Err(PetError::InsufficientSystemEntropy))
    }

    /// Validate and handle a message.
    pub fn validate_message(&self, message: &[u8]) -> Result<(), PetError> {
        if let Ok(_) = SumMessage::validate(self, message) {
            Ok(()) // placeholder: deal with result
        } else if let Ok(_) = UpdateMessage::validate(self, message) {
            Ok(()) // placeholder: deal with result
        } else if let Ok(_) = Sum2Message::validate(self, message) {
            Ok(()) // placeholder: deal with result
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        let (encr_pk, encr_sk) = box_::gen_keypair();
        let sum = 0.01_f64;
        let update = 0.1_f64;
        let seed = randombytes(32_usize);
        let dict_sum = HashMap::new();
        let dict_seed = HashMap::new();
        Self {
            encr_pk,
            encr_sk,
            sum,
            update,
            seed,
            dict_sum,
            dict_seed,
        }
    }
}

// Message access with buffers:
//
// SumMessage
//  └-> SumMessageBuffer
//       ├-> SealedBox
//       |    └-> SealedBoxBuffer
//       |         ├-> encr_pk
//       |         └-> sign_pk
//       └-> SumBox
//            └-> SumBoxBuffer
//                 ├-> certificate
//                 ├-> signature_sum
//                 └-> ephm_pk
//
// UpdateMessage
//  └-> UpdateMessageBuffer
//       ├-> SealedBox
//       |    └-> SealedBoxBuffer
//       |         ├-> encr_pk
//       |         └-> sign_pk
//       └-> UpdateBox
//            └-> UpdateBoxBuffer
//                 ├-> certificate
//                 ├-> signature_sum
//                 ├-> signature_update
//                 ├-> model_url
//                 └-> dict_seed
//
// Sum2Message
//  └-> Sum2MessageBuffer
//       ├-> SealedBox
//       |    └-> SealedBoxBuffer
//       |         ├-> encr_pk
//       |         └-> sign_pk
//       └-> Sum2Box
//            └-> Sum2BoxBuffer
//                 ├-> certificate
//                 ├-> signature_sum
//                 └-> mask_url

/// Buffer and access an encrypted "sum" message.
struct SumMessageBuffer<'msg>(&'msg [u8]);

impl<'msg> SumMessageBuffer<'msg> {
    const SEALEDBOX_RANGE: Range<usize> = 0..117;
    const NONCE_RANGE: Range<usize> = 117..141;
    const BOX_RANGE: Range<usize> = 141..320;
    const MESSAGE_LENGTH: usize = 320;

    fn new(message: &'msg [u8]) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH)
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    fn open_sealedbox(
        &self,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        sealedbox::open(&self.0[Self::SEALEDBOX_RANGE], coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))
    }

    fn open_box(
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
struct UpdateMessageBuffer<'msg>(&'msg [u8], Range<usize>);

impl<'msg> UpdateMessageBuffer<'msg> {
    const SEALEDBOX_RANGE: Range<usize> = 0..117;
    const NONCE_RANGE: Range<usize> = 117..141;
    const BOX_START: usize = 141;
    const BOX_END_WO_DICT_SEED: usize = 323;
    const DICT_SEED_ITEM_LENGTH: usize = 112;
    const MESSAGE_LENGTH_WO_DICT_SEED: usize = 323;

    fn new(message: &'msg [u8], dict_sum_length: usize) -> Result<Self, PetError> {
        let box_range = Self::BOX_START
            ..Self::BOX_END_WO_DICT_SEED + Self::DICT_SEED_ITEM_LENGTH * dict_sum_length;
        let message_length =
            Self::MESSAGE_LENGTH_WO_DICT_SEED + Self::DICT_SEED_ITEM_LENGTH * dict_sum_length;
        (message.len() == message_length)
            .then_some(Self(message, box_range))
            .ok_or(PetError::InvalidMessage)
    }

    fn open_sealedbox(
        &self,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        sealedbox::open(&self.0[Self::SEALEDBOX_RANGE], coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))
    }

    fn open_box(
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
struct Sum2MessageBuffer<'msg>(&'msg [u8]);

impl<'msg> Sum2MessageBuffer<'msg> {
    const SEALEDBOX_RANGE: Range<usize> = 0..117;
    const NONCE_RANGE: Range<usize> = 117..141;
    const BOX_RANGE: Range<usize> = 141..321;
    const MESSAGE_LENGTH: usize = 321;

    fn new(message: &'msg [u8]) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH)
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    fn open_sealedbox(
        &self,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Vec<u8>, PetError> {
        sealedbox::open(&self.0[Self::SEALEDBOX_RANGE], coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))
    }

    fn open_box(
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
struct SealedBoxBuffer<'sbox>(&'sbox [u8]);

impl<'sbox> SealedBoxBuffer<'sbox> {
    const ROUND_RANGE: Range<usize> = 0..5;
    const ENCR_PK_RANGE: Range<usize> = 5..37;
    const SIGN_PK_RANGE: Range<usize> = 37..69;
    const MESSAGE_LENGTH: usize = 69;

    fn new(message: &'sbox [u8]) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH && &message[Self::ROUND_RANGE] == b"round")
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    fn get_part_encr_pk(&self) -> Result<box_::PublicKey, PetError> {
        box_::PublicKey::from_slice(&self.0[Self::ENCR_PK_RANGE]).ok_or(PetError::InvalidMessage)
    }

    fn get_part_sign_pk(&self) -> Result<sign::PublicKey, PetError> {
        sign::PublicKey::from_slice(&self.0[Self::SIGN_PK_RANGE]).ok_or(PetError::InvalidMessage)
    }
}

/// Buffer and access the symmetrically decrypted part of a "sum" message.
struct SumBoxBuffer<'box__>(&'box__ [u8]);

impl<'box__> SumBoxBuffer<'box__> {
    const SUM_RANGE: Range<usize> = 0..3;
    const CERTIFICATE_RANGE: Range<usize> = 3..3;
    const SIGN_SUM_RANGE: Range<usize> = 3..67;
    const SIGN_UPDATE_RANGE: Range<usize> = 67..131;
    const EPHM_PK_RANGE: Range<usize> = 131..163;
    const MESSAGE_LENGTH: usize = 163;

    fn new(message: &'box__ [u8]) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH && &message[Self::SUM_RANGE] == b"sum")
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    // dummy
    fn get_certificate(&self) -> Vec<u8> {
        self.0[Self::CERTIFICATE_RANGE].to_vec()
    }

    fn get_signature_sum(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[Self::SIGN_SUM_RANGE]).ok_or(PetError::InvalidMessage)
    }

    fn get_part_ephm_pk(&self) -> Result<box_::PublicKey, PetError> {
        box_::PublicKey::from_slice(&self.0[Self::EPHM_PK_RANGE]).ok_or(PetError::InvalidMessage)
    }
}

/// Buffer and access the symmetrically decrypted part of an "update" message.
struct UpdateBoxBuffer<'box__>(&'box__ [u8], usize, Range<usize>);

impl<'box__> UpdateBoxBuffer<'box__> {
    const UPDATE_RANGE: Range<usize> = 0..6;
    const CERTIFICATE_RANGE: Range<usize> = 6..6;
    const SIGN_SUM_RANGE: Range<usize> = 6..70;
    const SIGN_UPDATE_RANGE: Range<usize> = 70..134;
    const MODEL_URL_RANGE: Range<usize> = 134..166;
    const DICT_SEED_START: usize = 166;
    const DICT_SEED_KEY_LENGTH: usize = 32;
    const DICT_SEED_ITEM_LENGTH: usize = 112;
    const MESSAGE_LENGTH_WO_DICT_SEED: usize = 166;

    fn new(message: &'box__ [u8], dict_sum_length: usize) -> Result<Self, PetError> {
        let dict_seed_range = Self::DICT_SEED_START
            ..Self::DICT_SEED_START + Self::DICT_SEED_ITEM_LENGTH * dict_sum_length;
        let message_length =
            Self::MESSAGE_LENGTH_WO_DICT_SEED + Self::DICT_SEED_ITEM_LENGTH * dict_sum_length;
        (message.len() == message_length && &message[Self::UPDATE_RANGE] == b"update")
            .then_some(Self(message, dict_sum_length, dict_seed_range))
            .ok_or(PetError::InvalidMessage)
    }

    // dummy
    fn get_certificate(&self) -> Vec<u8> {
        self.0[Self::CERTIFICATE_RANGE].to_vec()
    }

    fn get_signature_sum(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[Self::SIGN_SUM_RANGE]).ok_or(PetError::InvalidMessage)
    }

    fn get_signature_update(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[Self::SIGN_UPDATE_RANGE])
            .ok_or(PetError::InvalidMessage)
    }

    fn get_model_url(&self) -> Vec<u8> {
        self.0[Self::MODEL_URL_RANGE].to_vec()
    }

    fn get_dict_seed(&self) -> Result<HashMap<box_::PublicKey, Vec<u8>>, PetError> {
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
struct Sum2BoxBuffer<'box__>(&'box__ [u8]);

impl<'box__> Sum2BoxBuffer<'box__> {
    const SUM2_RANGE: Range<usize> = 0..4;
    const CERTIFICATE_RANGE: Range<usize> = 4..4;
    const SIGN_SUM_RANGE: Range<usize> = 4..68;
    const SIGN_UPDATE_RANGE: Range<usize> = 68..132;
    const MASK_URL_RANGE: Range<usize> = 132..164;
    const MESSAGE_LENGTH: usize = 164;

    fn new(message: &'box__ [u8]) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH && &message[Self::SUM2_RANGE] == b"sum2")
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    // dummy
    fn get_certificate(&self) -> Vec<u8> {
        self.0[Self::CERTIFICATE_RANGE].to_vec()
    }

    fn get_signature_sum(&self) -> Result<sign::Signature, PetError> {
        sign::Signature::from_slice(&self.0[Self::SIGN_SUM_RANGE]).ok_or(PetError::InvalidMessage)
    }

    fn get_mask_url(&self) -> Vec<u8> {
        self.0[Self::MASK_URL_RANGE].to_vec()
    }
}

/// An item for the dictionary of "sum" participants.
pub struct SumMessage {
    pub part_encr_pk: box_::PublicKey, // 32 bytes
    pub part_ephm_pk: box_::PublicKey, // 32 bytes
}

impl SumMessage {
    /// Decrypt and validate a "sum" message. Get an item for the dictionary of "sum" participants.
    fn validate(coord: &Coordinator, message: &[u8]) -> Result<Self, PetError> {
        let msg_buf = SumMessageBuffer::new(message)?;

        // get public keys
        let sbox = msg_buf.open_sealedbox(&coord.encr_pk, &coord.encr_sk)?;
        let sbox_buf = SealedBoxBuffer::new(&sbox)?;
        let part_encr_pk = sbox_buf.get_part_encr_pk()?;
        let part_sign_pk = sbox_buf.get_part_sign_pk()?;

        // get ephemeral key
        let sumbox = msg_buf.open_box(&part_encr_pk, &coord.encr_sk)?;
        let box_buf = SumBoxBuffer::new(&sumbox)?;
        Self::validate_certificate(&box_buf.get_certificate())?;
        Self::validate_signature(
            &box_buf.get_signature_sum()?,
            &part_sign_pk,
            &coord.seed,
            coord.sum,
        )?;
        let part_ephm_pk = box_buf.get_part_ephm_pk()?;

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

/// An url for a masked local model and items for the dictionary of encrypted masking seeds.
pub struct UpdateMessage {
    pub model_url: Vec<u8>,                           // 32 bytes (dummy)
    pub dict_seed: HashMap<box_::PublicKey, Vec<u8>>, // 112 * dict_sum.len() bytes
}

impl UpdateMessage {
    /// Decrypt and validate an "update" message. Get an url to a masked local model and items for
    /// the dictionary of encrypted seeds.
    fn validate(coord: &Coordinator, message: &[u8]) -> Result<Self, PetError> {
        let msg_buf = UpdateMessageBuffer::new(message, coord.dict_sum.len())?;

        // get public keys
        let sbox = msg_buf.open_sealedbox(&coord.encr_pk, &coord.encr_sk)?;
        let sbox_buf = SealedBoxBuffer::new(&sbox)?;
        let part_encr_pk = sbox_buf.get_part_encr_pk()?;
        let part_sign_pk = sbox_buf.get_part_sign_pk()?;

        // get model url and dictionary of encrypted seeds
        let updatebox = msg_buf.open_box(&part_encr_pk, &coord.encr_sk)?;
        let box_buf = UpdateBoxBuffer::new(&updatebox, coord.dict_sum.len())?;
        Self::validate_certificate(&box_buf.get_certificate())?;
        Self::validate_signature(
            &box_buf.get_signature_sum()?,
            &box_buf.get_signature_update()?,
            &part_sign_pk,
            &coord.seed,
            coord.sum,
            coord.update,
        )?;
        let model_url = box_buf.get_model_url();
        let dict_seed = box_buf.get_dict_seed()?;

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

/// An url for a mask of a global model.
pub struct Sum2Message {
    pub mask_url: Vec<u8>,
}

impl Sum2Message {
    /// Decrypt and validate a "sum2" message. Get an url for a global mask.
    fn validate(coord: &Coordinator, message: &[u8]) -> Result<Self, PetError> {
        let msg_buf = Sum2MessageBuffer::new(message)?;

        // get public keys
        let sbox = msg_buf.open_sealedbox(&coord.encr_pk, &coord.encr_sk)?;
        let sbox_buf = SealedBoxBuffer::new(&sbox)?;
        let part_encr_pk = sbox_buf.get_part_encr_pk()?;
        let part_sign_pk = sbox_buf.get_part_sign_pk()?;

        // get ephemeral key
        let sum2box = msg_buf.open_box(&part_encr_pk, &coord.encr_sk)?;
        let box_buf = Sum2BoxBuffer::new(&sum2box)?;
        Self::validate_certificate(&box_buf.get_certificate())?;
        Self::validate_signature(
            &box_buf.get_signature_sum()?,
            &part_sign_pk,
            &coord.seed,
            coord.sum,
        )?;
        let mask_url = box_buf.get_mask_url();

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
