#![allow(dead_code)] // temporary

pub mod round;
pub mod sum;
pub mod sum2;
pub mod update;

use std::{collections::HashMap, ops::Range};

use sodiumoxide::crypto::{box_, sealedbox, sign};

use self::{round::RoundBox, sum::SumBox, sum2::Sum2Box, update::UpdateBox};
use crate::pet::PetError;

// box tags
const ROUND_TAG: u8 = 100;
const SUM_TAG: u8 = 101;
const UPDATE_TAG: u8 = 102;
const SUM2_TAG: u8 = 103;

// common message box field ranges
const TAG_RANGE: Range<usize> = 0..1; // 1 byte
const CERTIFICATE_RANGE: Range<usize> = 1..1; // 0 bytes (dummy)
const SIGN_SUM_RANGE: Range<usize> = 1..65; // 64 bytes

// encrypted message field ranges
const ROUNDBOX_RANGE: Range<usize> = 0..113; // 113 bytes
const NONCE_RANGE: Range<usize> = 113..137; // 24 bytes
const MESSAGEBOX_START: usize = 137;

/// Immutable buffer access to common message box fields.
trait MsgBoxBufRef<'b> {
    /// Access the message box buffer by reference.
    fn bytes(&self) -> &'b [u8];

    /// Access the tag field of the message box buffer by reference.
    fn tag(&self) -> &'b [u8] {
        &self.bytes()[TAG_RANGE]
    }

    /// Access the certificate field of the message box buffer by reference.
    fn certificate(&self) -> &'b [u8] {
        &self.bytes()[CERTIFICATE_RANGE]
    }

    /// Access the sum signature field of the message box buffer by reference.
    fn signature_sum(&self) -> &'b [u8] {
        &self.bytes()[SIGN_SUM_RANGE]
    }
}

/// Mutable buffer access to common message box fields.
trait MsgBoxBufMut {
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

#[allow(clippy::len_without_is_empty)]
/// Encryption of message boxes.
pub trait MsgBoxEncr {
    /// Get the length of the serialized message box.
    fn len(&self) -> usize;

    /// Serialize the message box to bytes.
    fn serialize(&self) -> Vec<u8>;

    /// Encrypt the message box.
    fn seal(&self, pk: &box_::PublicKey, sk: &box_::SecretKey) -> (box_::Nonce, Vec<u8>) {
        let bytes = self.serialize();
        let nonce = box_::gen_nonce();
        let sumbox = box_::seal(&bytes, &nonce, pk, sk);
        (nonce, sumbox)
    }
}

/// Decryption of message boxes.
pub trait MsgBoxDecr: Sized {
    /// Get the expected length of a serialized message box. Optional dependence on an external
    /// parameter.
    fn exp_len(param: Option<usize>) -> usize;

    /// Deserialize a message box from bytes. Fails if the `bytes` don't conform to the expected
    /// message box length `exp_len`.
    fn deserialize(bytes: &[u8], exp_len: usize) -> Result<Self, PetError>;

    /// Decrypt a message box. Fails if the `bytes` don't conform to a valid encrypted message box.
    fn open(
        bytes: &[u8],
        nonce: &box_::Nonce,
        pk: &box_::PublicKey,
        sk: &box_::SecretKey,
        exp_len: usize,
    ) -> Result<Self, PetError> {
        let bytes = box_::open(bytes, nonce, pk, sk).or(Err(PetError::InvalidMessage))?;
        Self::deserialize(&bytes, exp_len)
    }
}

/// Mutable and immutable buffer access to encrypted message fields.
struct MessageBuffer<B> {
    bytes: B,
}

impl MessageBuffer<Vec<u8>> {
    /// Create an empty message buffer of size `len`.
    fn new(len: usize) -> Self {
        Self {
            bytes: vec![0_u8; len],
        }
    }
}

impl<B: AsRef<[u8]>> MessageBuffer<B> {
    /// Create a message buffer from `bytes`. Fails if the `bytes` don't conform to the expected
    /// message length `len`.
    fn from(bytes: B, len: usize) -> Result<Self, PetError> {
        (bytes.as_ref().len() == len)
            .then_some(Self { bytes })
            .ok_or(PetError::InvalidMessage)
    }
}

impl<'b, B: AsRef<[u8]> + ?Sized> MessageBuffer<&'b B> {
    /// Access the round box field of the message buffer by reference.
    fn round_box(&self) -> &'b [u8] {
        &self.bytes.as_ref()[ROUNDBOX_RANGE]
    }

    /// Access the nonce field of the message buffer by reference.
    fn nonce(&self) -> &'b [u8] {
        &self.bytes.as_ref()[NONCE_RANGE]
    }

    /// Access the message box field of the message buffer by reference.
    fn message_box(&self) -> &'b [u8] {
        &self.bytes.as_ref()[MESSAGEBOX_START..]
    }
}

impl<B: AsMut<[u8]>> MessageBuffer<B> {
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
pub struct Message<E, S, M> {
    pub round_box: RoundBox<E, S>,
    pub message_box: M,
}

#[allow(clippy::len_without_is_empty)]
impl<'m, M: MsgBoxEncr> Message<&'m box_::PublicKey, &'m sign::PublicKey, M> {
    /// Create a message.
    pub fn new(
        round_box: RoundBox<&'m box_::PublicKey, &'m sign::PublicKey>,
        message_box: M,
    ) -> Self {
        Self {
            round_box,
            message_box,
        }
    }

