#![cfg_attr(doc, forbid(warnings))]
//! A C-API of the Xaynet mobile client.
//!
//! # Safety
//!
//! Many functions of this module are marked as `unsafe` to explicitly announce the possible
//! unsafety of the function body as well as the return value to the caller. At the same time,
//! each `unsafe fn` uses `unsafe` blocks to precisely pinpoint the sources of unsafety for
//! reviewers (redundancy warnings will be fixed by [#71668]).
//!
//! Most of the functions have one / or more pointers as parameters.
//! All functions ensure null-safety for these pointers.
//! However, when calling this function, you need to ensure that a pointer:
//! - is properly aligned,
//! - points to an initialized instance of T where T is the type of data.
//! The behavior of the function is undefined if the requirements are not met.
//!
//! All function of the API are **not** thread-safe.
//!
//! # Error handling
//!
//! In terms of Error handling, the C-API tries to follow the Posix-style.
//! Functions return `0` to indicate success and negative values ​​to indicate failure.
//! Functions that return an opaque pointer (like [`xaynet_ffi_init_mobile_client`])
//! return a non null pointer to indicate success and a null pointer to indicate failure.
//!
//! [#71668]: https://github.com/rust-lang/rust/issues/71668

#[macro_use]
extern crate ffi_support;

#[cfg(feature = "tls")]
use std::path::PathBuf;
use std::{
    convert::TryFrom,
    iter::Iterator,
    os::raw::{c_double, c_int, c_uchar, c_uint, c_void},
    ptr,
    slice,
};

use ffi_support::FfiStr;

use xaynet_client::mobile_client::{
    participant::{AggregationConfig, MaxMessageSize, ParticipantSettings},
    MobileClient,
};
use xaynet_core::{
    crypto::ByteObject,
    mask::{
        BoundType,
        DataType,
        FromPrimitives,
        GroupType,
        IntoPrimitives,
        MaskConfig,
        Model,
        ModelType,
    },
    ParticipantSecretKey,
};

#[cfg(feature = "tls")]
#[allow(unused_unsafe)]
/// Converts raw certificate path strings to rust paths.
///
/// Interprets null pointers, length zero and invalid UTF-8 characters as `None`.
unsafe fn certificate_paths_from(raw: *const FfiStr, len: c_uint) -> Option<Vec<PathBuf>> {
    // a return value like `Result<Option<Vec<PathBuf>>, ()> would be desirable, but the
    // ffi-support crate doesn't differentiate between null pointers and UTF-8 errors
    // and turns both into None, which makes it impossible to tell them apart later on

    let len = if len > 0 {
        len as usize
    } else {
        // ignore `certificates` if `len` is zero
        return None;
    };

    unsafe { raw.as_ref() }
        .map(|certificates| {
            // convert raw array to slice and try to read the raw strings from each slice element
            unsafe { slice::from_raw_parts(certificates, len) }
                .iter()
                .map(|certificate| unsafe { certificate.as_opt_str() }.map(PathBuf::from))
                .collect::<Option<Vec<_>>>()
        })
        .flatten()
}

#[cfg(feature = "tls")]
#[allow(unused_unsafe)]
/// Converts a raw identity path string to a rust path.
///
/// Interprets a null pointer and invalid UTF-8 characters as `None`.
unsafe fn identity_path_from(raw: FfiStr) -> Option<PathBuf> {
    // a return value like `Result<Option<PathBuf>, ()> would be desirable, but the
    // ffi-support crate doesn't differentiate between null pointers and UTF-8 errors
    // and turns both into None, which makes it impossible to tell them apart later on

    unsafe { raw.as_opt_str() }.map(PathBuf::from)
}

/// An opaque type of MobileClient.
/// see [FFI-C-OPAQUE](https://anssi-fr.github.io/rust-guide/07_ffi.html#recommendation-a-idffi-c-opaqueaffi-c-opaque)
pub struct CMobileClient(MobileClient);

