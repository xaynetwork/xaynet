//! Message headers.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: ../index.html

use std::convert::TryFrom;

use anyhow::{anyhow, Context};

use crate::{
    message::{
        buffer::{MessageBuffer, HEADER_LENGTH},
        traits::{FromBytes, ToBytes},
        DecodeError,
    },
    ParticipantPublicKey,
};

#[derive(Copy, Debug, Clone, Eq, PartialEq)]
/// A tag that indicates the type of the [`Message`].
///
/// [`Message`]: struct.Message.html
pub enum Tag {
    /// A tag for [`Sum`] messages.
    ///
    /// [`Sum`]: struct.Sum.html
    Sum,
    /// A tag for [`Update`] messages.
    ///
    /// [`Update`]: struct.Update.html
    Update,
    /// A tag for [`Sum2`] messages.
    ///
    /// [`Sum2`]: struct.Sum2.html
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

pub type Flags = u8;

#[derive(Debug, Eq, PartialEq, Clone)]
/// A header common to all [`Message`]s.
///
/// [`Message`]: struct.Message.html
pub struct Header {
    /// The type of the message.
    pub tag: Tag,
    /// The participant public key.
    pub participant_pk: ParticipantPublicKey,
}

impl ToBytes for Header {
    fn buffer_length(&self) -> usize {
        HEADER_LENGTH
    }

    fn to_bytes<T: AsMut<[u8]>>(&self, buffer: &mut T) {
        let mut writer = MessageBuffer::new(buffer.as_mut()).unwrap();
        writer.set_tag(self.tag.into());
        writer.set_flags(0);
        self.participant_pk
            .to_bytes(&mut writer.participant_pk_mut());
    }
}

impl FromBytes for Header {
    fn from_bytes<T: AsRef<[u8]>>(buffer: &T) -> Result<Self, DecodeError> {
        let reader = MessageBuffer::new(buffer.as_ref())?;
        Ok(Self {
            tag: Tag::try_from(reader.tag())?,
            participant_pk: ParticipantPublicKey::from_bytes(&reader.participant_pk())
                .context("invalid participant public key")?,
        })
    }
}
