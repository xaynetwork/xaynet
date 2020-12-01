use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::Chunker;
use xaynet_core::{
    crypto::{PublicEncryptKey, SecretSigningKey, SigningKeyPair},
    message::{Chunk, Message, Payload, Tag, ToBytes},
};

/// An encoder for multipart messages. It implements
/// `Iterator<Item=Vec<u8>>`, which yields message parts ready to be
/// sent over the wire.
#[derive(Serialize, Deserialize, Debug)]
pub struct MultipartEncoder {
    keys: SigningKeyPair,
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
    /// A random ID common to all the message chunks.
    message_id: u16,
}

/// Overhead induced by wrapping the data in [`Payload::Chunk`]
pub const CHUNK_OVERHEAD: usize = 8;
pub const MIN_PAYLOAD_SIZE: usize = CHUNK_OVERHEAD + 1;

impl Iterator for MultipartEncoder {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let chunker = Chunker::new(&self.data, self.payload_size - CHUNK_OVERHEAD);

        if self.id as usize >= chunker.nb_chunks() {
            return None;
        }

        let chunk = Chunk {
            id: self.id,
            message_id: self.message_id,
            last: self.id as usize == chunker.nb_chunks() - 1,
            data: chunker.get_chunk(self.id as usize).to_vec(),
        };
        self.id += 1;

        let message = Message {
            // The signature is computed when serializing the message
            signature: None,
            participant_pk: self.keys.public,
            is_multipart: true,
            tag: self.tag,
            payload: Payload::Chunk(chunk),
            coordinator_pk: self.coordinator_pk,
        };
        let data = serialize_message(&message, &self.keys.secret);
        Some(data)
    }
}

/// An encoder for a [`Payload`] representing a sum, update or sum2
/// message. If the [`Payload`] is small enough, a [`Message`] header
/// is added, and the message is serialized and signed. If
/// the [`Payload`] is too large to fit in a single message, it is
/// split in chunks which are also serialized and signed.
#[derive(Serialize, Deserialize, Debug)]
pub enum MessageEncoder {
    /// Encoder for a payload that fits in a single message.
    Simple(Option<Vec<u8>>),
    /// Encoder for a large payload that needs to be split in several
    /// parts.
    Multipart(MultipartEncoder),
}

impl Iterator for MessageEncoder {
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

impl MessageEncoder {
    // NOTE: the only reason we need to consume the payload is because creating the Message
    // consumes it.
    /// Create a new encoder for the given payload. The `participant`
    /// is used to sign the message(s). If the serialized payload is
    /// larger than `max_payload_size`, the message will we split in
    /// multiple chunks. If `max_payload_size` is `0`, the message
    /// will not be split.
    ///
    /// # Errors
    ///
    /// An [`InvalidPayload`] error is returned when `payload` is of
    /// type [`Payload::Chunk`]. Only [`Payload::Sum`],
    /// [`Payload::Update`], [`Payload::Sum2`] are accepted.
    pub fn new(
        keys: SigningKeyPair,
        payload: Payload,
        coordinator_pk: PublicEncryptKey,
        max_payload_size: usize,
    ) -> Result<Self, InvalidEncodingInput> {
        // Reject payloads of type Payload::Chunk. It is the job of the encoder to produce those if
        // the payload is deemed to big to be sent in a single message
        if payload.is_chunk() {
            return Err(InvalidEncodingInput::Payload);
        }

        if max_payload_size != 0 && max_payload_size <= MIN_PAYLOAD_SIZE {
            return Err(InvalidEncodingInput::PayloadSize);
        }

        if max_payload_size != 0 && payload.buffer_length() > max_payload_size {
            Ok(Self::new_multipart(
                keys,
                coordinator_pk,
                payload,
                max_payload_size,
            ))
        } else {
            Ok(Self::new_simple(keys, coordinator_pk, payload))
        }
    }

    fn new_simple(
        keys: SigningKeyPair,
        coordinator_pk: PublicEncryptKey,
        payload: Payload,
    ) -> Self {
        let message = Message {
            // The signature is computed when serializing the message
            signature: None,
            participant_pk: keys.public,
            is_multipart: false,
            coordinator_pk,
            tag: Self::get_tag_from_payload(&payload),
            payload,
        };
        let data = serialize_message(&message, &keys.secret);
        Self::Simple(Some(data))
    }