/// Initializes a fresh [`CMobileClient`]. This method only needs to be called once.
///
/// To serialize and restore a client use the
/// [`xaynet_ffi_serialize_mobile_client`] and [`xaynet_ffi_restore_mobile_client`].
///
/// # Parameters
///
/// - `url`: The URL fo the coordinator to which the [`MobileClient`] will try to connect to.
/// - `secret_key`: The array that contains the secret key.
/// - `group_type`: The [`GroupType`].
/// - `data_type`: The [`DataType`].
/// - `bound_type`: The [`BoundType`].
/// - `model_type`: The [`ModelType`].
/// - `scalar`: The scalar.
/// - `certificates`: The optional array of paths to DER/PEM encoded trusted server certificates for
///   TLS server authentication. Requires the `tls` feature to be enabled. Interprets null pointers
///   as `None` and is ignored for `certificates_len` of zero.
/// - `certificates_len`: The number of DER/PEM encoded optional certificates. Requires the `tls`
///   feature to be enabled. Interprets zero as `None`.
/// - `identity`: The optional path to a PEM encoded client certificate for TLS client
///   authentication. Requires the `tls` feature to be enabled. Interprets a null pointer as `None`.
///
/// Requires at least one of the following arguments if the `tls` feature is enabled:
/// - `certificates` together with `certificates_len`
/// - `identity`
///
/// # Safety
///
/// `secret_key`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of `c_uchar`,
/// - the data the pointer points to is properly aligned,
/// - the data is valid for reads for [`ParticipantSecretKey::LENGTH`]` * mem::size_of::<c_uchar>()`
/// many bytes,
/// - the memory of secret_key is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_init_mobile_client`].
///
/// `certificates`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of `*const FfiStr`
/// - the data the pointers point to are properly aligned and valid UTF-8,
/// - the data is valid for reads for `certificates_len * mem::size_of::<*const _>()` many bytes,
/// - the memory of the certificates is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_init_mobile_client`].
///
/// `identity`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of `c_char`
/// - the data the pointer points to is properly aligned and valid UTF-8,
/// - the memory of the identity is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_init_mobile_client`].
///
/// # Return Value
///
/// Returns a new instance of [`CMobileClient`].
///
/// ## Returns `NULL` if:
///
/// - a value of `group_type`, `data_type`, `bound_type` or `model_type` is not a valid value
/// (see the module documentation of [`xaynet_core::mask`] for more information),
/// - the pointer of `url` or `secret_key` points to `NULL`,
/// - the `url` contains invalid UTF-8 characters.
/// - the TLS settings are invalid
///
/// [`MobileClient`]: xaynet_client::mobile_client::MobileClient
#[allow(unused_unsafe)]
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_init_mobile_client(
    url: FfiStr,
    secret_key: *const c_uchar,
    group_type: c_uchar,
    data_type: c_uchar,
    bound_type: c_uchar,
    model_type: c_uchar,
    scalar: c_double,
    #[cfg(feature = "tls")] certificates: *const FfiStr,
    #[cfg(feature = "tls")] certificates_len: c_uint,
    #[cfg(feature = "tls")] identity: FfiStr,
) -> *mut CMobileClient {
    // we could return *const CMobileClient, however, the caller can ignore it
    // https://newrustacean.com/show_notes/e031/struct.script#strings

    // Check the URL of the coordinator.
    // Returns `NULL` if the value of URL is `NULL` or if the string contains
    // invalid UTF-8 characters.
    let url = match url.as_opt_str() {
        Some(url) => url,
        None => return ptr::null_mut(),
    };

    // Check the `secret key` of the client.
    // Returns `NULL` if the pointer points to `NULL`.
    //
    // Safety:
    // `core::ptr::const_ptr::as_ref` only ensures null-safety.
    // It is not guaranteed that the pointer is either properly aligned or
    // points to an initialized instance of *const c_uchar.
    let secret_key = match unsafe { secret_key.as_ref() } {
        Some(secret_key) => secret_key,
        None => return ptr::null_mut(),
    };

    let group_type = match GroupType::try_from(group_type) {
        Ok(group_type) => group_type,
        Err(_) => return ptr::null_mut(),
    };

    let data_type = match DataType::try_from(data_type) {
        Ok(data_type) => data_type,
        Err(_) => return ptr::null_mut(),
    };

    let bound_type = match BoundType::try_from(bound_type) {
        Ok(bound_type) => bound_type,
        Err(_) => return ptr::null_mut(),
    };

    let model_type = match ModelType::try_from(model_type) {
        Ok(model_type) => model_type,
        Err(_) => return ptr::null_mut(),
    };

    let secret_key = unsafe { slice::from_raw_parts(secret_key, ParticipantSecretKey::LENGTH) };
    let secret_key = ParticipantSecretKey::from_slice_unchecked(secret_key);

    let mask_config = MaskConfig {
        group_type,
        data_type,
        bound_type,
        model_type,
    };

    let participant_settings = ParticipantSettings {
        secret_key,
        aggregation_config: AggregationConfig {
            mask: mask_config,
            scalar,
        },
        max_message_size: MaxMessageSize::default(),
    };

    // Check the certificates.
    // Returns `None` if any of the pointers points to `NULL` or is invalid UTF-8.
    // Slice alignment and memory initialization safety concerns apply as usual.
    #[cfg(feature = "tls")]
    let certificates = certificate_paths_from(certificates, certificates_len);
    #[cfg(feature = "tls")]
    let identity = identity_path_from(identity);

    if let Ok(mobile_client) = MobileClient::init(
        url,
        participant_settings,
        #[cfg(feature = "tls")]
        certificates,
        #[cfg(feature = "tls")]
        identity,
    ) {
        Box::into_raw(Box::new(CMobileClient(mobile_client)))
    } else {
        ptr::null_mut()
    }
}

