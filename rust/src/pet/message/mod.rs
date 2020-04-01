#![allow(dead_code)] // temporary

pub mod round;
pub mod sum;
pub mod sum2;
pub mod update;

use std::ops::Range;

use sodiumoxide::crypto::{box_, sealedbox};

use self::round::RoundBox;
use crate::pet::PetError;

// box tags
const ROUND_TAG: u8 = 100;
const SUM_TAG: u8 = 101;
const UPDATE_TAG: u8 = 102;
const SUM2_TAG: u8 = 103;

// common message box field ranges
const TAG_RANGE: Range<usize> = 0..1;
const CERTIFICATE_RANGE: Range<usize> = 1..1;
const SIGN_SUM_RANGE: Range<usize> = 1..65;

// message field ranges
const ROUNDBOX_RANGE: Range<usize> = 0..117;
const NONCE_RANGE: Range<usize> = 117..141;
const MESSAGEBOX_START: usize = 141;

/// Immutable buffer access to common message box fields.
trait MessageBoxBufferRef<'a> {
    /// Access the message box buffer by reference.
    fn bytes(&self) -> &'a [u8];

    /// Access the tag field of the message box buffer by reference.
    fn tag(&self) -> &'a [u8] {
        &self.bytes()[TAG_RANGE]
    }

    /// Access the certificate field of the message box buffer by reference.
    fn certificate(&self) -> &'a [u8] {
        &self.bytes()[CERTIFICATE_RANGE]
    }

    /// Access the sum signature field of the message box buffer by reference.
    fn signature_sum(&self) -> &'a [u8] {
        &self.bytes()[SIGN_SUM_RANGE]
    }
}

/// Mutable buffer access to common message box fields.
trait MessageBoxBufferMut {
    /// Access the message box buffer by mutable reference.
    fn bytes_mut(&mut self) -> &mut [u8];

    /// Access the tag field of the message box buffer by mutable reference.
    fn tag_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[TAG_RANGE]
    }

    /// Access the certificate field of the message box buffer by mutable reference.
    fn certificate_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[CERTIFICATE_RANGE]
    }

    /// Access the sum signature field of the message box buffer by mutable reference.
    fn signature_sum_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[SIGN_SUM_RANGE]
    }
}

/// Encryption and decryption of message boxes.
pub trait MessageBox: Sized + Clone {
    /// Get the length of the serialized message box.
    fn len(&self) -> usize;

    /// Get the expected length of a serialized message box. Optional dependence on an external
    /// parameter.
    fn exp_len(param: Option<usize>) -> usize;

    /// Serialize the message box to bytes.
    fn serialize(&self) -> Vec<u8>;

    /// Deserialize a message box from bytes. Fails if the `bytes` don't conform to the expected
    /// message box length `exp_len`.
    fn deserialize(bytes: &[u8], exp_len: usize) -> Result<Self, PetError>;

    /// Encrypt the message box.
    fn seal(
        &self,
        coord_encr_pk: &box_::PublicKey,
        part_encr_sk: &box_::SecretKey,
    ) -> (box_::Nonce, Vec<u8>) {
        let bytes = self.serialize();
        let nonce = box_::gen_nonce();
        let sumbox = box_::seal(&bytes, &nonce, coord_encr_pk, part_encr_sk);
        (nonce, sumbox)
    }

    /// Decrypt a message box. Fails if the `bytes` don't conform to a valid encrypted message box.
    fn open(
        bytes: &[u8],
        nonce: &box_::Nonce,
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
        exp_len: usize,
    ) -> Result<Self, PetError> {
        let bytes = box_::open(bytes, nonce, coord_encr_pk, coord_encr_sk)
            .or(Err(PetError::InvalidMessage))?;
        Self::deserialize(&bytes, exp_len)
    }
}

/// Mutable and immutable buffer access to message fields.
struct MessageBuffer<T> {
    bytes: T,
}

impl MessageBuffer<Vec<u8>> {
    /// Create an empty message buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }
}

impl<T: AsRef<[u8]>> MessageBuffer<T> {
    /// Create a message buffer from `bytes`. Fails if the `bytes` don't conform to the expected
    /// message length `len`.
    fn from(bytes: T, len: usize) -> Result<Self, PetError> {
        (bytes.as_ref().len() == len)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> MessageBuffer<&'a T> {
    /// Access the round box field of the message buffer by reference.
    fn round_box(&self) -> &'a [u8] {
        &self.bytes.as_ref()[ROUNDBOX_RANGE]
    }

