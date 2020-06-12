use anyhow::{anyhow, Context};
use std::{borrow::Borrow, convert::TryFrom};

use crate::{
    certificate::Certificate,
    message::{header_length, DecodeError, FromBytes, MessageBuffer, ToBytes},
    CoordinatorPublicKey,
    ParticipantPublicKey,
};

#[derive(Copy, Debug, Clone, Eq, PartialEq)]
/// Tag that indicates the type of message
pub enum Tag {
    /// Tag for sum messages
    Sum,
    /// Tag for update messages
    Update,
    /// Tag for sum2 messages
    Sum2,
}

impl TryFrom<u8> for Tag {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Tag::Sum,
            2 => Tag::Update,
            3 => Tag::Sum2,
            _ => return Err(anyhow!("invalid tag {}", value)),
        })
    }
}

impl Into<u8> for Tag {
    fn into(self) -> u8 {
        match self {
            Tag::Sum => 1,
            Tag::Update => 2,
            Tag::Sum2 => 3,
        }
    }
}

const CERTIFICATE_FLAG: u8 = 0;

bitflags::bitflags! {
    /// Bitmask that defines flags for a message
    pub struct Flags: u8 {
        /// Indicates the presence of a client certificate in the
        /// message
        const CERTIFICATE = 1 << CERTIFICATE_FLAG;
    }
}

/// A header common to all the messages
#[derive(Debug)]
pub struct Header<C> {
    /// Type of message
    pub tag: Tag,
    /// Coordinator public key
    pub coordinator_pk: CoordinatorPublicKey,
    /// Participant public key
    pub participant_pk: ParticipantPublicKey,
    /// A certificate that identifies the author of the message
    pub certificate: Option<C>,
}

impl<C> ToBytes for Header<C>
where
    C: Borrow<Certificate>,
{
    fn buffer_length(&self) -> usize {
        let cert_length = self
            .certificate
            .as_ref()
            .map(|cert| cert.borrow().buffer_length())
            .unwrap_or(0);
        header_length(cert_length)
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        let mut writer = MessageBuffer::new(buffer.as_mut()).unwrap();
        writer.set_tag(self.tag.into());
        if self.certificate.is_some() {
            writer.set_flags(Flags::CERTIFICATE);
        } else {
            writer.set_flags(Flags::empty());
        }
        self.coordinator_pk
            .to_bytes(&mut writer.coordinator_pk_mut());
        self.participant_pk
            .to_bytes(&mut writer.participant_pk_mut());
    }
}

/// Owned version of a [`Header`]
pub type HeaderOwned = Header<Certificate>;

impl FromBytes for HeaderOwned {
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MessageBuffer::new(buffer.as_ref())?;
        let certificate = if let Some(bytes) = reader.certificate() {
            Some(Certificate::from_bytes(&bytes.value())?)
        } else {
            None
        };
        Ok(Self {
            tag: Tag::try_from(reader.tag())?,
            coordinator_pk: CoordinatorPublicKey::from_bytes(&reader.coordinator_pk())
                .context("invalid coordinator public key")?,
            participant_pk: ParticipantPublicKey::from_bytes(&reader.participant_pk())
                .context("invalid participant public key")?,
            certificate,
        })
    }
}