/// Restores a [`MobileClient`] from its serialized state.
///
/// # Parameters
///
/// - `url`: The URL fo the coordinator to which the [`MobileClient`] will try to connect to.
/// - `buffer`: The array that contains the serialized state.
/// - `len`: The length of `buffer`.
/// - `certificates`: The optional array of paths to DER/PEM encoded trusted server certificates for
///   TLS server authentication. Requires the `tls` feature to be enabled. Interprets null pointers
///   as `None` and is ignored for `certificates_len` of zero.
/// - `certificates_len`: The number of DER/PEM encoded optional certificates. Requires the `tls`
///   feature to be enabled. Interprets zero as `None`.
/// - `identity`: The optional path to a PEM encoded client certificate for TLS client
///   authentication. Requires the `tls` feature to be enabled. Interprets a null pointer as `None`.
///
/// Requires at least one of the following arguments if the `tls` feature is enabled:
/// - `certificates` together with `certificates_len`
/// - `identity`
///
/// # Safety
///
/// `buffer`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of `c_uchar`,
/// - the data the pointer points to is properly aligned,
/// - the data is valid for reads for `len` * mem::size_of::<c_uchar>() many bytes,
/// - the memory of `buffer` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_restore_mobile_client`].
///
/// `certificates`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of `*const FfiStr`
/// - the data the pointers point to are properly aligned and valid UTF-8,
/// - the data is valid for reads for `certificates_len * mem::size_of::<*const _>()` many bytes,
/// - the memory of the certificates is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_restore_mobile_client`].
///
/// `identity`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of `c_char`
/// - the data the pointer points to is properly aligned and valid UTF-8,
/// - the memory of the identity is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_restore_mobile_client`].
///
/// # Return Value
///
/// Returns a new instance of [`CMobileClient`].
///
/// ## Returns `NULL` if:
///
/// - the pointer of `url` or `buffer` points to `NULL`,
/// - `url` contains invalid UTF-8 characters.
/// - the TLS settings are invalid
///
/// [`MobileClient`]: xaynet_client::mobile_client::MobileClient
#[allow(unused_unsafe)]
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_restore_mobile_client(
    url: FfiStr,
    buffer: *const c_uchar,
    buffer_len: c_uint,
    #[cfg(feature = "tls")] certificates: *const FfiStr,
    #[cfg(feature = "tls")] certificates_len: c_uint,
    #[cfg(feature = "tls")] identity: FfiStr,
) -> *mut CMobileClient {
    let url = match url.as_opt_str() {
        Some(url) => url,
        None => return ptr::null_mut(),
    };

    let buffer = match unsafe { buffer.as_ref() } {
        Some(buffer) => buffer,
        None => return ptr::null_mut(),
    };
    let buffer = unsafe { slice::from_raw_parts(buffer, buffer_len as usize) };

    // Check the certificates.
    // Returns `None` if any of the pointers points to `NULL` or is invalid UTF-8.
    // Slice alignment and memory initialization safety concerns apply as usual.
    #[cfg(feature = "tls")]
    let certificates = certificate_paths_from(certificates, certificates_len);
    #[cfg(feature = "tls")]
    let identity = identity_path_from(identity);

    if let Ok(mobile_client) = MobileClient::restore(
        url,
        buffer,
        #[cfg(feature = "tls")]
        certificates,
        #[cfg(feature = "tls")]
        identity,
    ) {
        Box::into_raw(Box::new(CMobileClient(mobile_client)))
    } else {
        ptr::null_mut()
    }
}

