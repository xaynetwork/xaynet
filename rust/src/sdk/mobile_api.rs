// TODO:
//  - error handling
//  - panic handling
//  - safety checks
//  - documentation

use std::{
    convert::TryFrom,
    ffi::CStr,
    iter::Iterator,
    os::raw::{c_char, c_int, c_uchar, c_uint, c_void},
    ptr,
    slice,
};

use crate::{
    certificate::Certificate,
    client::mobile_client::{participant::ParticipantSettings, MobileClient},
    crypto::ByteObject,
    mask::{
        config::{BoundType, DataType, GroupType, MaskConfig, ModelType},
        model::{FromPrimitives, IntoPrimitives, Model},
    },
    ParticipantSecretKey,
};

#[no_mangle]
pub unsafe extern "C" fn new_client(
    url: *const c_char,
    secret_key: *const c_uchar,
    group_type: c_uchar,
    data_type: c_uchar,
    bound_type: c_uchar,
    model_type: c_uchar,
) -> *mut MobileClient {
    if url.is_null() || secret_key.is_null() {
        return ptr::null_mut() as *mut MobileClient;
    }
    if !(group_type == 0 || group_type == 1 || group_type == 2) {
        return ptr::null_mut() as *mut MobileClient;
    }
    if !(data_type == 0 || data_type == 1 || data_type == 2 || data_type == 3) {
        return ptr::null_mut() as *mut MobileClient;
    }
    if !(bound_type == 0
        || bound_type == 2
        || bound_type == 4
        || bound_type == 6
        || bound_type == 255)
    {
        return ptr::null_mut() as *mut MobileClient;
    }
    if !(model_type == 3 || model_type == 6 || model_type == 9 || model_type == 12) {
        return ptr::null_mut() as *mut MobileClient;
    }

    let url = if let Ok(url) = CStr::from_ptr(url).to_str() {
        url
    } else {
        return ptr::null_mut() as *mut MobileClient;
    };

    let secret_key = slice::from_raw_parts(secret_key, ParticipantSecretKey::LENGTH);
    let secret_key = ParticipantSecretKey::from_slice_unchecked(secret_key);

    let group_type = GroupType::try_from(group_type).unwrap();
    let data_type = DataType::try_from(data_type).unwrap();
    let bound_type = BoundType::try_from(bound_type).unwrap();
    let model_type = ModelType::try_from(model_type).unwrap();
    let mask_config = MaskConfig {
        group_type,
        data_type,
        bound_type,
        model_type,
    };

    let certificate = Certificate::new();

    let participant_settings = ParticipantSettings {
        secret_key,
        mask_config,
        certificate,
    };

    Box::into_raw(Box::new(MobileClient::new(url, participant_settings)))
}

#[no_mangle]
pub unsafe extern "C" fn drop_client(client: *mut MobileClient) {
    if !client.is_null() {
        Box::from_raw(client);
    }
}

// error codes:
// -1: null pointers
// 0: success
// 1: no model available
// 2: conversion failed
#[no_mangle]
pub unsafe extern "C" fn get_model(
    client: *mut MobileClient,
    dtype: c_uint,
    model: *mut c_void,
    len: c_uint,
) -> c_int {
    if client.is_null() || model.is_null() {
        return -1_i32 as c_int;
    }
    let client = &mut *client;

    let len = len as usize;
    if let Some(m) = client.get_global_model() {
        match dtype as u32 {
            1 => {
                // safety checks missing
                let model = slice::from_raw_parts_mut(model as *mut f32, len);
                for (i, p) in m.into_primitives().enumerate() {
                    if i >= len {
                        return 2_i32 as c_int;
                    }
                    if let Ok(p) = p {
                        model[i] = p;
                    } else {
                        return 2_i32 as c_int;
                    }
                }
            }
            2 => {
                let model = slice::from_raw_parts_mut(model as *mut f64, len);
                for (i, p) in m.into_primitives().enumerate() {
                    if i >= len {
                        return 2_i32 as c_int;
                    }
                    if let Ok(p) = p {
                        model[i] = p;
                    } else {
                        return 2_i32 as c_int;
                    }
                }
            }
            3 => {
                let model = slice::from_raw_parts_mut(model as *mut i32, len);
                for (i, p) in m.into_primitives().enumerate() {
                    if i >= len {
                        return 2_i32 as c_int;
                    }
                    if let Ok(p) = p {
                        model[i] = p;
                    } else {
                        return 2_i32 as c_int;
                    }
                }
            }
            4 => {
                let model = slice::from_raw_parts_mut(model as *mut i64, len);
                for (i, p) in m.into_primitives().enumerate() {
                    if i >= len {
                        return 2_i32 as c_int;
                    }
                    if let Ok(p) = p {
                        model[i] = p;
                    } else {
                        return 2_i32 as c_int;
                    }
                }
            }
            _ => return 2_i32 as c_int,
        }
        0_i32 as c_int
    } else {
        1_i32 as c_int
    }
}

// error codes:
// -1: null pointers
// 0: success
// 1: length mismatch
// 2: conversion failed
#[no_mangle]
pub unsafe extern "C" fn update_model(
    client: *mut MobileClient,
    dtype: c_uint,
    model: *const c_void,
    len: c_uint,
) -> c_int {
    if client.is_null() || model.is_null() {
        return -1_i32 as c_int;
    }
    let client = &mut *client;

    // length checks missing
    let len = len as usize;
    match dtype as u32 {
        1 => {
            // safety checks missing
            let model = slice::from_raw_parts(model as *const f32, len);
            if let Ok(m) = Model::from_primitives(model.iter().copied()) {
                client.set_local_model(m);
                0_i32 as c_int
            } else {
                2_i32 as c_int
            }
        }
        2 => {
            let model = slice::from_raw_parts(model as *const f64, len);
            if let Ok(m) = Model::from_primitives(model.iter().copied()) {
                client.set_local_model(m);
                0_i32 as c_int
            } else {
                2_i32 as c_int
            }
        }
        3 => {
            let model = slice::from_raw_parts(model as *const i32, len);
            if let Ok(m) = Model::from_primitives(model.iter().copied()) {
                client.set_local_model(m);
                0_i32 as c_int
            } else {
                2_i32 as c_int
            }
        }
        4 => {
            let model = slice::from_raw_parts(model as *const i64, len);
            if let Ok(m) = Model::from_primitives(model.iter().copied()) {
                client.set_local_model(m);
                0_i32 as c_int
            } else {
                2_i32 as c_int
            }
        }
        _ => 2_i32 as c_int,
    }
}

#[no_mangle]
pub unsafe extern "C" fn next(client: *mut MobileClient) {
    if !client.is_null() {
        let client = &mut *client;
        client.next();
    }
}

#[no_mangle]
pub unsafe extern "C" fn new_secret_key(secret_key: *mut c_uchar) {
    if !secret_key.is_null() {
        let secret_key = slice::from_raw_parts_mut(secret_key, ParticipantSecretKey::LENGTH);
        secret_key.copy_from_slice(MobileClient::create_participant_secret_key().as_slice());
    }
}
