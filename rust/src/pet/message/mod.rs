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

#[derive(Debug)]
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
    /// Access the message buffer by reference.
    fn bytes(&self) -> &'b [u8] {
        self.bytes.as_ref()
    }

    /// Access the round box field of the message buffer by reference.
    fn round_box(&self) -> &'b [u8] {
        &self.bytes()[ROUNDBOX_RANGE]
    }

    /// Access the nonce field of the message buffer by reference.
    fn nonce(&self) -> &'b [u8] {
        &self.bytes()[NONCE_RANGE]
    }

    /// Access the message box field of the message buffer by reference.
    fn message_box(&self) -> &'b [u8] {
        &self.bytes()[MESSAGEBOX_START..]
    }
}

impl<B: AsMut<[u8]>> MessageBuffer<B> {
    /// Access the message buffer by mutable reference.
    fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_mut()
    }

    /// Access the round box field of the message buffer by mutable reference.
    fn round_box_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[ROUNDBOX_RANGE]
    }

    /// Access the nonce field of the message buffer by mutable reference.
    fn nonce_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[NONCE_RANGE]
    }

    /// Access the message box field of the message buffer by mutable reference.
    fn message_box_mut(&mut self) -> &mut [u8] {
        &mut self.bytes_mut()[MESSAGEBOX_START..]
    }
}

#[derive(Debug, PartialEq)]
/// Encryption and decryption of messages.
pub struct Message<E, S, M> {
    round_box: RoundBox<E, S>,
    message_box: M,
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
        // 250 / 314 + 112 * len(dict_seed) / 250 bytes for sum/update/sum2
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
        // 250 / 314 + 112 * len(dict_sum) / 250 bytes for sum/update/sum2
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

#[cfg(test)]
mod tests {
    use std::iter;

    use sodiumoxide::randombytes::{randombytes, randombytes_uniform};

    use super::*;

    #[test]
    fn test_box_tags() {
        assert_eq!(ROUND_TAG, 100);
        assert_eq!(SUM_TAG, 101);
        assert_eq!(UPDATE_TAG, 102);
        assert_eq!(SUM2_TAG, 103);
    }

    #[test]
    fn test_msgbox_field_ranges() {
        assert_eq!(TAG_RANGE.end - TAG_RANGE.start, 1);
        assert_eq!(CERTIFICATE_RANGE.end - CERTIFICATE_RANGE.start, 0);
        assert_eq!(SIGN_SUM_RANGE.end - SIGN_SUM_RANGE.start, 64);
    }

    #[test]
    fn test_msg_field_ranges() {
        assert_eq!(ROUNDBOX_RANGE.end - ROUNDBOX_RANGE.start, 113);
        assert_eq!(NONCE_RANGE.end - NONCE_RANGE.start, 24);
        assert_eq!(MESSAGEBOX_START, 137);
    }

    #[test]
    fn test_messagebuffer() {
        // new
        assert_eq!(MessageBuffer::new(10).bytes, vec![0_u8; 10]);

        // from
        let len = 153;
        let bytes = randombytes(len);
        let bytes_ = bytes.clone();
        let mut bytes_mut = bytes.clone();
        let mut bytes_mut_ = bytes.clone();
        assert_eq!(
            MessageBuffer::from(bytes.clone(), len).unwrap().bytes,
            bytes.clone(),
        );
        assert_eq!(
            MessageBuffer::from(&bytes, len).unwrap().bytes as *const Vec<u8>,
            &bytes as *const Vec<u8>,
        );
        assert_eq!(
            MessageBuffer::from(&mut bytes_mut, len).unwrap().bytes as *mut Vec<u8>,
            &mut bytes_mut as *mut Vec<u8>,
        );
        assert_eq!(
            MessageBuffer::from(&bytes, 10).unwrap_err(),
            PetError::InvalidMessage,
        );

        // bytes
        let buf = MessageBuffer::from(&bytes, len).unwrap();
        let mut buf_mut = MessageBuffer::from(&mut bytes_mut, len).unwrap();
        assert_eq!(buf.bytes(), &bytes_[..]);
        assert_eq!(buf_mut.bytes_mut(), &mut bytes_mut_[..]);

        // round box
        assert_eq!(buf.round_box(), &bytes_[0..113]);
        assert_eq!(buf_mut.round_box_mut(), &mut bytes_mut_[0..113]);

        // nonce
        assert_eq!(buf.nonce(), &bytes_[113..137]);
        assert_eq!(buf_mut.nonce_mut(), &mut bytes_mut_[113..137]);

        // message box
        assert_eq!(buf.message_box(), &bytes_[137..153]);
        assert_eq!(buf_mut.message_box_mut(), &mut bytes_mut_[137..153]);
    }