/// Serializes the current state of `client`.
///
/// # Parameters
///
/// - `client`: A pointer that points to an instance of [`CMobileClient`].
///
/// # Safety
///
/// `client`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of [`CMobileClient`],
/// - the data the pointer points to is properly aligned,
/// - the memory of `client` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_serialize_mobile_client`].
///
/// # Return Value
///
/// Returns a new instance of [`BytesBuffer`] that contains the serialized state of `client`.
///
/// ## Returns `NULL` if:
///
/// - the pointer of `client` points to `NULL`.
#[allow(unused_unsafe)]
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_serialize_mobile_client(
    client: *const CMobileClient,
) -> *mut BytesBuffer {
    let client = match unsafe { client.as_ref() } {
        Some(client) => &client.0,
        None => return ptr::null_mut(),
    };

    Box::into_raw(Box::new(BytesBuffer(client.serialize())))
}

/// Tries to proceed with the current client task.
/// This will consume the current state of the client and produces a new one.
///
/// # Parameters
///
/// - `client`: A pointer that points to an instance of [`CMobileClient`].
///
/// # Safety
///
/// `client`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of [`CMobileClient`],
/// - the data the pointer points to is properly aligned,
/// - the memory of `client` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_get_current_state_mobile_client`].
///
/// # Return Value
///
/// Returns a new instance of [`CMobileClient`].
///
/// ## Returns `NULL` if:
///
/// - the pointer of `client` points to `NULL`.
#[allow(unused_unsafe)]
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_try_to_proceed_mobile_client(
    client: *mut CMobileClient,
) -> *mut CMobileClient {
    let client = match unsafe { client.as_mut() } {
        Some(client) => client,
        None => return ptr::null_mut(),
    };

    // access to the current mobile client
    let CMobileClient(client) = unsafe { *Box::from_raw(client) };

    // perform the task (consumes the current client)
    let client = match client.try_to_proceed() {
        Ok(new_client) => new_client,
        Err((old_client, _)) => old_client,
    };

    Box::into_raw(Box::new(CMobileClient(client)))
}

/// Returns the current state of `client`.
///
/// # Parameters
///
/// - `client`: A pointer that points to an instance of [`CMobileClient`].
///
/// # Safety
///
/// `client`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of [`CMobileClient`],
/// - the data the pointer points to is properly aligned,
/// - the memory of `client` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_get_current_state_mobile_client`].
///
/// # Return Value
///
/// - `-1`: the pointer of `client` points to `NULL`,
/// - `0`: `Awaiting` state,
/// - `1`: `Sum` state,
/// - `2`: `Update` state,
/// - `3`: `Sum2` state.
#[allow(unused_unsafe)]
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_get_current_state_mobile_client(
    client: *mut CMobileClient,
) -> c_int {
    let client = match unsafe { client.as_mut() } {
        Some(client) => &mut (*client).0,
        None => return -1 as c_int,
    };

    (client.get_current_state() as u8) as c_int
}

define_box_destructor!(CMobileClient, xaynet_ffi_destroy_mobile_client);

