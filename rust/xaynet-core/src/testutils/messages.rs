//! This module provides helpers for generating messages or messages
//! parts such as signatures, cryptographic keys, or mask objects.

use std::convert::TryFrom;

use num::BigUint;

use crate::{
    crypto::{ByteObject, PublicEncryptKey, PublicSigningKey, Signature},
    mask::EncryptedMaskSeed,
    message::{Message, Payload, Sum, Sum2, Tag, Update},
    LocalSeedDict,
};

// A message adds 136 bytes of overhead:
//
// - a signature (64 bytes)
// - the participant pk (32 bytes)
// - the coordinator pk (32 bytes)
// - a length field (4 bytes)
// - a tag (1 byte)
// - flags (1 byte)
// - a reserved field (2 bytes)
pub const HEADER_LENGTH: usize = 136;

pub fn signature() -> (Signature, Vec<u8>) {
    let bytes = vec![0x1a; 64];
    let signature = Signature::from_slice(bytes.as_slice()).unwrap();
    (signature, bytes)
}

pub fn participant_pk() -> (PublicSigningKey, Vec<u8>) {
    let bytes = vec![0xbb; 32];
    let pk = PublicSigningKey::from_slice(&bytes).unwrap();
    (pk, bytes)
}

pub fn coordinator_pk() -> (PublicEncryptKey, Vec<u8>) {
    let bytes = vec![0xcc; 32];
    let pk = PublicEncryptKey::from_slice(&bytes).unwrap();
    (pk, bytes)
}

pub fn message<F, P>(f: F) -> (Message, Vec<u8>)
where
    F: Fn() -> (P, Vec<u8>),
    P: Into<Payload>,
{
    let (payload, payload_bytes) = f();
    let payload: Payload = payload.into();
    let tag = match payload {
        Payload::Sum(_) => Tag::Sum,
        Payload::Update(_) => Tag::Update,
        Payload::Sum2(_) => Tag::Sum2,
        _ => panic!("chunks not supported"),
    };
    let message = Message {
        signature: Some(signature().0),
        participant_pk: participant_pk().0,
        coordinator_pk: coordinator_pk().0,
        payload,
        is_multipart: false,
        tag,
    };

    let mut buf = signature().1;
    buf.extend(participant_pk().1);
    buf.extend(coordinator_pk().1);
    let length = payload_bytes.len() + HEADER_LENGTH;
    buf.extend(&(length as u32).to_be_bytes());
    buf.push(tag.into());
    buf.extend(vec![0, 0, 0]);
    buf.extend(payload_bytes);

    (message, buf)
}

pub mod sum {
    //! This module provides helpers for generating sum payloads

    use super::*;

    /// Return a fake sum task signature and its serialized version
    pub fn sum_task_signature() -> (Signature, Vec<u8>) {
        let bytes = vec![0x11; 64];
        let signature = Signature::from_slice(&bytes[..]).unwrap();
        (signature, bytes)
    }

    /// Return a fake ephemeral public key and its serialized version
    pub fn ephm_pk() -> (PublicEncryptKey, Vec<u8>) {
        let bytes = vec![0x22; 32];
        let pk = PublicEncryptKey::from_slice(&bytes[..]).unwrap();
        (pk, bytes)
    }

    /// Return an sum payload with its serialized version
    pub fn payload() -> (Sum, Vec<u8>) {
        let mut bytes = sum_task_signature().1;
        bytes.extend(ephm_pk().1);

        let sum = Sum {
            sum_signature: sum_task_signature().0,
            ephm_pk: ephm_pk().0,
        };
        (sum, bytes)
    }
}

pub mod update {
    //! This module provides helpers for generating update payloads
    pub use mask::{mask_object, mask_unit, mask_vect};
    pub use sum::sum_task_signature;

    use super::*;

    /// Return a fake update task signature and its serialized version
    pub fn update_task_signature() -> (Signature, Vec<u8>) {
        let bytes = vec![0x14; 64];
        let signature = Signature::from_slice(&bytes[..]).unwrap();
        (signature, bytes)
    }