    #[test]
    fn test_summessage() {
        // new
        let encr_pk = &box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let sign_pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let rbox = RoundBox::new(encr_pk, sign_pk);
        let certificate = Vec::<u8>::new();
        let signature_sum = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let ephm_pk = &box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let sbox = SumBox::new(&certificate, signature_sum, ephm_pk);
        let msg = Message::new(rbox.clone(), sbox.clone());
        assert_eq!(msg.round_box, rbox);
        assert_eq!(msg.message_box, sbox);

        // len
        let len = 250;
        assert_eq!(msg.len(), len);
        assert_eq!(SumMessage::exp_len(None), len);

        // serialize
        let (pk, sk) = box_::gen_keypair();
        let rbox = rbox.seal(&pk);
        let (nonce, sbox) = sbox.seal(&pk, &sk);
        let msg = msg.serialize(rbox.clone(), nonce.clone(), sbox.clone());
        assert_eq!(
            msg,
            [rbox.as_slice(), nonce.as_ref(), sbox.as_slice()].concat(),
        );

        // deserialize
        let msg = SumMessage::deserialize(&msg, len).unwrap();
        assert_eq!(msg.0, rbox.as_slice());
        assert_eq!(msg.1, nonce);
        assert_eq!(msg.2, sbox.as_slice());
        assert_eq!(
            SumMessage::deserialize(&vec![0_u8; 10], len).unwrap_err(),
            PetError::InvalidMessage,
        );
        let msg = SumMessage {
            round_box: RoundBox::open(msg.0, &pk, &sk).unwrap(),
            message_box: SumBox::open(msg.2, &msg.1, &pk, &sk, SumBox::exp_len(None)).unwrap(),
        };

        // encr pk
        assert_eq!(msg.encr_pk(), encr_pk);

        // sign pk
        assert_eq!(msg.sign_pk(), sign_pk);

        // certificate
        assert_eq!(msg.certificate(), certificate.as_slice());

        // signature sum
        assert_eq!(msg.signature_sum(), signature_sum);

        // ephm pk
        assert_eq!(msg.ephm_pk(), ephm_pk);
    }