/// Fetches and returns the latest global model from the coordinator.
///
/// # Parameters
///
/// - `client`: A pointer that points to an instance of [`CMobileClient`].
/// - `data_type`: The [`DataType`] of the global model.
/// - `buffer`: The array in which the global model should be copied.
/// - `len`: The length of `buffer`.
///
/// # Note
///
/// The data type must match the data type that was used when the client was initialized.
///
/// # Safety
///
/// `client`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of [`CMobileClient`],
/// - the data the pointer points to is properly aligned,
/// - the memory of `client` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_get_global_model_mobile_client`].
///
/// `buffer`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of `c_void`,
/// - the data the pointer points to is properly aligned,
/// - the data is valid for writes for `len` * mem::size_of::<c_void>() many bytes,
/// - the memory of `buffer` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_get_global_model_mobile_client`].
///
/// # Return Value
///
/// - `-1`: the pointer of `client` points to `NULL`,
/// - `-2`: the pointer of `buffer` points to `NULL`,
/// - `-3`: the value of `data_type` is not a valid value (see the module documentation of [`xaynet_core::mask`] for more information),
/// - `-4`: the API request failed,
/// - `-5`: the global model does not fit into `buffer`,
/// - `-6`: the pointer of `client` points to `NULL`,
/// - `0`: success,
/// - `1`: no global model available,
#[allow(unused_unsafe)]
#[allow(clippy::unnecessary_cast)]
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_get_global_model_mobile_client(
    client: *mut CMobileClient,
    data_type: c_uchar,
    buffer: *mut c_void,
    len: c_uint,
) -> c_int {
    let client = match unsafe { client.as_mut() } {
        Some(client) => &mut (*client).0,
        None => return -1 as c_int,
    };

    if buffer.is_null() {
        return -2 as c_int;
    }

    let data_type = match DataType::try_from(data_type) {
        Ok(data_type) => data_type,
        Err(_) => return -3 as c_int,
    };

    let global_model = if let Ok(global_model) = client.get_global_model() {
        global_model
    } else {
        return -4 as c_int;
    };

    let global_model = if let Some(global_model) = global_model {
        global_model
    } else {
        return 1 as c_int;
    };

    let len = len as usize;
    match data_type {
        DataType::F32 => {
            // safety checks missing
            let buffer = unsafe { slice::from_raw_parts_mut(buffer as *mut f32, len) };
            for (i, p) in global_model.into_primitives().enumerate() {
                if i >= len {
                    return -5 as c_int;
                }
                if let Ok(p) = p {
                    buffer[i] = p;
                } else {
                    return -6 as c_int;
                }
            }
        }
        DataType::F64 => {
            let buffer = unsafe { slice::from_raw_parts_mut(buffer as *mut f64, len) };
            for (i, p) in global_model.into_primitives().enumerate() {
                if i >= len {
                    return -5 as c_int;
                }
                if let Ok(p) = p {
                    buffer[i] = p;
                } else {
                    return -6 as c_int;
                }
            }
        }
        DataType::I32 => {
            let buffer = unsafe { slice::from_raw_parts_mut(buffer as *mut i32, len) };
            for (i, p) in global_model.into_primitives().enumerate() {
                if i >= len {
                    return -5 as c_int;
                }
                if let Ok(p) = p {
                    buffer[i] = p;
                } else {
                    return -6 as c_int;
                }
            }
        }
        DataType::I64 => {
            let buffer = unsafe { slice::from_raw_parts_mut(buffer as *mut i64, len) };
            for (i, p) in global_model.into_primitives().enumerate() {
                if i >= len {
                    return -5 as c_int;
                }
                if let Ok(p) = p {
                    buffer[i] = p;
                } else {
                    return -6 as c_int;
                }
            }
        }
    };
    0 as c_int
}

/// Sets the local model.
///
/// The local model is only sent if the client has been selected as an update client.
/// If the client is an update client and no local model is available, the client remains
/// in this state until a local model has been set or a new round has been started by the
/// coordinator.
///
/// # Parameters
///
/// - `client`: A pointer that points to an instance of [`CMobileClient`].
/// - `data_type`: The [`DataType`] of the local model.
/// - `buffer`: The array in which the local model should be copied.
/// - `len`: The length of `buffer`.
///
/// # Note
///
/// The data type must match the data type that was used when the client was initialized.
///
/// # Safety
///
/// `client`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of [`CMobileClient`],
/// - the data the pointer points to is properly aligned,
/// - the memory of `client` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_set_local_model_mobile_client`].
///
/// `buffer`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of `c_void`,
/// - the data the pointer points to is properly aligned,
/// - the data is valid for writes for `len` * mem::size_of::<c_void>() many bytes,
/// - the memory of `buffer` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_set_local_model_mobile_client`].
///
/// # Return Value
///
/// - `-1`: the pointer of `client` points to `NULL`,
/// - `-2`: the pointer of `buffer` points to `NULL`,
/// - `-3`: the value of `data_type` is not a valid value (see the module documentation of [`xaynet_core::mask`] for more information),
/// - `-4`: failed to create a model,
/// - `0`: success,
#[allow(unused_unsafe)]
#[allow(clippy::unnecessary_cast)]
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_set_local_model_mobile_client(
    client: *mut CMobileClient,
    data_type: c_uchar,
    buffer: *const c_void,
    len: c_uint,
) -> c_int {
    let client = match unsafe { client.as_mut() } {
        Some(client) => &mut (*client).0,
        None => return -1 as c_int,
    };

    if buffer.is_null() {
        return -2 as c_int;
    }

    let data_type = match DataType::try_from(data_type) {
        Ok(data_type) => data_type,
        Err(_) => return -3 as c_int,
    };

    let len = len as usize;
    let model = match data_type {
        DataType::F32 => {
            let buffer = unsafe { slice::from_raw_parts(buffer as *const f32, len) };
            // we map the error so that we get an uniform error type
            Model::from_primitives(buffer.iter().copied()).map_err(|_| ())
        }
        DataType::F64 => {
            let buffer = unsafe { slice::from_raw_parts(buffer as *const f64, len) };
            Model::from_primitives(buffer.iter().copied()).map_err(|_| ())
        }
        DataType::I32 => {
            let buffer = unsafe { slice::from_raw_parts(buffer as *const i32, len) };
            Model::from_primitives(buffer.iter().copied()).map_err(|_| ())
        }
        DataType::I64 => {
            let buffer = unsafe { slice::from_raw_parts(buffer as *const i64, len) };
            Model::from_primitives(buffer.iter().copied()).map_err(|_| ())
        }
    };

    if let Ok(m) = model {
        client.set_local_model(m);
        0_i32 as c_int
    } else {
        -4_i32 as c_int
    }
}

