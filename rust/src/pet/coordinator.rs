use super::{utils::is_eligible, PetError};
use sodiumoxide::crypto::{box_, sealedbox, sign};

pub struct SumMessageBuffer(Vec<u8>);

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
            if msg.len() != 163 {
                return Err(PetError::InvalidMessage);
            }
            if &msg[128..131] != b"sum" {
                return Err(PetError::InvalidMessage);
            }
            return Ok(Self(msg));
        } else {
            return Err(PetError::InvalidMessage);
        }
    }

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

pub struct SumMessage {
    _sum_encr_pk: box_::PublicKey,
    _sum_ephm_pk: box_::PublicKey,
}

impl SumMessage {
    /// Decrypt and validate the message from a "sum" participant to get an item for the
    /// dictionary of "sum" participants.
    pub fn validate(
        message: Vec<u8>,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
        seed: &[u8],
        sum: f64,
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
            seed,
            sum,
        )?;
        let sum_ephm_pk = sumbox.get_part_ephm_pk()?;

        Ok(Self {
            _sum_encr_pk: sum_encr_pk,
            _sum_ephm_pk: sum_ephm_pk,
        })
    }

    fn validate_certificate(certificate: &[u8]) -> Result<(), PetError> {
        if certificate != b"" {
            return Err(PetError::InvalidMessage);
        }
        Ok(())
    }

    fn validate_signature(
        signature: &sign::Signature,
        part_sign_pk: &sign::PublicKey,
        seed: &[u8],
        sum: f64,
    ) -> Result<(), PetError> {
        if sign::verify_detached(signature, &[seed, &b"sum"[..]].concat(), part_sign_pk)
            && is_eligible(&signature.0[..], sum).ok_or(PetError::InvalidMessage)?
        {
            return Ok(());
        } else {
            return Err(PetError::InvalidMessage);
        }
    }
}