    #[test]
    fn test_updatemessage() {
        // new
        let encr_pk = &box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let sign_pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let rbox = RoundBox::new(encr_pk, sign_pk);
        let dict_sum_len = 1 + randombytes_uniform(10) as usize;
        let certificate = Vec::<u8>::new();
        let signature_sum = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let signature_update = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let model_url = randombytes(32);
        let dict_seed = &iter::repeat_with(|| {
            (
                box_::PublicKey::from_slice(&randombytes(32)).unwrap(),
                randombytes(80),
            )
        })
        .take(dict_sum_len)
        .collect();
        let ubox = UpdateBox::new(
            &certificate,
            signature_sum,
            signature_update,
            &model_url,
            dict_seed,
        );
        let msg = Message::new(rbox.clone(), ubox.clone());
        assert_eq!(msg.round_box, rbox);
        assert_eq!(msg.message_box, ubox);

        // len
        let len = 314 + 112 * dict_sum_len;
        assert_eq!(msg.len(), len);
        assert_eq!(UpdateMessage::exp_len(Some(dict_sum_len)), len);

        // serialize
        let (pk, sk) = box_::gen_keypair();
        let rbox = rbox.seal(&pk);
        let (nonce, ubox) = ubox.seal(&pk, &sk);
        let msg = msg.serialize(rbox.clone(), nonce.clone(), ubox.clone());
        assert_eq!(
            msg,
            [rbox.as_slice(), nonce.as_ref(), ubox.as_slice()].concat(),
        );

        // deserialize
        let msg = UpdateMessage::deserialize(&msg, len).unwrap();
        assert_eq!(msg.0, rbox.as_slice());
        assert_eq!(msg.1, nonce);
        assert_eq!(msg.2, ubox.as_slice());
        assert_eq!(
            UpdateMessage::deserialize(&vec![0_u8; 10], len).unwrap_err(),
            PetError::InvalidMessage,
        );
        let msg = UpdateMessage {
            round_box: RoundBox::open(msg.0, &pk, &sk).unwrap(),
            message_box: UpdateBox::open(
                msg.2,
                &msg.1,
                &pk,
                &sk,
                UpdateBox::exp_len(Some(dict_sum_len)),
            )
            .unwrap(),
        };

        // encr pk
        assert_eq!(msg.encr_pk(), encr_pk);

        // sign pk
        assert_eq!(msg.sign_pk(), sign_pk);

        // certificate
        assert_eq!(msg.certificate(), certificate.as_slice());

        // signature sum
        assert_eq!(msg.signature_sum(), signature_sum);

        // signature update
        assert_eq!(msg.signature_update(), signature_update);

        // model url
        assert_eq!(msg.model_url(), model_url.as_slice());

        // dict seed
        assert_eq!(msg.dict_seed(), dict_seed);
    }

    #[test]
    fn test_sum2message() {
        // new
        let encr_pk = &box_::PublicKey::from_slice(&randombytes(32)).unwrap();
        let sign_pk = &sign::PublicKey::from_slice(&randombytes(32)).unwrap();
        let rbox = RoundBox::new(encr_pk, sign_pk);
        let certificate = Vec::<u8>::new();
        let signature_sum = &sign::Signature::from_slice(&randombytes(64)).unwrap();
        let mask_url = randombytes(32);
        let sbox = Sum2Box::new(&certificate, signature_sum, &mask_url);
        let msg = Message::new(rbox.clone(), sbox.clone());
        assert_eq!(msg.round_box, rbox);
        assert_eq!(msg.message_box, sbox);

        // len
        let len = 250;
        assert_eq!(msg.len(), len);
        assert_eq!(Sum2Message::exp_len(None), len);

        // serialize
        let (pk, sk) = box_::gen_keypair();
        let rbox = rbox.seal(&pk);
        let (nonce, sbox) = sbox.seal(&pk, &sk);
        let msg = msg.serialize(rbox.clone(), nonce.clone(), sbox.clone());
        assert_eq!(
            msg,
            [rbox.as_slice(), nonce.as_ref(), sbox.as_slice()].concat(),
        );

        // deserialize
        let msg = Sum2Message::deserialize(&msg, len).unwrap();
        assert_eq!(msg.0, rbox.as_slice());
        assert_eq!(msg.1, nonce);
        assert_eq!(msg.2, sbox.as_slice());
        assert_eq!(
            Sum2Message::deserialize(&vec![0_u8; 10], len).unwrap_err(),
            PetError::InvalidMessage,
        );
        let msg = Sum2Message {
            round_box: RoundBox::open(msg.0, &pk, &sk).unwrap(),
            message_box: Sum2Box::open(msg.2, &msg.1, &pk, &sk, Sum2Box::exp_len(None)).unwrap(),
        };

        // encr pk
        assert_eq!(msg.encr_pk(), encr_pk);

        // sign pk
        assert_eq!(msg.sign_pk(), sign_pk);

        // certificate
        assert_eq!(msg.certificate(), certificate.as_slice());

        // signature sum
        assert_eq!(msg.signature_sum(), signature_sum);

        // mask url
        assert_eq!(msg.mask_url(), mask_url.as_slice());
    }
}