/// Creates a new participant secret key and writes it into `buffer`.
///
/// # Parameters
///
/// - `buffer`: A pointer that points to an instance of `c_uchar`.
///
/// # Safety
///
/// `buffer`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of `c_uchar`,
/// - the data the pointer points to is properly aligned,
/// - the data is valid for writes for [`ParticipantSecretKey::LENGTH`] * mem::size_of::<c_uchar>()
/// many bytes,
/// - the memory of `buffer` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_new_secret_key`].
///
/// # Return Value
///
/// - `-1`: the pointer of `buffer` points to `NULL`,
/// - `0`: success.
#[allow(unused_unsafe)]
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_new_secret_key(buffer: *mut c_uchar) -> c_int {
    let buffer = match unsafe { buffer.as_mut() } {
        Some(buffer) => buffer,
        None => return -1 as c_int,
    };

    let buffer = unsafe { slice::from_raw_parts_mut(buffer, ParticipantSecretKey::LENGTH) };
    buffer.copy_from_slice(MobileClient::create_participant_secret_key().as_slice());
    0 as c_int
}

/// ByteBuffer
/// A helper struct for sequences with an unknown size at compile-time.
pub struct BytesBuffer(Vec<u8>);

define_box_destructor!(BytesBuffer, xaynet_ffi_destroy_byte_buffer);

/// Returns the length of `buffer`.
///
/// # Parameters
///
/// - `buffer`: A pointer that points to an instance of [`BytesBuffer`].
///
/// # Safety
///
/// `buffer`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of [`BytesBuffer`],
/// - the memory of `buffer` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_get_len_of_byte_buffer`].
///
/// # Return Value
///
/// - `-1`: the pointer of `buffer` points to `NULL`,
/// - `> -1`: the length of `buffer`.
#[allow(unused_unsafe)]
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_get_len_of_byte_buffer(buffer: *const BytesBuffer) -> c_int {
    let buffer = match unsafe { buffer.as_ref() } {
        Some(buffer) => &buffer.0,
        None => return -1 as c_int,
    };

    buffer.len() as c_int
}

/// Copies the content of `buffer` into `foreign_buffer`.
///
/// # Parameters
///
/// - `buffer`: A pointer that points to an instance of [`BytesBuffer`].
/// - `foreign_buffer`: A pointer that points to an instance of `c_uchar`.
///
/// # Safety
///
/// `buffer`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of [`BytesBuffer`],
/// - the memory of `buffer` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_copy_into_foreign_buffer`].
///
/// `foreign_buffer`:
///
/// The function only ensures null-safety. You must ensure that:
/// - the pointer points to an initialized instance of `c_uchar`,
/// - the data the pointer points to is properly aligned,
/// - the data is valid for writes for `buffer_length` * mem::size_of::<c_uchar>() many bytes,
/// - the memory of `foreign_buffer` is not mutated (from the outside of this function)
/// for the duration of the execution of [`xaynet_ffi_copy_into_foreign_buffer`].
///
/// # Return Value
///
/// - `-1`: the pointer of `buffer` points to `NULL`,
/// - `-2`: the pointer of `foreign_buffer` points to `NULL`,
/// - `0`: success.
#[allow(unused_unsafe)]
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_copy_into_foreign_buffer(
    buffer: *const BytesBuffer,
    foreign_buffer: *mut c_uchar,
) -> c_int {
    let buffer = match unsafe { buffer.as_ref() } {
        Some(buffer) => &buffer.0,
        None => return -1 as c_int,
    };

    let foreign_buffer = match unsafe { foreign_buffer.as_mut() } {
        Some(foreign_buffer) => foreign_buffer,
        None => return -2 as c_int,
    };

    let foreign_buffer = unsafe { slice::from_raw_parts_mut(foreign_buffer, buffer.len()) };
    foreign_buffer.copy_from_slice(buffer.as_slice());
    0 as c_int
}