    /// Return a local seed dictionary with two entries with its
    /// expected serialized version
    pub fn local_seed_dict() -> (LocalSeedDict, Vec<u8>) {
        let mut local_seed_dict = LocalSeedDict::new();
        let mut bytes = vec![];

        // Length (32+80) * 2 + 4 = 228
        bytes.extend(vec![0x00, 0x00, 0x00, 0xe4]);

        bytes.extend(vec![0x55; PublicSigningKey::LENGTH]);
        bytes.extend(vec![0x66; EncryptedMaskSeed::LENGTH]);
        local_seed_dict.insert(
            PublicSigningKey::from_slice(vec![0x55; 32].as_slice()).unwrap(),
            EncryptedMaskSeed::try_from(vec![0x66; EncryptedMaskSeed::LENGTH]).unwrap(),
        );

        // Second entry
        bytes.extend(vec![0x77; PublicSigningKey::LENGTH]);
        bytes.extend(vec![0x88; EncryptedMaskSeed::LENGTH]);
        local_seed_dict.insert(
            PublicSigningKey::from_slice(vec![0x77; 32].as_slice()).unwrap(),
            EncryptedMaskSeed::try_from(vec![0x88; EncryptedMaskSeed::LENGTH]).unwrap(),
        );

        (local_seed_dict, bytes)
    }

    /// Return an update payload with its serialized version
    pub fn payload() -> (Update, Vec<u8>) {
        let mut bytes = sum_task_signature().1;
        bytes.extend(update_task_signature().1);
        bytes.extend(mask_object().1);
        bytes.extend(local_seed_dict().1);

        let update = Update {
            sum_signature: sum_task_signature().0,
            update_signature: update_task_signature().0,
            masked_model: mask_object().0,
            local_seed_dict: local_seed_dict().0,
        };
        (update, bytes)
    }
}

pub mod sum2 {
    //! This module provides helpers for generating update payloads
    pub use mask::{mask_object, mask_unit, mask_vect};
    pub use sum::sum_task_signature;

    use super::*;

    /// Return a sum2 message and its serialized version
    pub fn payload() -> (Sum2, Vec<u8>) {
        let (sum_signature, sum_signature_bytes) = sum_task_signature();
        let (model_mask, model_mask_bytes) = mask_object();
        let bytes = [sum_signature_bytes.as_slice(), model_mask_bytes.as_slice()].concat();

        let sum2 = Sum2 {
            sum_signature,
            model_mask,
        };
        (sum2, bytes)
    }
}

pub mod mask {
    //! This module provides helpers for generating mask objects
    use crate::mask::{
        BoundType,
        DataType,
        GroupType,
        MaskConfig,
        MaskObject,
        MaskUnit,
        MaskVect,
        ModelType,
    };

    use super::*;

    /// Return a mask config and its serialized version
    pub fn mask_config() -> (MaskConfig, Vec<u8>) {
        // config.order() = 20_000_000_000_001 with this config, so the data
        // should be stored on 6 bytes.
        let config = MaskConfig {
            group_type: GroupType::Integer,
            data_type: DataType::I32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        };
        let bytes = vec![0x00, 0x02, 0x00, 0x03];
        (config, bytes)
    }

    /// Return a masked vector and its serialized version
    pub fn mask_vect() -> (MaskVect, Vec<u8>) {
        let (config, mut bytes) = mask_config();
        let data = vec![
            BigUint::from(1_u8),
            BigUint::from(2_u8),
            BigUint::from(3_u8),
            BigUint::from(4_u8),
        ];
        let mask_vect = MaskVect::new(config, data).unwrap();

        bytes.extend(vec![
            // number of elements
            0x00, 0x00, 0x00, 0x04, // data (1 weight => 6 bytes with this config)
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // 1
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, // 2
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, // 3
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, // 4
        ]);

        (mask_vect, bytes)
    }

    /// Return a masked scalar and its serialized version
    pub fn mask_unit() -> (MaskUnit, Vec<u8>) {
        let (config, mut bytes) = mask_config();
        let data = BigUint::from(1_u8);
        let mask_unit = MaskUnit::new(config, data).unwrap();

        bytes.extend(vec![
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // data: 1
        ]);
        (mask_unit, bytes)
    }

    /// Return a mask object, containing a masked vector and a masked
    /// scalar, and its serialized version
    pub fn mask_object() -> (MaskObject, Vec<u8>) {
        let (mask_vect, mask_vect_bytes) = mask_vect();
        let (mask_unit, mask_unit_bytes) = mask_unit();
        let obj = MaskObject::new_unchecked(mask_vect, mask_unit);
        let bytes = [mask_vect_bytes.as_slice(), mask_unit_bytes.as_slice()].concat();

        (obj, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // This tests is just so that if something changes, we catch it
    // and can update the helpers accordingly
    #[test]
    fn check_object_lengths() {
        assert_eq!(Signature::LENGTH, 64);
        assert_eq!(PublicEncryptKey::LENGTH, 32);
        assert_eq!(PublicSigningKey::LENGTH, 32);
        assert_eq!(EncryptedMaskSeed::LENGTH, 80);
    }
}
