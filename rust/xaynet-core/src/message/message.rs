//! Messages.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: ../index.html

use anyhow::{anyhow, Context};

use crate::{
    crypto::{
        encrypt::{PublicEncryptKey, SecretEncryptKey},
        sign::{SecretSigningKey, Signature},
        ByteObject,
    },
    message::{
        header::{Header, Tag},
        payload::{sum::Sum, sum2::Sum2, update::Update, Payload},
        traits::{FromBytes, ToBytes},
        DecodeError,
    },
};

#[derive(Debug, PartialEq, Eq, Clone)]
/// A message.
pub struct Message {
    /// The message header.
    pub header: Header,
    /// The message payload.
    pub payload: Payload,
}

macro_rules! impl_new {
    ($name:ident, $payload:ty, $tag:expr, $doc:expr) => {
        paste::item! {
            #[doc = "Creates a new message containing"]
            #[doc = $doc]
            pub fn [<new_ $name>](
                participant_pk: $crate::ParticipantPublicKey,
                payload: $payload) -> Self
            {
                Self {
                    header: Header {
                        participant_pk,
                        tag: $tag,
                    },
                    payload: $crate::message::payload::Payload::from(payload),
                }
            }
        }
    };
}

impl Message {
    impl_new!(sum, Sum, Tag::Sum, "a [`Sum`].");
    impl_new!(update, Update, Tag::Update, "an [`Update`].");
    impl_new!(sum2, Sum2, Tag::Sum2, "a [`Sum2`].");
}

impl ToBytes for Message {
    fn buffer_length(&self) -> usize {
        self.header.buffer_length() + self.payload.buffer_length()
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        self.header.to_bytes(buffer);
        let mut payload_slice = &mut buffer.as_mut()[self.header.buffer_length()..];
        self.payload.to_bytes(&mut payload_slice);
    }
}

impl FromBytes for Message {
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let header = Header::from_bytes(&buffer)?;
        let payload_slice = &buffer.as_ref()[header.buffer_length()..];
        let payload = match header.tag {
            Tag::Sum => {
                Payload::Sum(Sum::from_bytes(&payload_slice).context("invalid sum payload")?)
            }
            Tag::Update => Payload::Update(
                Update::from_bytes(&payload_slice).context("invalid update payload")?,
            ),
            Tag::Sum2 => {
                Payload::Sum2(Sum2::from_bytes(&payload_slice).context("invalid sum2 payload")?)
            }
        };
        Ok(Self { header, payload })
    }
}

/// A seal to sign and encrypt [`Message`]s.
pub struct MessageSeal<'a, 'b> {
    /// The public key of the recipient, which is used to encrypt the messages.
    pub recipient_pk: &'a PublicEncryptKey,
    /// The Secret key of the sender, which is used to sign the messages.
    pub sender_sk: &'b SecretSigningKey,
}

impl<'a, 'b> MessageSeal<'a, 'b> {
    /// Signs and encrypts the given message.
    pub fn seal(&self, message: &Message) -> Vec<u8> {
        let signed_message = self.sign(&message);
        self.recipient_pk.encrypt(&signed_message[..])
    }

    /// Signs the given message.
    fn sign(&self, message: &Message) -> Vec<u8> {
        let signed_payload_length = message.buffer_length() + Signature::LENGTH;

        let mut buffer = vec![0; signed_payload_length];
        message.to_bytes(&mut &mut buffer[Signature::LENGTH..]);

        let signature = self.sender_sk.sign_detached(&buffer[Signature::LENGTH..]);
        signature.to_bytes(&mut &mut buffer[..Signature::LENGTH]);

        buffer
    }
}

/// An opener to decrypt [`Message`]s and to verify their signatures.
pub struct MessageOpen<'a, 'b> {
    /// The secret key of the recipient, which is used to decrypt the message.
    pub recipient_sk: &'b SecretEncryptKey,
    /// The public key of the recipient, which is used to decrypt the message.
    pub recipient_pk: &'a PublicEncryptKey,
}

impl<'a, 'b> MessageOpen<'a, 'b> {
    /// Decrypts the given message and verifies its signature.
    pub fn open<T: AsRef<[u8]>>(&self, buffer: &T) -> Result<Message, DecodeError> {
        // Step 1: decrypt the message
        let bytes = self
            .recipient_sk
            .decrypt(buffer.as_ref(), self.recipient_pk)
            .map_err(|_| anyhow!("invalid message: failed to decrypt message"))?;

        if bytes.len() < Signature::LENGTH {
            return Err(anyhow!("invalid message: invalid length"));
        }

        // UNWRAP_SAFE: the slice is exactly the size from_slice expects.
        let signature = Signature::from_slice(&bytes[..Signature::LENGTH]).unwrap();

        let message_bytes = &bytes[Signature::LENGTH..];
        let message =
            Message::from_bytes(&message_bytes).context("invalid message: parsing failed")?;
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
