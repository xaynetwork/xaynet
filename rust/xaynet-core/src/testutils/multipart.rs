use num::BigUint;

use crate::{
    crypto::{ByteObject, PublicSigningKey, Signature},
    mask::{
        BoundType,
        DataType,
        EncryptedMaskSeed,
        GroupType,
        MaskConfig,
        MaskObject,
        MaskUnit,
        MaskVect,
        ModelType,
    },
    message::{Message, ToBytes, Update},
    testutils::messages,
    LocalSeedDict,
};

/// Return a seed dict that has the given length `len` once
/// serialized. `len - 4` must be multiple of 112.
pub fn local_seed_dict(len: usize) -> LocalSeedDict {
    // a public key is 32 bytes and an encrypted mask seed 80.
    let entry_len = 32 + 80;
    if ((len - 4) % entry_len) != 0 {
        panic!("invalid length for seed dict");
    }

    let nb_entries = (len - 4) / entry_len;
    let mut dict = LocalSeedDict::new();
    for i in 0..nb_entries {
        let bytes = (i as u64).to_be_bytes();
        let pk_bytes = bytes.iter().cycle().take(32).copied().collect::<Vec<_>>();
        let seed_bytes = bytes.iter().cycle().take(80).copied().collect::<Vec<_>>();
        let pk = PublicSigningKey::from_slice(pk_bytes.as_slice()).unwrap();
        let mask_seed = EncryptedMaskSeed::from_slice(seed_bytes.as_slice()).unwrap();
        dict.insert(pk, mask_seed);
    }

    // Check that our calculations are correct
    assert_eq!(dict.buffer_length(), len);
    dict
}

pub fn mask_object(len: usize) -> MaskObject {
    // The model contains 2 sub mask objects:
    //    - the masked model, which has:
    //         - 4 bytes for the config
    //         - 4 bytes for the number of weights
    //         - 6 bytes (with our config) for each weight
    //    - the masked scalar:
    //         - 4 bytes for the config
    //         - 6 bytes (with our config) for the scalar
    //
    // The only parameter we control to make the length vary is
    // the number of weights. The lengths is then:
    //
    // len = (4 + 4 + n_weights * 6) + (4 + 6) = 18 + 6 * n_weights
    //
    // So we must have: (len - 18) % 6 = 0
    if (len - 18) % 6 != 0 {
        panic!("invalid masked model length")
    }
    let n_weights = (len - 18) / 6;
    // Let's not be too crazy, it makes no sense to test with too
    // many weights
    assert!(n_weights < u32::MAX as usize);

    let mut weights = vec![];
    for i in 0..n_weights {
        weights.push(BigUint::from(i));
    }

    let masked_model = MaskVect::new(mask_config(), weights).unwrap();
    let masked_scalar = MaskUnit::new(mask_config(), BigUint::from(0_u32)).unwrap();
    let obj = MaskObject::new_unchecked(masked_model, masked_scalar);

    // Check that our calculations are correct
    assert_eq!(obj.buffer_length(), len);
    obj
}

pub fn mask_config() -> MaskConfig {
    // config.order() = 20_000_000_000_001 with this config, so the data
    // should be stored on 6 bytes.
    MaskConfig {
        group_type: GroupType::Integer,
        data_type: DataType::I32,
        bound_type: BoundType::B0,
        model_type: ModelType::M3,
    }
}

pub fn task_signatures() -> (Signature, Signature) {
    (
        messages::sum::sum_task_signature().0,
        messages::update::update_task_signature().0,
    )
}

/// Create an update payload with a seed dictionary of length
/// `dict_len` and a mask object of length `mask_len`. For a payload
/// of size `S`, the following must hold true:
///
/// ```no_rust
/// (mask_len - 22) % 6 = 0
/// (dict_len - 4) % 112 = 0
/// S = dict_len + mask_len + 64*2
/// ```
pub fn update(dict_len: usize, mask_obj_len: usize) -> Update {
    // An update message is made of:
    // - 2 signatures of 64 bytes each
    // - a mask object of variable length
    // - a seed dictionary of variable length
    //
    // The `Message` overhead is 136 bytes (see
    // crate::messages::HEADER_LEN). So a message with
    // `dict_len` = 100 and `mask_obj_len` = 100 will be:
    //
    //    100 + 100 + 64*2 + 136 = 464 bytes
    let (sum_signature, update_signature) = task_signatures();

    let payload = Update {
        sum_signature,
        update_signature,
        masked_model: mask_object(mask_obj_len),
        local_seed_dict: local_seed_dict(dict_len),
    };

    assert_eq!(payload.buffer_length(), mask_obj_len + dict_len + 64 * 2);
    payload
}

/// Create an update message with a seed dictionary of length
/// `dict_len` and a mask object of length `mask_len`. For a message
/// of size `S`, the following must hold true:
///
/// ```no_rust
/// (mask_len - 22) % 6 = 0
/// (dict_len - 4) % 112 = 0
/// S = dict_len + mask_len + 64*2 + 136
/// ```
pub fn message(dict_len: usize, mask_obj_len: usize) -> Message {
    let (message, _) = messages::message(|| {
        let payload = update(dict_len, mask_obj_len);
        let dummy_buf = vec![];
        (payload, dummy_buf)
    });
    message
}