    /// Access the nonce field of the message buffer by reference.
    fn nonce(&self) -> &'a [u8] {
        &self.bytes.as_ref()[NONCE_RANGE]
    }

    /// Access the message box field of the message buffer by reference.
    fn message_box(&self) -> &'a [u8] {
        &self.bytes.as_ref()[MESSAGEBOX_START..]
    }
}

impl<T: AsMut<[u8]>> MessageBuffer<T> {
    /// Access the round box field of the message buffer by mutable reference.
    fn round_box_mut(&mut self) -> &mut [u8] {
        &mut self.bytes.as_mut()[ROUNDBOX_RANGE]
    }

    /// Access the nonce field of the message buffer by mutable reference.
    fn nonce_mut(&mut self) -> &mut [u8] {
        &mut self.bytes.as_mut()[NONCE_RANGE]
    }

    /// Access the message box field of the message buffer by mutable reference.
    fn message_box_mut(&mut self) -> &mut [u8] {
        &mut self.bytes.as_mut()[MESSAGEBOX_START..]
    }
}

/// Encryption and decryption of messages.
pub struct Message<T: MessageBox> {
    roundbox: RoundBox,
    messagebox: T,
}

impl<T: MessageBox> Message<T> {
    /// Create a message.
    pub fn new(roundbox: &RoundBox, messagebox: &T) -> Self {
        Self {
            roundbox: (*roundbox).clone(),
            messagebox: messagebox.clone(),
        }
    }

    /// Get the length of the serialized encrypted message.
    pub fn len(&self) -> usize {
        sealedbox::SEALBYTES
            + RoundBox::len()
            + box_::NONCEBYTES
            + box_::MACBYTES
            + self.messagebox.len()
    }

    /// Get the expected length of a serialized encrypted message. Optional dependence on an
    /// external parameter.
    pub fn exp_len(&self, param: Option<usize>) -> usize {
        sealedbox::SEALBYTES
            + RoundBox::exp_len()
            + box_::NONCEBYTES
            + box_::MACBYTES
            + T::exp_len(param)
    }

    /// Get the expected length of a message box from the expected length of a serialized encrypted
    /// message.
    fn msg_box_exp_len(exp_len: usize) -> usize {
        exp_len - sealedbox::SEALBYTES - RoundBox::exp_len() - box_::NONCEBYTES - box_::MACBYTES
    }

    /// Serialize the encrypted message to bytes.
    fn serialize(&self, roundbox: &[u8], nonce: &box_::Nonce, messagebox: &[u8]) -> Vec<u8> {
        let mut buffer = MessageBuffer::new(self.len());
        buffer.round_box_mut().copy_from_slice(roundbox);
        buffer.nonce_mut().copy_from_slice(nonce.as_ref());
        buffer.message_box_mut().copy_from_slice(messagebox);
        buffer.bytes
    }

    /// Deserialize an encrypted message from bytes. Fails if the `bytes` don't conform to the
    /// expected encrypted message length `exp_len`.
    fn deserialize(bytes: &[u8], exp_len: usize) -> Result<(&[u8], box_::Nonce, &[u8]), PetError> {
        let buffer = MessageBuffer::from(bytes, exp_len)?;
        let roundbox = buffer.round_box();
        let nonce = box_::Nonce::from_slice(buffer.nonce()).unwrap();
        let messagebox = buffer.message_box();
        Ok((roundbox, nonce, messagebox))
    }

    /// Encrypt the message.
    pub fn seal(&self, coord_encr_pk: &box_::PublicKey, part_encr_sk: &box_::SecretKey) -> Vec<u8> {
        let roundbox = self.roundbox.seal(coord_encr_pk);
        let (nonce, messagebox) = self.messagebox.seal(coord_encr_pk, part_encr_sk);
        self.serialize(&roundbox, &nonce, &messagebox)
    }

    /// Decrypt a message. Fails if the `bytes` don't conform to a valid encrypted message.
    pub fn open(
        bytes: &[u8],
        coord_encr_pk: &box_::PublicKey,
        coord_encr_sk: &box_::SecretKey,
        exp_len: usize,
    ) -> Result<Self, PetError> {
        let (roundbox, nonce, messagebox) = Self::deserialize(bytes, exp_len)?;
        let roundbox = RoundBox::open(roundbox, coord_encr_pk, coord_encr_sk)?;
        let messagebox = T::open(
            messagebox,
            &nonce,
            coord_encr_pk,
            coord_encr_sk,
            Self::msg_box_exp_len(exp_len),
        )?;
        Ok(Self {
            roundbox,
            messagebox,
        })
    }
}
