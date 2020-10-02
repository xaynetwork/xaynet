#![allow(dead_code)]

use thiserror::Error;
use xaynet_core::{
    crypto::PublicEncryptKey,
    message::{Chunk, Message, Payload, Tag, ToBytes},
};

use super::Chunker;
use crate::mobile_client::participant::Participant;

/// An encoder for multipart messages. It implements
/// `Iterator<Item=Vec<u8>>`, which yields message parts ready to be
/// sent over the wire.
pub struct MultipartEncoder<'a, T> {
    /// Participant sending the message. Each chunk is signed with the
    /// participant secret key.
    participant: &'a Participant<T>,
    /// The coordinator public key. It should be the key used to
    /// encrypt the message.
    coordinator_pk: PublicEncryptKey,
    /// Serialized message payload.
    data: Vec<u8>,
    /// Next chunk ID to be produced by the iterator
    id: u16,
    /// Message tag
    tag: Tag,
    /// The maximum size allowed for the payload. `self.data` is split
    /// in chunks of this size.
    payload_size: usize,
}

/// Overhead induced by wrapping the data in [`Payload::Chunk`]
pub const CHUNK_OVERHEAD: usize = 8;
pub const MIN_PAYLOAD_SIZE: usize = CHUNK_OVERHEAD + 1;

impl<'a, T> Iterator for MultipartEncoder<'a, T> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let chunker = Chunker::new(&self.data, self.payload_size - CHUNK_OVERHEAD);

        if self.id as usize >= chunker.nb_chunks() {
            return None;
        }

        let chunk = Chunk {
            id: self.id,
            // FIXME: make this random
            message_id: 1,
            last: self.id as usize == chunker.nb_chunks() - 1,
            data: chunker.get_chunk(self.id as usize).to_vec(),
        };
        self.id += 1;

        let message = Message {
            // The signature is computed when serializing the message
            signature: None,
            participant_pk: self.participant.public_key(),
            is_multipart: true,
            tag: self.tag,
            payload: Payload::Chunk(chunk),
            coordinator_pk: self.coordinator_pk,
        };
        let data = self.participant.serialize_message(&message);
        Some(data)
    }
}

/// An encoder for a [`Payload`] representing a sum, update or sum2
/// message. If the [`Payload`] is small enough, a [`Message`] header
/// is added, and the message is serialized and signed. If
/// the [`Payload`] is too large to fit in a single message, it is
/// split in chunks which are also serialized and signed.
pub enum MessageEncoder<'a, T> {
    /// Encoder for a payload that fits in a single message.
    Simple(Option<Vec<u8>>),
    /// Encoder for a large payload that needs to be split in several
    /// parts.
    Multipart(MultipartEncoder<'a, T>),
}

impl<'a, T> Iterator for MessageEncoder<'a, T> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MessageEncoder::Simple(ref mut data) => data.take(),
            MessageEncoder::Multipart(ref mut multipart_encoder) => multipart_encoder.next(),
        }
    }
}

#[derive(Error, Debug)]
pub enum InvalidEncodingInput {
    #[error("only sum, update, and sum2 messages can be encoded")]
    Payload,
    #[error("the max payload size is too small")]
    PayloadSize,
}

impl<'a, T> MessageEncoder<'a, T> {
    // NOTE: the only reason we need to consume the payload is because creating the Message
    // consumes it.
    /// Create a new encoder for the given payload. The `participant`
    /// is used to sign the message(s). If the serialized payload is
    /// larger than `max_payload_size`, the message will we split in
    /// multiple chunks.
    ///
    /// # Errors
    ///
    /// An [`InvalidPayload`] error is returned when `payload` is of
    /// type [`Payload::Chunk`]. Only [`Payload::Sum`],
    /// [`Payload::Update`], [`Payload::Sum2`] are accepted.
    pub fn new(
        participant: &'a Participant<T>,
        payload: Payload,
        coordinator_pk: PublicEncryptKey,
        max_payload_size: usize,
    ) -> Result<Self, InvalidEncodingInput> {
        // Reject payloads of type Payload::Chunk. It is the job of the encoder to produce those if
        // the payload is deemed to big to be sent in a single message
        if payload.is_chunk() {
            return Err(InvalidEncodingInput::Payload);
        }

        if max_payload_size <= MIN_PAYLOAD_SIZE {
            return Err(InvalidEncodingInput::PayloadSize);
        }

        if payload.buffer_length() > max_payload_size {
            Ok(Self::new_multipart(
                participant,
                coordinator_pk,
                payload,
                max_payload_size,
            ))
        } else {
            Ok(Self::new_simple(participant, coordinator_pk, payload))
        }
    }

    fn new_simple(
        participant: &'a Participant<T>,
        coordinator_pk: PublicEncryptKey,
        payload: Payload,
    ) -> Self {
        let message = Message {
            // The signature is computed when serializing the message
            signature: None,
            participant_pk: participant.public_key(),
            is_multipart: false,
            coordinator_pk,
            tag: Self::get_tag_from_payload(&payload),
            payload,
        };
        let data = participant.serialize_message(&message);
        Self::Simple(Some(data))
    }

    fn new_multipart(
        participant: &'a Participant<T>,
        coordinator_pk: PublicEncryptKey,
        payload: Payload,
        payload_size: usize,
    ) -> Self {
        let tag = Self::get_tag_from_payload(&payload);
        let mut data = vec![0; payload.buffer_length()];
        payload.to_bytes(&mut data);
        Self::Multipart(MultipartEncoder {
            participant,
            data,
            id: 0,
            tag,
            coordinator_pk,
            payload_size,
        })
    }

    fn get_tag_from_payload(payload: &Payload) -> Tag {
        match payload {
            Payload::Sum(_) => Tag::Sum,
            Payload::Update(_) => Tag::Update,
            Payload::Sum2(_) => Tag::Sum2,
            Payload::Chunk(_) => panic!("no tag associated to Payload::Chunk"),
        }
    }
}
