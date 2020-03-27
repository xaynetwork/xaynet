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
            // placeholder: handle result
            Ok(())
        } else if let Ok(_) = UpdateMessage::validate(self, message) {
            // placeholder: handle result
            Ok(())
        } else if let Ok(_) = Sum2Message::validate(self, message) {
            // placeholder: handle result
            Ok(())
        } else {
            // unknown message type
            Err(PetError::InvalidMessage)
        }
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        let (encr_pk, encr_sk) = box_::gen_keypair();
        let sum = 0.01_f64;
        let update = 0.1_f64;
        let seed = randombytes(32);
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

#[derive(Debug)]
/// Buffer and access an encrypted "sum" message.
struct SumMessageBuffer<'msg>(&'msg [u8]);

impl<'msg> SumMessageBuffer<'msg> {
    const SEALEDBOX_RANGE: Range<usize> = 0..117;
    const NONCE_RANGE: Range<usize> = 117..141;
    const BOX_RANGE: Range<usize> = 141..256;
    const MESSAGE_LENGTH: usize = 256;

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

#[derive(Debug)]
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
            &box_::Nonce::from_slice(&self.0[Self::NONCE_RANGE]).unwrap(),
            part_encr_pk,
            coord_encr_sk,
        )
        .or(Err(PetError::InvalidMessage))
    }
}

#[derive(Debug)]
/// Buffer and access an encrypted "sum2" message.
struct Sum2MessageBuffer<'msg>(&'msg [u8]);

impl<'msg> Sum2MessageBuffer<'msg> {
    const SEALEDBOX_RANGE: Range<usize> = 0..117;
    const NONCE_RANGE: Range<usize> = 117..141;
    const BOX_RANGE: Range<usize> = 141..257;
    const MESSAGE_LENGTH: usize = 257;

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
            &box_::Nonce::from_slice(&self.0[Self::NONCE_RANGE]).unwrap(),
            part_encr_pk,
            coord_encr_sk,
        )
        .or(Err(PetError::InvalidMessage))
    }
}

#[derive(Debug)]
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

    fn get_part_encr_pk(&self) -> box_::PublicKey {
        box_::PublicKey::from_slice(&self.0[Self::ENCR_PK_RANGE]).unwrap()
    }

    fn get_part_sign_pk(&self) -> sign::PublicKey {
        sign::PublicKey::from_slice(&self.0[Self::SIGN_PK_RANGE]).unwrap()
    }
}

#[derive(Debug)]
/// Buffer and access the symmetrically decrypted part of a "sum" message.
struct SumBoxBuffer<'box__>(&'box__ [u8]);

impl<'box__> SumBoxBuffer<'box__> {
    const SUM_RANGE: Range<usize> = 0..3;
    const CERTIFICATE_RANGE: Range<usize> = 3..3;
    const SIGN_SUM_RANGE: Range<usize> = 3..67;
    const EPHM_PK_RANGE: Range<usize> = 67..99;
    const MESSAGE_LENGTH: usize = 99;