    fn new_multipart(
        keys: SigningKeyPair,
        coordinator_pk: PublicEncryptKey,
        payload: Payload,
        payload_size: usize,
    ) -> Self {
        let tag = Self::get_tag_from_payload(&payload);
        let mut data = vec![0; payload.buffer_length()];
        payload.to_bytes(&mut data);
        Self::Multipart(MultipartEncoder {
            keys,
            data,
            id: 0,
            tag,
            coordinator_pk,
            payload_size,
            message_id: rand::random::<u16>(),
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

#[cfg(test)]
mod tests {
    use xaynet_core::{
        crypto::{ByteObject, EncryptKeyPair, EncryptKeySeed, SigningKeyPair, SigningKeySeed},
        message::{FromBytes, Update},
        testutils::multipart as helpers,
    };

    use super::*;

    fn participant_keys() -> SigningKeyPair {
        let seed = SigningKeySeed::from_slice(vec![0; 32].as_slice()).unwrap();
        SigningKeyPair::derive_from_seed(&seed)
    }

    fn coordinator_keys() -> EncryptKeyPair {
        let seed = EncryptKeySeed::from_slice(vec![0; 32].as_slice()).unwrap();
        EncryptKeyPair::derive_from_seed(&seed)
    }

    fn message(dict_len: usize, mask_obj_len: usize) -> Message {
        let payload = helpers::update(dict_len, mask_obj_len).into();
        Message {
            signature: None,
            participant_pk: participant_keys().public,
            is_multipart: false,
            tag: Tag::Update,
            payload,
            coordinator_pk: coordinator_keys().public,
        }
    }

    fn small_message() -> Message {
        let dict_len = 80 + 32 + 4; // 116 => dict with a single entry
        let model_len = 6 + 18; // 24 => masked model with single weight
        let message = message(dict_len, model_len);
        let payload_len = dict_len + model_len + 64 * 2; // 268
        let message_len = payload_len + 136; // 404
        assert_eq!(message.payload.buffer_length(), payload_len);
        assert_eq!(message.buffer_length(), message_len);
        message
    }

    #[test]
    fn no_chunk() {
        let msg = small_message();

        let mut enc = MessageEncoder::new(
            participant_keys(),
            msg.clone().payload,
            msg.coordinator_pk,
            272,
        )
        .unwrap();

        let data = enc.next().unwrap();
        let parsed = Message::from_byte_slice(&data.as_slice()).unwrap();
        assert_eq!(parsed.is_multipart, false);
        assert_eq!(parsed.payload, msg.payload);
        assert!(enc.next().is_none());
    }

    #[test]
    fn two_chunks() {
        let msg = small_message();

        let mut enc = MessageEncoder::new(
            participant_keys(),
            msg.clone().payload,
            msg.coordinator_pk,
            200,
        )
        .unwrap();

        let data = enc.next().unwrap();
        // The payload should be 200 bytes + 136 bytes for the
        // message header.
        //
        // 8 of these 200 payload bytes are for the Chunk payload
        // header. So this chunk actually only contains 192 bytes (out
        // of 268) from the Update payload. So 76 bytes remain.
        assert_eq!(data.len(), 200 + 136);
        let parsed = Message::from_byte_slice(&data.as_slice()).unwrap();
        assert_eq!(parsed.is_multipart, true);
        let chunk1 = extract_chunk(parsed);
        assert!(!chunk1.last);
        assert_eq!(chunk1.id, 0);
        assert_eq!(chunk1.data.len(), 192);

        let data = enc.next().unwrap();
        // The payload should be 76 bytes + 8 bytes of CHUNK_OVERHEAD,
        // plus 136 byte for the message header
        assert_eq!(data.len(), 84 + 136);
        let parsed = Message::from_byte_slice(&data.as_slice()).unwrap();
        assert_eq!(parsed.is_multipart, true);
        let chunk2 = extract_chunk(parsed);
        assert!(chunk2.last);
        assert_eq!(chunk2.id, 1);
        assert_eq!(chunk2.data.len(), 76);

        let payload_data: Vec<u8> = [chunk1.data, chunk2.data].concat();
        let update = Update::from_byte_slice(&payload_data).unwrap();
        assert_eq!(update, extract_update(msg));
    }

    fn extract_chunk(message: Message) -> Chunk {
        if let Payload::Chunk(c) = message.payload {
            c
        } else {
            panic!("not a chunk message");
        }
    }

    fn extract_update(message: Message) -> Update {
        if let Payload::Update(u) = message.payload {
            u
        } else {
            panic!("not an update message");
        }
    }
}

fn serialize_message(message: &Message, sk: &SecretSigningKey) -> Vec<u8> {
    let mut buf = vec![0; message.buffer_length()];
    message.to_bytes(&mut buf, sk);
    buf
}
