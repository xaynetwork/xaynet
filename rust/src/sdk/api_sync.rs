use std::{
    ffi::CStr,
    os::raw::{c_char, c_int, c_uint, c_void},
    panic,
    ptr,
    slice,
};

use crate::{
    client::SyncClient,
    mask::model::{FromPrimitives, IntoPrimitives, Model},
};

#[no_mangle]
pub unsafe extern "C" fn new_client(address: *const c_char) -> *mut SyncClient {
    if address.is_null() {
        return ptr::null_mut() as *mut SyncClient;
    }
    let address = if let Ok(address) = unsafe {
        // safe if the raw pointer `address` comes from a null-terminated C-string
        CStr::from_ptr(address)
    }
    .to_str()
    {
        address
    } else {
        return ptr::null_mut() as *mut SyncClient;
    };
    Box::into_raw(Box::new(SyncClient::new(address)))
}

#[no_mangle]
pub unsafe extern "C" fn start_client(client: *mut SyncClient) {
    if !client.is_null() {
        let client = &mut *client;
        if let Err(_) = panic::catch_unwind(panic::AssertUnwindSafe(|| client.start())) {
            error!("client panicked");
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn stop_client(client: *mut SyncClient) {
    if !client.is_null() {
        let client = &mut *client;
        if let Err(_) = panic::catch_unwind(panic::AssertUnwindSafe(|| client.stop())) {
            error!("client panicked");
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn drop_client(client: *mut SyncClient) {
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
    client: *mut SyncClient,
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
        return 0_i32 as c_int;
    } else {
        return 1_i32 as c_int;
    }
}

// error codes:
// -1: null pointers
// 0: success
// 1: length mismatch
// 2: conversion failed
#[no_mangle]
pub unsafe extern "C" fn update_model(
    client: *mut SyncClient,
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
                return 0_i32 as c_int;
            } else {
                return 2_i32 as c_int;
            }
        }
        2 => {
            let model = slice::from_raw_parts(model as *const f64, len);
            if let Ok(m) = Model::from_primitives(model.iter().copied()) {
                client.set_local_model(m);
                return 0_i32 as c_int;
            } else {
                return 2_i32 as c_int;
            }
        }
        3 => {
            let model = slice::from_raw_parts(model as *const i32, len);
            if let Ok(m) = Model::from_primitives(model.iter().copied()) {
                client.set_local_model(m);
                return 0_i32 as c_int;
            } else {
                return 2_i32 as c_int;
            }
        }
        4 => {
            let model = slice::from_raw_parts(model as *const i64, len);
            if let Ok(m) = Model::from_primitives(model.iter().copied()) {
                client.set_local_model(m);
                return 0_i32 as c_int;
            } else {
                return 2_i32 as c_int;
            }
        }
        _ => {
            return 2_i32 as c_int;
        }
    }
}
