//! Messages.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: ../index.html

use std::borrow::Borrow;

use anyhow::{anyhow, Context};

use crate::{
    certificate::Certificate,
    crypto::{
        encrypt::{PublicEncryptKey, SecretEncryptKey},
        sign::{SecretSigningKey, Signature},
        ByteObject,
    },
    mask::object::MaskObject,
    message::{
        header::{Header, HeaderOwned, Tag},
        payload::{
            sum::{Sum, SumOwned},
            sum2::{Sum2, Sum2Owned},
            update::{Update, UpdateOwned},
            Payload,
            PayloadOwned,
        },
        traits::{FromBytes, ToBytes},
        DecodeError,
    },
    LocalSeedDict,
};

#[derive(Debug, PartialEq, Eq)]
/// A message.
pub struct Message<C, D, M, N> {
    /// The message header.
    pub header: Header<C>,
    /// The message payload.
    pub payload: Payload<D, M, N>,
}

/// An owned version of a [`Message`].
pub type MessageOwned = Message<Certificate, LocalSeedDict, MaskObject, MaskObject>;

macro_rules! impl_new {
    ($name:ident, $payload:ty, $tag:expr, $doc:expr) => {
        paste::item! {
            #[doc = "Creates a new message containing"]
            #[doc = $doc]
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
                    payload: $crate::message::payload::Payload::from(payload),
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
    impl_new!(sum, Sum, Tag::Sum, "a [`Sum`].");
    impl_new!(update, Update<D, M>, Tag::Update, "an [`Update`].");
    impl_new!(sum2, Sum2<N>, Tag::Sum2, "a [`Sum2`].");
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

/// A seal to sign and encrypt [`Message`]s.
pub struct MessageSeal<'a, 'b> {
    /// The public key of the recipient, which is used to encrypt the messages.
    pub recipient_pk: &'a PublicEncryptKey,
    /// The Secret key of the sender, which is used to sign the messages.
    pub sender_sk: &'b SecretSigningKey,
}

impl<'a, 'b> MessageSeal<'a, 'b> {
    /// Signs and encrypts the given message.
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

    /// Signs the given message.
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

/// An opener to decrypt [`Message`]s and to verify their signatures.
pub struct MessageOpen<'a, 'b> {
    /// The secret key of the recipient, which is used to decrypt the message.
    pub recipient_sk: &'b SecretEncryptKey,
    /// The public key of the recipient, which is used to decrypt the message.
    pub recipient_pk: &'a PublicEncryptKey,
}

impl<'a, 'b> MessageOpen<'a, 'b> {
    /// Decrypts the given message and verifies its signature.
    pub fn open<T: AsRef<[u8]>>(&self, buffer: &T) -> Result<MessageOwned, DecodeError> {
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
