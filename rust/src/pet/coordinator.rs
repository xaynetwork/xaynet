use super::PetError;
use sodiumoxide::crypto::{box_, sealedbox, sign};
use std::{collections::HashMap, convert::TryFrom};

/// # Validate the received "sum" message.
/// Decrypt and validate the message parts from a "sum" participant. Then update the
/// dictionary of "sum" participants.
///
/// ## Note
/// Corresponds to steps 5., 6. and 7. of the PET protocol.
///
/// ## Args
/// - `coord_encr_pk`: The public key for asymmetric encryption of the coordinator.
/// - `coord_encr_sk`: The private key for asymmetric encryption of the coordinator.
/// - `message`: An encrypted message from a participant.
/// - `dict_sum`: The dictionary of "sum" participants.
///
/// ## Returns
/// An `Ok(())` if validation succeeds.
///
/// ## Raises
/// - `Err(())`: If validation fails.
pub fn validate_sum_message(
    coord_encr_pk: &box_::PublicKey,
    coord_encr_sk: &box_::SecretKey,
    message: &[Vec<u8>; 5],
    dict_sum: &mut HashMap<box_::PublicKey, box_::PublicKey>,
) -> Result<(), ()> {
    // validate "round" message and get participant public keys
    let msg = sealedbox::open(&message[0], &coord_encr_pk, &coord_encr_sk)?;
    if msg[64..] != b"round"[..] {
        return Err(());
    }
    let sum_encr_pk = box_::PublicKey(<[u8; 32]>::try_from(&msg[..32]).map_err(|_| -> () { () })?);
    let _sum_sign_pk =
        sign::PublicKey(<[u8; 32]>::try_from(&msg[32..64]).map_err(|_| -> () { () })?);

    // compute shared symmetric key
    let key = box_::precompute(&sum_encr_pk, &coord_encr_sk);

    // validate "sum" message
    let msg = box_::open_precomputed(
        &message[1][24..],
        &box_::Nonce(<[u8; 24]>::try_from(&message[1][..24]).map_err(|_| -> () { () })?),
        &key,
    )?;
    if msg != b"sum" {
        return Err(());
    }

    // validate dummy certificate
    let msg = box_::open_precomputed(
        &message[2][24..],
        &box_::Nonce(<[u8; 24]>::try_from(&message[2][..24]).map_err(|_| -> () { () })?),
        &key,
    )?;
    if msg != b"" {
        return Err(());
    }

    // get participant ephemeral public key
    let msg = box_::open_precomputed(
        &message[3][24..],
        &box_::Nonce(<[u8; 24]>::try_from(&message[3][..24]).map_err(|_| -> () { () })?),
        &key,
    )?;
    let sum_ephm_pk = box_::PublicKey(<[u8; 32]>::try_from(&msg[..]).map_err(|_| -> () { () })?);

    // validate dummy "sum" signature
    let msg = box_::open_precomputed(
        &message[4][24..],
        &box_::Nonce(<[u8; 24]>::try_from(&message[4][..24]).map_err(|_| -> () { () })?),
        &key,
    )?;
    if msg != b"" {
        return Err(());
    }

    // update dictionary of "sum" participants
    dict_sum.insert(sum_encr_pk, sum_ephm_pk);
    Ok(())
}

pub struct SumMessageBuffer(Vec<u8>);

impl SumMessageBuffer {
    pub fn new(message: Vec<u8>) -> Result<Self, PetError> {
        if message.len() != 192 {
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
        &self.0[141..192]
    }
}

pub struct SealedBoxBuffer(Vec<u8>);

impl SealedBoxBuffer {
    pub fn new(message: Result<Vec<u8>, ()>) -> Result<Self, PetError> {
        if let Ok(msg) = message {
            if msg.len() != 69 {
                return Err(PetError::InvalidMessage);
            }
            if &msg[64..69] != b"round" {
                return Err(PetError::InvalidMessage);
            }
            return Ok(Self(msg));
        } else {
            return Err(PetError::InvalidMessage);
        }
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
    pub fn new(message: Result<Vec<u8>, ()>) -> Result<Self, PetError> {
        if let Ok(msg) = message {
            if msg.len() != 35 {
                return Err(PetError::InvalidMessage);
            }
            if &msg[0..3] != b"sum" {
                return Err(PetError::InvalidMessage);
            }
            return Ok(Self(msg));
        } else {
            return Err(PetError::InvalidMessage);
        }
    }

    pub fn get_certificate(&self) -> &[u8] {
        &self.0[3..3]
    }

    pub fn get_part_ephm_pk(&self) -> Result<box_::PublicKey, PetError> {
        box_::PublicKey::from_slice(&self.0[3..35]).ok_or(PetError::InvalidMessage)
    }

    pub fn get_signature(&self) -> (&[u8], &[u8]) {
        (&self.0[35..35], &self.0[35..35])
    }
}

pub struct SumMessage {
    sum_encr_pk: box_::PublicKey,
    sum_ephm_pk: box_::PublicKey,
}

impl SumMessage {
    /// Decrypt and validate the message from a "sum" participant.
    pub fn validate(
        message: Vec<u8>,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
    ) -> Result<Self, PetError> {
        let msg = SumMessageBuffer::new(message)?;

        // get public keys
        let sealedbox = SealedBoxBuffer::new(sealedbox::open(
            msg.get_sealedbox(),
            coord_encr_pk,
            coord_encr_sk,
        ))?;
        let sum_encr_pk = sealedbox.get_part_encr_pk()?;
        let sum_sign_pk = sealedbox.get_part_sign_pk()?;

        // get ephemeral key
        let nonce = msg.get_nonce()?;
        let sumbox = SumBoxBuffer::new(box_::open(
            msg.get_box(),
            &nonce,
            &sum_encr_pk,
            coord_encr_sk,
        ))?;
        Self::validate_certificate(sumbox.get_certificate())?;
        Self::validate_signature(sumbox.get_signature(), &sum_sign_pk)?;
        let sum_ephm_pk = sumbox.get_part_ephm_pk()?;

        Ok(Self {
            sum_encr_pk,
            sum_ephm_pk,
        })
    }

    fn validate_certificate(certificate: &[u8]) -> Result<(), PetError> {
        if certificate != b"" {
            return Err(PetError::InvalidMessage);
        }
        Ok(())
    }

    fn validate_signature(
        signature: (&[u8], &[u8]),
        part_sign_pk: &sign::PublicKey,
    ) -> Result<(), PetError> {
        if signature.0 != b"" {
            return Err(PetError::InvalidMessage);
        }
        if signature.1 != b"" {
            return Err(PetError::InvalidMessage);
        }
        Ok(())
    }
}