    fn new(message: &'box__ [u8]) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH && &message[Self::SUM_RANGE] == b"sum")
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    // dummy
    fn get_certificate(&self) -> Vec<u8> {
        self.0[Self::CERTIFICATE_RANGE].to_vec()
    }

    fn get_signature_sum(&self) -> sign::Signature {
        sign::Signature::from_slice(&self.0[Self::SIGN_SUM_RANGE]).unwrap()
    }

    fn get_part_ephm_pk(&self) -> box_::PublicKey {
        box_::PublicKey::from_slice(&self.0[Self::EPHM_PK_RANGE]).unwrap()
    }
}

#[derive(Debug)]
/// Buffer and access the symmetrically decrypted part of an "update" message.
struct UpdateBoxBuffer<'box__>(&'box__ [u8], Range<usize>);

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
            .then_some(Self(message, dict_seed_range))
            .ok_or(PetError::InvalidMessage)
    }

    // dummy
    fn get_certificate(&self) -> Vec<u8> {
        self.0[Self::CERTIFICATE_RANGE].to_vec()
    }

    fn get_signature_sum(&self) -> sign::Signature {
        sign::Signature::from_slice(&self.0[Self::SIGN_SUM_RANGE]).unwrap()
    }

    fn get_signature_update(&self) -> sign::Signature {
        sign::Signature::from_slice(&self.0[Self::SIGN_UPDATE_RANGE]).unwrap()
    }

    fn get_model_url(&self) -> Vec<u8> {
        self.0[Self::MODEL_URL_RANGE].to_vec()
    }

    fn get_dict_seed(&self) -> HashMap<box_::PublicKey, Vec<u8>> {
        // map "sum" participants to encrypted seeds
        let mut dict_seed: HashMap<box_::PublicKey, Vec<u8>> = HashMap::new();
        for i in (self.1.clone()).step_by(Self::DICT_SEED_ITEM_LENGTH) {
            dict_seed.insert(
                box_::PublicKey::from_slice(&self.0[i..i + Self::DICT_SEED_KEY_LENGTH]).unwrap(),
                self.0[i + Self::DICT_SEED_KEY_LENGTH..i + Self::DICT_SEED_ITEM_LENGTH].to_vec(),
            );
        }
        dict_seed
    }
}

#[derive(Debug)]
/// Buffer and access the symmetrically decrypted part of a "sum2" message.
struct Sum2BoxBuffer<'box__>(&'box__ [u8]);

impl<'box__> Sum2BoxBuffer<'box__> {
    const SUM2_RANGE: Range<usize> = 0..4;
    const CERTIFICATE_RANGE: Range<usize> = 4..4;
    const SIGN_SUM_RANGE: Range<usize> = 4..68;
    const MASK_URL_RANGE: Range<usize> = 68..100;
    const MESSAGE_LENGTH: usize = 100;

    fn new(message: &'box__ [u8]) -> Result<Self, PetError> {
        (message.len() == Self::MESSAGE_LENGTH && &message[Self::SUM2_RANGE] == b"sum2")
            .then_some(Self(message))
            .ok_or(PetError::InvalidMessage)
    }

    // dummy
    fn get_certificate(&self) -> Vec<u8> {
        self.0[Self::CERTIFICATE_RANGE].to_vec()
    }

    fn get_signature_sum(&self) -> sign::Signature {
        sign::Signature::from_slice(&self.0[Self::SIGN_SUM_RANGE]).unwrap()
    }

    fn get_mask_url(&self) -> Vec<u8> {
        self.0[Self::MASK_URL_RANGE].to_vec()
    }
}

#[derive(Debug)]
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
        let part_encr_pk = sbox_buf.get_part_encr_pk();
        let part_sign_pk = sbox_buf.get_part_sign_pk();

        // get ephemeral key
        let sumbox = msg_buf.open_box(&part_encr_pk, &coord.encr_sk)?;
        let box_buf = SumBoxBuffer::new(&sumbox)?;
        Self::validate_certificate(&box_buf.get_certificate())?;
        Self::validate_signature(
            &box_buf.get_signature_sum(),
            &part_sign_pk,
            &coord.seed,
            coord.sum,
        )?;
        let part_ephm_pk = box_buf.get_part_ephm_pk();

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
            && is_eligible(sign_sum, sum))
        .then_some(())
        .ok_or(PetError::InvalidMessage)
    }
}

#[derive(Debug)]
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
        let part_encr_pk = sbox_buf.get_part_encr_pk();
        let part_sign_pk = sbox_buf.get_part_sign_pk();

        // get model url and dictionary of encrypted seeds
        let updatebox = msg_buf.open_box(&part_encr_pk, &coord.encr_sk)?;
        let box_buf = UpdateBoxBuffer::new(&updatebox, coord.dict_sum.len())?;
        Self::validate_certificate(&box_buf.get_certificate())?;
        Self::validate_signature(
            &box_buf.get_signature_sum(),
            &box_buf.get_signature_update(),
            &part_sign_pk,
            &coord.seed,
            coord.sum,
            coord.update,
        )?;
        let model_url = box_buf.get_model_url();
        let dict_seed = box_buf.get_dict_seed();

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
            && !is_eligible(sign_sum, sum)
            && is_eligible(sign_update, update))
        .then_some(())
        .ok_or(PetError::InvalidMessage)
    }
}