    /// Get the length of the serialized encrypted message.
    pub fn len(&self) -> usize {
        // 113 / 177 + 112 * len(dict_seed) / 113 bytes for sum/update/sum2
        sealedbox::SEALBYTES
            + self.round_box.len()
            + box_::NONCEBYTES
            + box_::MACBYTES
            + self.message_box.len()
    }

    /// Serialize the encrypted message to bytes.
    fn serialize(&self, round_box: Vec<u8>, nonce: box_::Nonce, message_box: Vec<u8>) -> Vec<u8> {
        let mut buffer = MessageBuffer::new(self.len());
        buffer.round_box_mut().copy_from_slice(&round_box);
        buffer.nonce_mut().copy_from_slice(nonce.as_ref());
        buffer.message_box_mut().copy_from_slice(&message_box);
        buffer.bytes
    }

    /// Encrypt the message.
    pub fn seal(&self, pk: &box_::PublicKey, sk: &box_::SecretKey) -> Vec<u8> {
        let round_box = self.round_box.seal(pk);
        let (nonce, message_box) = self.message_box.seal(pk, sk);
        self.serialize(round_box, nonce, message_box)
    }
}

impl<M: MsgBoxDecr> Message<box_::PublicKey, sign::PublicKey, M> {
    /// Get the expected length of a serialized encrypted message. Optional dependence on an
    /// external parameter.
    pub fn exp_len(param: Option<usize>) -> usize {
        // 113 / 177 + 112 * len(dict_sum) / 113 bytes for sum/update/sum2
        sealedbox::SEALBYTES
            + RoundBox::exp_len()
            + box_::NONCEBYTES
            + box_::MACBYTES
            + M::exp_len(param)
    }

    /// Get the expected length of a message box from the expected length of a serialized encrypted
    /// message.
    fn msg_box_exp_len(exp_len: usize) -> usize {
        exp_len - sealedbox::SEALBYTES - RoundBox::exp_len() - box_::NONCEBYTES - box_::MACBYTES
    }

    /// Deserialize an encrypted message from bytes. Fails if the `bytes` don't conform to the
    /// expected encrypted message length `exp_len`.
    fn deserialize(bytes: &[u8], exp_len: usize) -> Result<(&[u8], box_::Nonce, &[u8]), PetError> {
        let buffer = MessageBuffer::from(bytes, exp_len)?;
        let round_box = buffer.round_box();
        let nonce = box_::Nonce::from_slice(buffer.nonce()).unwrap();
        let message_box = buffer.message_box();
        Ok((round_box, nonce, message_box))
    }

    /// Decrypt a message. Fails if the `bytes` don't conform to a valid encrypted message.
    pub fn open(
        bytes: &[u8],
        pk: &box_::PublicKey,
        sk: &box_::SecretKey,
        exp_len: usize,
    ) -> Result<Self, PetError> {
        let (round_box, nonce, message_box) = Self::deserialize(bytes, exp_len)?;
        let round_box = RoundBox::open(round_box, pk, sk)?;
        let message_box = M::open(message_box, &nonce, pk, sk, Self::msg_box_exp_len(exp_len))?;
        Ok(Self {
            round_box,
            message_box,
        })
    }

    /// Get a reference to the public encryption key.
    pub fn encr_pk(&self) -> &box_::PublicKey {
        self.round_box.encr_pk()
    }

    /// Get a reference to the public signature key.
    pub fn sign_pk(&self) -> &sign::PublicKey {
        self.round_box.sign_pk()
    }
}

pub type SumMessage =
    Message<box_::PublicKey, sign::PublicKey, SumBox<Vec<u8>, sign::Signature, box_::PublicKey>>;

impl SumMessage {
    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &[u8] {
        self.message_box.certificate()
    }

    /// Get a reference to the sum signature.
    pub fn signature_sum(&self) -> &sign::Signature {
        self.message_box.signature_sum()
    }

    /// Get a reference to the public ephemeral key.
    pub fn ephm_pk(&self) -> &box_::PublicKey {
        self.message_box.ephm_pk()
    }
}

pub type UpdateMessage = Message<
    box_::PublicKey,
    sign::PublicKey,
    UpdateBox<Vec<u8>, sign::Signature, Vec<u8>, HashMap<box_::PublicKey, Vec<u8>>>,
>;

impl UpdateMessage {
    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &[u8] {
        self.message_box.certificate()
    }

    /// Get a reference to the sum signature.
    pub fn signature_sum(&self) -> &sign::Signature {
        self.message_box.signature_sum()
    }

    /// Get a reference to the update signature.
    pub fn signature_update(&self) -> &sign::Signature {
        self.message_box.signature_update()
    }

    /// Get a reference to the model url.
    pub fn model_url(&self) -> &[u8] {
        self.message_box.model_url()
    }

    /// Get a reference to the seed dictionary.
    pub fn dict_seed(&self) -> &HashMap<box_::PublicKey, Vec<u8>> {
        self.message_box.dict_seed()
    }
}

pub type Sum2Message =
    Message<box_::PublicKey, sign::PublicKey, Sum2Box<Vec<u8>, sign::Signature, Vec<u8>>>;

impl Sum2Message {
    /// Get a reference to the certificate.
    pub fn certificate(&self) -> &[u8] {
        self.message_box.certificate()
    }

    /// Get a reference to the sum signature.
    pub fn signature_sum(&self) -> &sign::Signature {
        self.message_box.signature_sum()
    }

    /// Get a reference to the mask url.
    pub fn mask_url(&self) -> &[u8] {
        self.message_box.mask_url()
    }
}
