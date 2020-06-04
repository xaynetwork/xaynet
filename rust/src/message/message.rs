use anyhow::{anyhow, Context};
use std::borrow::Borrow;

use crate::{
    certificate::Certificate,
    crypto::{ByteObject, PublicEncryptKey, SecretEncryptKey, SecretSigningKey, Signature},
    mask::MaskObject,
    message::{
        DecodeError,
        FromBytes,
        Header,
        HeaderOwned,
        Payload,
        PayloadOwned,
        Sum2Owned,
        SumOwned,
        Tag,
        ToBytes,
        UpdateOwned,
    },
    LocalSeedDict,
};

/// A message
#[derive(Debug)]
pub struct Message<C, D, M, N> {
    /// Message header
    pub header: Header<C>,
    /// Message payload
    pub payload: Payload<D, M, N>,
}

pub type MessageOwned = Message<Certificate, LocalSeedDict, MaskObject, MaskObject>;

macro_rules! impl_new {
    ($name:ident, $payload:ty, $tag:expr) => {
        paste::item! {
            pub fn [<new_ $name>](
                coordinator_pk: $crate::CoordinatorPublicKey,
                participant_pk: $crate::ParticipantPublicKey,
                payload: $payload) -> Self
            {
                Self {
                    header: Header {
                        coordinator_pk,
                        participant_pk,
                        tag: $tag,
                        certificate: None,
                    },
                    payload: $crate::message::Payload::from(payload),
                }
            }
        }
    };
}

impl<C, D, M, N> Message<C, D, M, N>
where
    C: Borrow<Certificate>,
    D: Borrow<LocalSeedDict>,
    M: Borrow<MaskObject>,
    N: Borrow<MaskObject>,
{
    impl_new!(sum, crate::message::Sum, Tag::Sum);
    impl_new!(update, crate::message::Update<D, M>, Tag::Update);
    impl_new!(sum2, crate::message::Sum2<N>, Tag::Sum2);
}

impl<C, D, M, N> ToBytes for Message<C, D, M, N>
where
    C: Borrow<Certificate>,
    D: Borrow<LocalSeedDict>,
    M: Borrow<MaskObject>,
    N: Borrow<MaskObject>,
{
    fn buffer_length(&self) -> usize {
        self.header.buffer_length() + self.payload.buffer_length()
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        self.header.to_bytes(buffer);
        let mut payload_slice = &mut buffer.as_mut()[self.header.buffer_length()..];
        self.payload.to_bytes(&mut payload_slice);
    }
}

impl FromBytes for MessageOwned {
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let header = HeaderOwned::from_bytes(&buffer)?;
        let payload_slice = &buffer.as_ref()[header.buffer_length()..];
        let payload = match header.tag {
            Tag::Sum => PayloadOwned::Sum(
                SumOwned::from_bytes(&payload_slice).context("invalid sum payload")?,
            ),
            Tag::Update => PayloadOwned::Update(
                UpdateOwned::from_bytes(&payload_slice).context("invalid update payload")?,
            ),
            Tag::Sum2 => PayloadOwned::Sum2(
                Sum2Owned::from_bytes(&payload_slice).context("invalid sum2 payload")?,
            ),
        };
        Ok(Self { header, payload })
    }
}

/// A seal to sign and encrypt messages
pub struct MessageSeal<'a, 'b> {
    /// Public key of the recipient, used to encrypt messages
    pub recipient_pk: &'a PublicEncryptKey,
    /// Secret key of the sender, used to sign messages
    pub sender_sk: &'b SecretSigningKey,
}

impl<'a, 'b> MessageSeal<'a, 'b> {
    /// Sign and encrypt the given message
    pub fn seal<C, D, M, N>(&self, message: &Message<C, D, M, N>) -> Vec<u8>
    where
        C: Borrow<Certificate>,
        D: Borrow<LocalSeedDict>,
        M: Borrow<MaskObject>,
        N: Borrow<MaskObject>,
    {
        let signed_message = self.sign(&message);
        self.recipient_pk.encrypt(&signed_message[..])
    }

    /// Sign the given message
    fn sign<C, D, M, N>(&self, message: &Message<C, D, M, N>) -> Vec<u8>
    where
        C: Borrow<Certificate>,
        D: Borrow<LocalSeedDict>,
        M: Borrow<MaskObject>,
        N: Borrow<MaskObject>,
    {
        let signed_payload_length = message.buffer_length() + Signature::LENGTH;

        let mut buffer = vec![0; signed_payload_length];
        message.to_bytes(&mut &mut buffer[Signature::LENGTH..]);

        let signature = self.sender_sk.sign_detached(&buffer[Signature::LENGTH..]);
        signature.to_bytes(&mut &mut buffer[..Signature::LENGTH]);

        buffer
    }
}

/// A message opener that decrypts a message and verifies its signature
pub struct MessageOpen<'a, 'b> {
    /// Secret key for decrypting the message
    pub recipient_sk: &'b SecretEncryptKey,
    /// Public key for decrypting the message
    pub recipient_pk: &'a PublicEncryptKey,
}

impl<'a, 'b> MessageOpen<'a, 'b> {
    pub fn open<T: AsRef<[u8]>>(&self, buffer: &T) -> Result<MessageOwned, DecodeError> {
        // Step 1: decrypt the message
        let bytes = self
            .recipient_sk
            .decrypt(buffer.as_ref(), self.recipient_pk)
            .map_err(|_| anyhow!("invalid message: failed to decrypt message"))?;

        if bytes.len() < Signature::LENGTH {
            return Err(anyhow!("invalid message: invalid length"));
        }

        // UNWRAP_SAFE: the slice is exactly the size from_slice
        // expects.
        let signature = Signature::from_slice(&bytes[..Signature::LENGTH]).unwrap();

        let message_bytes = &bytes[Signature::LENGTH..];
        let message =
            MessageOwned::from_bytes(&message_bytes).context("invalid message: parsing failed")?;
        if !message
            .header
            .participant_pk
            .verify_detached(&signature, message_bytes)
        {
            return Err(anyhow!("invalid message: invalid signature"));
        }
        Ok(message)
    }
}