#[derive(Debug)]
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
        let part_encr_pk = sbox_buf.get_part_encr_pk();
        let part_sign_pk = sbox_buf.get_part_sign_pk();

        // get ephemeral key
        let sum2box = msg_buf.open_box(&part_encr_pk, &coord.encr_sk)?;
        let box_buf = Sum2BoxBuffer::new(&sum2box)?;
        Self::validate_certificate(&box_buf.get_certificate())?;
        Self::validate_signature(
            &box_buf.get_signature_sum(),
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
            && is_eligible(sign_sum, sum))
        .then_some(())
        .ok_or(PetError::InvalidMessage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pet::participant::Participant;

    #[test]
    fn test_coordinator() {
        // new
        let coord = Coordinator::new().unwrap();
        assert_eq!(coord.encr_pk, coord.encr_sk.public_key());
        assert_eq!(coord.encr_sk.as_ref().len(), 32);
        assert_eq!(coord.sum, 0.01_f64);
        assert_eq!(coord.update, 0.1_f64);
        assert_eq!(coord.seed.len(), 32);
        assert_eq!(coord.dict_sum, HashMap::new());
        assert_eq!(coord.dict_seed, HashMap::new());
    }

    #[test]
    fn test_summessagebuffer() {
        let coord = Coordinator::new().unwrap();
        let mut part = Participant::new().unwrap();

        // new
        let msg = part.compose_sum_message(&coord.encr_pk).message;
        let buf = SumMessageBuffer::new(&msg).unwrap();
        assert_eq!(buf.0, msg.as_slice());

        // new error: invalid message length
        assert_eq!(
            SumMessageBuffer::new(&vec![0_u8; 0]).unwrap_err(),
            PetError::InvalidMessage
        );

        // open sealedbox
        let msg = buf.open_sealedbox(&coord.encr_pk, &coord.encr_sk).unwrap();
        assert_eq!(msg.len(), 69);

        // open sealedbox error: invalid sealedbox
        assert_eq!(
            SumMessageBuffer::new(&vec![0_u8; 256])
                .unwrap()
                .open_sealedbox(&coord.encr_pk, &coord.encr_sk)
                .unwrap_err(),
            PetError::InvalidMessage
        );

        // open box
        let msg = buf.open_box(&part.encr_pk, &coord.encr_sk).unwrap();
        assert_eq!(msg.len(), 99);

        // open box error: invalid box
        assert_eq!(
            SumMessageBuffer::new(&vec![0_u8; 256])
                .unwrap()
                .open_box(&part.encr_pk, &coord.encr_sk)
                .unwrap_err(),
            PetError::InvalidMessage
        );
    }

    #[test]
    fn test_updatemessagebuffer() {
        let coord = Coordinator::new().unwrap();
        let part = Participant::new().unwrap();

        // new
        let msg = part
            .compose_update_message(
                &coord.encr_pk,
                &[(
                    box_::PublicKey::from_slice(randombytes(32).as_slice()).unwrap(),
                    box_::PublicKey::from_slice(randombytes(32).as_slice()).unwrap(),
                )]
                .iter()
                .cloned()
                .collect(),
            )
            .message;
        let buf = UpdateMessageBuffer::new(&msg, 1).unwrap();
        assert_eq!(buf.0, msg.as_slice());
        assert_eq!(buf.1, 141..435);

        // new error: invalid message length
        assert_eq!(
            UpdateMessageBuffer::new(&vec![0_u8; 0], 0).unwrap_err(),
            PetError::InvalidMessage
        );

        // open sealedbox
        let msg = buf.open_sealedbox(&coord.encr_pk, &coord.encr_sk).unwrap();
        assert_eq!(msg.len(), 69);

        // open sealedbox error: invalid sealedbox
        assert_eq!(
            UpdateMessageBuffer::new(&vec![0_u8; 323], 0)
                .unwrap()
                .open_sealedbox(&coord.encr_pk, &coord.encr_sk)
                .unwrap_err(),
            PetError::InvalidMessage
        );

        // open box
        let msg = buf.open_box(&part.encr_pk, &coord.encr_sk).unwrap();
        assert_eq!(msg.len(), 278);

        // open box error: invalid box
        assert_eq!(
            UpdateMessageBuffer::new(&vec![0_u8; 323], 0)
                .unwrap()
                .open_box(&part.encr_pk, &coord.encr_sk)
                .unwrap_err(),
            PetError::InvalidMessage
        );
    }

    #[test]
    fn test_sum2messagebuffer() {
        let coord = Coordinator::new().unwrap();
        let mut part = Participant::new().unwrap();
        part.compose_sum_message(&coord.encr_pk);

        // new
        let msg = part
            .compose_sum2_message(
                &coord.encr_pk,
                &[(
                    box_::PublicKey::from_slice(part.encr_pk.as_ref()).unwrap(),
                    [(
                        box_::PublicKey::from_slice(randombytes(32).as_slice()).unwrap(),
                        sealedbox::seal(randombytes(32).as_slice(), &part.ephm_pk),
                    )]
                    .iter()
                    .cloned()
                    .collect(),
                )]
                .iter()
                .cloned()
                .collect(),
            )
            .unwrap()
            .message;
        let buf = Sum2MessageBuffer::new(&msg).unwrap();
        assert_eq!(buf.0, msg.as_slice());

        // new error: invalid message length
        assert_eq!(
            Sum2MessageBuffer::new(&vec![0_u8; 0]).unwrap_err(),
            PetError::InvalidMessage
        );

        // open sealedbox
        let msg = buf.open_sealedbox(&coord.encr_pk, &coord.encr_sk).unwrap();
        assert_eq!(msg.len(), 69);

        // open sealedbox error: invalid sealedbox
        assert_eq!(
            Sum2MessageBuffer::new(&vec![0_u8; 257])
                .unwrap()
                .open_sealedbox(&coord.encr_pk, &coord.encr_sk)
                .unwrap_err(),
            PetError::InvalidMessage
        );

        // open box
        let msg = buf.open_box(&part.encr_pk, &coord.encr_sk).unwrap();
        assert_eq!(msg.len(), 100);

        // open box error: invalid box
        assert_eq!(
            Sum2MessageBuffer::new(&vec![0_u8; 257])
                .unwrap()
                .open_box(&part.encr_pk, &coord.encr_sk)
                .unwrap_err(),
            PetError::InvalidMessage
        );
    }

    #[test]
    fn test_sealedboxbuffer() {
        // new
        let encr_pk = box_::PublicKey::from_slice(randombytes(32).as_slice()).unwrap();
        let sign_pk = sign::PublicKey::from_slice(randombytes(32).as_slice()).unwrap();
        let msg = [b"round", encr_pk.as_ref(), sign_pk.as_ref()].concat();
        let buf = SealedBoxBuffer::new(&msg).unwrap();
        assert_eq!(buf.0, msg.as_slice());

        // new error: invalid message length
        assert_eq!(
            SealedBoxBuffer::new(&vec![0_u8; 0]).unwrap_err(),
            PetError::InvalidMessage
        );

        // new error: invalid message tag
        assert_eq!(
            SealedBoxBuffer::new(&vec![0_u8; 69]).unwrap_err(),
            PetError::InvalidMessage
        );

        // get part encr pk
        let pk = buf.get_part_encr_pk();
        assert_eq!(pk, encr_pk);

        // get part sign pk
        let pk = buf.get_part_sign_pk();
        assert_eq!(pk, sign_pk);
    }

    #[test]
    fn test_sumboxbuffer() {
        // new
        let sign_sum = sign::Signature::from_slice(randombytes(64).as_slice()).unwrap();
        let ephm_pk = box_::PublicKey::from_slice(randombytes(32).as_slice()).unwrap();
        let msg = [b"sum", sign_sum.as_ref(), ephm_pk.as_ref()].concat();
        let buf = SumBoxBuffer::new(&msg).unwrap();
        assert_eq!(buf.0, msg.as_slice());

        // new error: invalid message length
        assert_eq!(
            SumBoxBuffer::new(&vec![0_u8; 0]).unwrap_err(),
            PetError::InvalidMessage
        );

        // new error: invalid message tag
        assert_eq!(
            SumBoxBuffer::new(&vec![0_u8; 99]).unwrap_err(),
            PetError::InvalidMessage
        );

        // get certificate
        let msg = buf.get_certificate();
        assert_eq!(msg, vec![0_u8; 0]);

        // get signature sum
        let msg = buf.get_signature_sum();
        assert_eq!(msg, sign_sum);

        // get part ephm pk
        let msg = buf.get_part_ephm_pk();
        assert_eq!(msg, ephm_pk);
    }

    #[test]
    fn test_updateboxbuffer() {
        // new
        let sign_sum = sign::Signature::from_slice(randombytes(64).as_slice()).unwrap();
        let sign_update = sign::Signature::from_slice(randombytes(64).as_slice()).unwrap();
        let model_url = randombytes(32);
        let dict_seed_key = box_::PublicKey::from_slice(randombytes(32).as_slice()).unwrap();
        let dict_seed_value = randombytes(80);
        let dict_seed = [(dict_seed_key, dict_seed_value.clone())]
            .iter()
            .cloned()
            .collect();
        let msg = [
            b"update",
            sign_sum.as_ref(),
            sign_update.as_ref(),
            model_url.as_slice(),
            dict_seed_key.as_ref(),
            dict_seed_value.as_slice(),
        ]
        .concat();
        let buf = UpdateBoxBuffer::new(&msg, 1).unwrap();
        assert_eq!(buf.0, msg.as_slice());

        // new error: invalid message length
        assert_eq!(
            UpdateBoxBuffer::new(&vec![0_u8; 0], 0).unwrap_err(),
            PetError::InvalidMessage
        );

        // new error: invalid message tag
        assert_eq!(
            UpdateBoxBuffer::new(&vec![0_u8; 278], 1).unwrap_err(),
            PetError::InvalidMessage
        );

        // get certificate
        let msg = buf.get_certificate();
        assert_eq!(msg, vec![0_u8; 0]);

        // get signature sum
        let msg = buf.get_signature_sum();
        assert_eq!(msg, sign_sum);

        // get signature update
        let msg = buf.get_signature_update();
        assert_eq!(msg, sign_update);

        // get model url
        let msg = buf.get_model_url();
        assert_eq!(msg, model_url);

        // get dict seed
        let msg = buf.get_dict_seed();
        assert_eq!(msg, dict_seed);
    }

    #[test]
    fn test_sum2boxbuffer() {
        // new
        let sign_sum = sign::Signature::from_slice(randombytes(64).as_slice()).unwrap();
        let mask_url = randombytes(32);
        let msg = [b"sum2", sign_sum.as_ref(), mask_url.as_slice()].concat();
        let buf = Sum2BoxBuffer::new(&msg).unwrap();
        assert_eq!(buf.0, msg.as_slice());

        // new error: invalid message length
        assert_eq!(
            Sum2BoxBuffer::new(&vec![0_u8; 0]).unwrap_err(),
            PetError::InvalidMessage
        );

        // new error: invalid message tag
        assert_eq!(
            Sum2BoxBuffer::new(&vec![0_u8; 100]).unwrap_err(),
            PetError::InvalidMessage
        );

        // get certificate
        let msg = buf.get_certificate();
        assert_eq!(msg, vec![0_u8; 0]);

        // get signature sum
        let msg = buf.get_signature_sum();
        assert_eq!(msg, sign_sum);

        // get mask url
        let msg = buf.get_mask_url();
        assert_eq!(msg, mask_url);
    }
}
