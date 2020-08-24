//! A C-API to communicate model updates between a XayNet participant and an application.
//!
//! # Workflow
//! 1. Initialize a [`Client`] with [`new_client()`]. The [`Client`] takes care of the
//!    [`Participant`]'s PET protocol work as well as the networking with the [`Coordinator`].
//! 2. Start the execution of the [`Client`]'s tasks with [`run_client()`].
//! 3. Optionally request status information:
//!     - [`is_next_round()`] indicates if another round of the PET protocol has started.
//!     - [`has_next_model()`] indicates if another global model is available.
//!     - [`is_update_participant()`] indicates if this [`Participant`] is eligible to submit a
//!       trained local model in the current round.
//! 4. Create a new zero-initialized model with [`new_model()`] or get the latest global model with
//!    [`get_model()`]. Currently, the primitive data types [`f32`], [`f64`], [`i32`] and [`i64`]
//!    are supported. The functions return a fat pointer [`PrimitiveModel`] to the cached primitive
//!    model, whereas the primitive model itself is cached within the [`Client`]. The cached
//!    primitive model can then be modified in place, for example for training. The slice is valid
//!    across the FFI-boundary until one of the following happens:
//!    - [`new_model()`] reallocates the memory to which [`PrimitiveModel`] points to.
//!    - [`get_model()`] reallocates the memory to which [`PrimitiveModel`] points to if a new
//!      global model is available since the last call to [`get_model()`].
//!    - [`update_model()`] frees the memory to which [`PrimitiveModel`] points to.
//!    - [`drop_model()`] frees the memory to which [`PrimitiveModel`] points to.
//!    - [`drop_client()`] frees the memory of the [`Client`] including the model.
//! 5. Register the cached model as an updated local model with [`update_model()`].
//! 6. Stop and destroy the [`Client`] with [`drop_client()`].
//!
//! # Safety
//! Many functions of this module are marked as `unsafe` to explicitly announce the possible
//! unsafety of the function body as well as the return value to the caller. At the same time,
//! each `unsafe fn` uses `unsafe` blocks to precisely pinpoint the sources of unsafety for
//! reviewers (redundancy warnings will be fixed by [#69173]).
//!
//! **Note, that the `unsafe` code has not been externally audited yet!**
//!
//! [`Coordinator`]: ../../coordinator/struct.Coordinator.html
//! [`Participant`]: ../../participant/struct.Participant.html
//! [#69173]: https://github.com/rust-lang/rust/issues/69173

use std::{
    ffi::CStr,
    iter::{IntoIterator, Iterator},
    mem,
    os::raw::{c_char, c_int, c_uint, c_ulonglong, c_void},
    panic,
    ptr,
};

use tokio::{
    runtime::{Builder, Runtime},
    time::Duration,
};

use crate::{
    client::{api::HttpApiClient, Client, ClientError, Task},
    mask::model::{FromPrimitives, IntoPrimitives, Model},
};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
/// A fat pointer to a cached model of primitive data type which can be accessed from C.
///
/// This is returned from [`new_model()`] and [`get_model()`]. The length `len` will be small enough
/// such that the respective array fits into memory and the data type `dtype` will be one of the
/// following:
/// - `0`: void data type
/// - `1`: primitive data type [`f32`]
/// - `2`: primitive data type [`f64`]
/// - `3`: primitive data type [`i32`]
/// - `4`: primitive data type [`i64`]
pub struct PrimitiveModel {
    /// A raw mutable pointer to an array of primitive values.
    pub ptr: *mut c_void,
    /// The length of that array.
    pub len: c_ulonglong,
    /// The data type of the array's elements.
    pub dtype: c_uint,
}

#[derive(Clone, Debug)]
/// A primitive model cached on the heap.
///
/// The fat pointer [`PrimitiveModel`] returned from [`new_model()`] and [`get_model()`] references
/// this memory.
pub(crate) enum CachedModel {
    F32(Vec<f32>),
    F64(Vec<f64>),
    I32(Vec<i32>),
    I64(Vec<i64>),
}

/// A wrapper for a [`Client`] within an asynchronous runtime.
///
/// This is returned from [`new_client()`]. See the [workflow] on how to use it.
///
/// [workflow]: index.html#workflow
pub struct FFIClient {
    client: Client<HttpApiClient>,
    runtime: Runtime,
}

#[allow(unused_unsafe)]
#[allow(clippy::unnecessary_cast)]
#[no_mangle]
/// Creates a new [`Client`] within an asynchronous runtime.
///
/// Takes a network `address` to the coordinator to which the [`Client`] will try to connect to.
///
/// Takes a `period` in seconds after which the [`Client`] will try to poll the coordinator for new
/// broadcasted FL round data again.
///
/// # Errors
/// Ignores null pointer `address`es and zero `period`s and returns a null pointer immediately.
///
/// Returns a null pointer if the initialization of the runtime or the client fails.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of the method is
/// undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn new_client(address: *const c_char, period: c_ulonglong) -> *mut FFIClient {
    if address.is_null() || period == 0 {
        return ptr::null_mut() as *mut FFIClient;
    }
    let address = if let Ok(address) = unsafe {
        // safe if the raw pointer `address` comes from a null-terminated C-string
        CStr::from_ptr(address)
    }
    .to_str()
    {
        address
    } else {
        return ptr::null_mut() as *mut FFIClient;
    };
    let runtime = if let Ok(runtime) = Builder::new()
        .threaded_scheduler()
        .core_threads(1)
        .max_threads(4)
        .thread_name("xaynet-client-runtime-worker")
        .enable_all()
        .build()
    {
        runtime
    } else {
        return ptr::null_mut() as *mut FFIClient;
    };
    let client = if let Ok(client) =
        runtime.enter(move || Client::new(period as u64, 0, HttpApiClient::new(address)))
    {
        client
    } else {
        return ptr::null_mut() as *mut FFIClient;
    };
    Box::into_raw(Box::new(FFIClient { runtime, client }))
}

#[allow(unused_unsafe)]
#[allow(clippy::unnecessary_cast)]
#[no_mangle]
/// Starts the [`Client`] and executes its tasks in an asynchronous runtime.
///
/// # Errors
/// Ignores null pointer `client`s and returns an error immediately.
///
/// If the client must be stopped because of a panic or error or when the client terminates
/// successfully, then one of the following error codes is returned:
/// - `-1`: client didn't start due to null pointer
/// - `0`: no error (only for clients with finite running time)
/// - `1`: client panicked due to unexpected/unhandled error
/// - `2`: client stopped due to error [`ParticipantInitErr`]
/// - `3`: client stopped due to error [`ParticipantErr`]
/// - `4`: client stopped due to error [`TooEarly`]
/// - `5`: client stopped due to error [`RoundOutdated`]
/// - `6`: client stopped due to error [`Api`]
///
/// # Safety
///
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of the method is
/// undefined if the arguments don't point to valid objects.
///
/// If the client panicked (error code `1`), it is the users responsibility to not access possibly
/// invalid client state and to drop the client.
///
/// [`ParticipantInitErr`]: ../../client/enum.ClientError.html#variant.ParticipantInitErr
/// [`ParticipantErr`]: ../../client/enum.ClientError.html#variant.ParticipantErr
/// [`TooEarly`]: ../../client/enum.ClientError.html#variant.TooEarly
/// [`RoundOutdated`]: ../../client/enum.ClientError.html#variant.RoundOutdated
/// [`Api`]: ../../client/enum.ClientError.html#variant.Api
pub unsafe extern "C" fn run_client(client: *mut FFIClient) -> c_int {
    if client.is_null() {
        return -1_i32 as c_int;
    }
    let (runtime, client) = unsafe {
        // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
        (&(*client).runtime, &mut (*client).client)
    };

    // `UnwindSafe` basically says that there is no danger in accessing a value after a panic
    // happened. this is generally true for immutable references, but not for mutable references.
    // currently the docs have a note that it is the user's responsibility to not access possibly
    // invalid values (we could clean up the client ourself but then the paradigm of create-use-
    // destroy all happening on one side, in this case the non-Rust side of the API, is violated
    // and might as well lead to severe bugs/segfaults). the main issue is that we must catch the
    // panic, because letting it propagate across the FFI-boundary is undefined behavior and will
    // most likely result in segfaults. what we can do is to improve error handling on our side to
    // reduce the number of possible panics and return proper errors instead.
    match panic::catch_unwind(unsafe {
        // even though `&mut Client` is `!UnwindSafe` we can assert this because the user will be
        // notified about a panic immediately to be able to safely act accordingly
        panic::AssertUnwindSafe(|| runtime.handle().block_on(client.start()))
    }) {
        Ok(Ok(_)) => 0_i32 as c_int,
        Err(_) => 1_i32 as c_int,
        Ok(Err(ClientError::ParticipantInitErr(_))) => 2_i32 as c_int,
        Ok(Err(ClientError::ParticipantErr(_))) => 3_i32 as c_int,
        Ok(Err(ClientError::TooEarly(_))) => 4_i32 as c_int,
        Ok(Err(ClientError::RoundOutdated)) => 5_i32 as c_int,
        Ok(Err(ClientError::Api(_))) => 6_i32 as c_int,
    }
}

#[allow(unused_unsafe)]
#[allow(clippy::unnecessary_cast)]
#[no_mangle]
/// Stops and destroys a [`Client`] and frees its allocated memory.
///
/// Tries to gracefully stop the client for `timeout` seconds by blocking the current thread before
/// shutting it down forcefully (outstanding tasks are potentially leaked in case of an elapsed
/// timeout). Usually, no timeout (i.e. 0 seconds) suffices, but stopping might take indefinitely
/// if the client performs long blocking tasks.
///
/// # Errors
/// Ignores null pointer `client`s and returns immediately.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of the method is
/// undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn drop_client(client: *mut FFIClient, timeout: c_ulonglong) {
    if !client.is_null() {
        let client = unsafe {
            // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
            Box::from_raw(client)
        };
        if timeout as usize != 0 {
            client
                .runtime
                .shutdown_timeout(Duration::from_secs(timeout as u64));
        }
    }
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Checks if the next round has started.
///
/// # Errors
/// Ignores null pointer `client`s and returns `false` immediately.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of the method is
/// undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn is_next_round(client: *mut FFIClient) -> bool {
    if client.is_null() {
        false
    } else {
        let client = unsafe {
            // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
            &mut (*client).client
        };
        mem::replace(&mut client.has_new_coord_pk_since_last_check, false)
    }
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Checks if the next global model is available.
///
/// # Errors
/// Ignores null pointer `client`s and returns `false` immediately.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of the method is
/// undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn has_next_model(client: *mut FFIClient) -> bool {
    if client.is_null() {
        false
    } else {
        let client = unsafe {
            // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
            &mut (*client).client
        };
        mem::replace(&mut client.has_new_global_model_since_last_check, false)
    }
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Checks if the current role of the participant is [`Update`].
///
/// # Errors
/// Ignores null pointer `client`s and returns `false` immediately.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of the method is
/// undefined if the arguments don't point to valid objects.
///
/// [`Update`]: ../../participant/enum.Task.html#variant.Update
pub unsafe extern "C" fn is_update_participant(client: *mut FFIClient) -> bool {
    if client.is_null() {
        false
    } else {
        let client = unsafe {
            // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
            &(*client).client
        };
        client.participant.task == Task::Update
    }
}

#[allow(unused_unsafe)]
#[allow(clippy::unnecessary_cast)]
#[no_mangle]
/// Gets a mutable slice [`PrimitiveModel`] to a zero-initialized model of given primitive data type
/// `dtype` and length `len`.
///
/// The new model gets cached, which overwrites any existing cached model. The cache and slice are
/// valid as described in step 4 of the [workflow]. The cached model can be modified in place, for
/// example for training.
///
/// The following data types `dtype` are currently supported:
/// - `1`: [`f32`]
/// - `2`: [`f64`]
/// - `3`: [`i32`]
/// - `4`: [`i64`]
///
/// # Errors
/// Ignores null pointer `client`s and returns a [`PrimitiveModel`] with null pointer, length zero
/// and void data type immediately.
///
/// Returns a [`PrimitiveModel`] with null pointer, length zero and void data type if the model is
/// not representable in memory due to the given length `len` and data type `dtype`.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of the method is
/// undefined if the arguments don't point to valid objects.
///
/// [workflow]: index.html#workflow
pub unsafe extern "C" fn new_model(
    client: *mut FFIClient,
    dtype: c_uint,
    len: c_ulonglong,
) -> PrimitiveModel {
    let max_len = match dtype {
        1 | 3 => isize::MAX / 4,
        2 | 4 => isize::MAX / 8,
        _ => 0,
    } as c_ulonglong;
    if client.is_null() || dtype == 0 || dtype > 4 || len == 0 || len > max_len {
        return PrimitiveModel {
            ptr: ptr::null_mut() as *mut c_void,
            len: 0_u64 as c_ulonglong,
            dtype: 0_u32 as c_uint,
        };
    }
    let client = unsafe {
        // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
        &mut (*client).client
    };
    let ptr = match dtype {
        1 => {
            let mut cached_model = vec![0_f32; len as usize];
            let ptr = cached_model.as_mut_ptr() as *mut c_void;
            client.cached_model = Some(CachedModel::F32(cached_model));
            ptr
        }
        2 => {
            let mut cached_model = vec![0_f64; len as usize];
            let ptr = cached_model.as_mut_ptr() as *mut c_void;
            client.cached_model = Some(CachedModel::F64(cached_model));
            ptr
        }
        3 => {
            let mut cached_model = vec![0_i32; len as usize];
            let ptr = cached_model.as_mut_ptr() as *mut c_void;
            client.cached_model = Some(CachedModel::I32(cached_model));
            ptr
        }
        4 => {
            let mut cached_model = vec![0_i64; len as usize];
            let ptr = cached_model.as_mut_ptr() as *mut c_void;
            client.cached_model = Some(CachedModel::I64(cached_model));
            ptr
        }
        _ => unreachable!(),
    };
    PrimitiveModel { ptr, len, dtype }
}

#[allow(unused_unsafe)]
#[allow(clippy::unnecessary_cast)]
#[no_mangle]
/// Gets a mutable slice [`PrimitiveModel`] to the latest global model converted to the primitive
/// data type `dtype`.
///
/// The global model gets cached, which overwrites any existing cached model. The cache and slice
/// are valid as described in step 4 of the [workflow]. The cached model can be modified in place,
/// for example for training.
///
/// The following data types `dtype` are currently supported:
/// - `1`: [`f32`]
/// - `2`: [`f64`]
/// - `3`: [`i32`]
/// - `4`: [`i64`]
///
/// # Errors
/// Ignores null pointer `client`s and invalid `dtype`s and returns a [`PrimitiveModel`] with null
/// pointer, length zero and void data type immediately.
///
/// Returns a [`PrimitiveModel`] with null pointer, length zero and data type `dtype` if no global
/// model is available.
///
/// Returns a [`PrimitiveModel`] with null pointer, length of the global model and data type `dtype`
/// if the conversion of the global model into the primitive data type fails.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of the method is
/// undefined if the arguments don't point to valid objects.
///
/// [workflow]: index.html#workflow
pub unsafe extern "C" fn get_model(client: *mut FFIClient, dtype: c_uint) -> PrimitiveModel {
    if client.is_null() || dtype == 0 || dtype > 4 {
        return PrimitiveModel {
            ptr: ptr::null_mut() as *mut c_void,
            len: 0_u64 as c_ulonglong,
            dtype: 0_u32 as c_uint,
        };
    }
    let client = unsafe {
        // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
        &mut (*client).client
    };

    // global model available
    if let Some(ref global_model) = client.global_model {
        // global model is already cached as a primitive model
        if !client.has_new_global_model_since_last_cache {
            match dtype {
                1 => {
                    if let Some(CachedModel::F32(ref mut cached_model)) = client.cached_model {
                        return PrimitiveModel {
                            ptr: cached_model.as_mut_ptr() as *mut c_void,
                            len: cached_model.len() as c_ulonglong,
                            dtype,
                        };
                    }
                }
                2 => {
                    if let Some(CachedModel::F64(ref mut cached_model)) = client.cached_model {
                        return PrimitiveModel {
                            ptr: cached_model.as_mut_ptr() as *mut c_void,
                            len: cached_model.len() as c_ulonglong,
                            dtype,
                        };
                    }
                }
                3 => {
                    if let Some(CachedModel::I32(ref mut cached_model)) = client.cached_model {
                        return PrimitiveModel {
                            ptr: cached_model.as_mut_ptr() as *mut c_void,
                            len: cached_model.len() as c_ulonglong,
                            dtype,
                        };
                    }
                }
                4 => {
                    if let Some(CachedModel::I64(ref mut cached_model)) = client.cached_model {
                        return PrimitiveModel {
                            ptr: cached_model.as_mut_ptr() as *mut c_void,
                            len: cached_model.len() as c_ulonglong,
                            dtype,
                        };
                    }
                }
                _ => unreachable!(),
            }
        }

        // convert the global model to a primitive model and cache it
        client.has_new_global_model_since_last_cache = false;
        let len = global_model.len() as c_ulonglong;
        let ptr = match dtype {
            1 => {
                if let Ok(mut cached_model) = global_model
                    .to_primitives()
                    .map(|res| res.map_err(|_| ()))
                    .collect::<Result<Vec<f32>, ()>>()
                {
                    // conversion succeeded
                    let ptr = cached_model.as_mut_ptr() as *mut c_void;
                    client.cached_model = Some(CachedModel::F32(cached_model));
                    ptr
                } else {
                    // conversion failed
                    client.cached_model = None;
                    ptr::null_mut() as *mut c_void
                }
            }
            2 => {
                if let Ok(mut cached_model) = global_model
                    .to_primitives()
                    .map(|res| res.map_err(|_| ()))
                    .collect::<Result<Vec<f64>, ()>>()
                {
                    // conversion succeeded
                    let ptr = cached_model.as_mut_ptr() as *mut c_void;
                    client.cached_model = Some(CachedModel::F64(cached_model));
                    ptr
                } else {
                    // conversion failed
                    client.cached_model = None;
                    ptr::null_mut() as *mut c_void
                }
            }
            3 => {
                if let Ok(mut cached_model) = global_model
                    .to_primitives()
                    .map(|res| res.map_err(|_| ()))
                    .collect::<Result<Vec<i32>, ()>>()
                {
                    // conversion succeeded
                    let ptr = cached_model.as_mut_ptr() as *mut c_void;
                    client.cached_model = Some(CachedModel::I32(cached_model));
                    ptr
                } else {
                    // conversion failed
                    client.cached_model = None;
                    ptr::null_mut() as *mut c_void
                }
            }
            4 => {
                if let Ok(mut cached_model) = global_model
                    .to_primitives()
                    .map(|res| res.map_err(|_| ()))
                    .collect::<Result<Vec<i64>, ()>>()
                {
                    // conversion succeeded
                    let ptr = cached_model.as_mut_ptr() as *mut c_void;
                    client.cached_model = Some(CachedModel::I64(cached_model));
                    ptr
                } else {
                    // conversion failed
                    client.cached_model = None;
                    ptr::null_mut() as *mut c_void
                }
            }
            _ => unreachable!(),
        };
        return PrimitiveModel { ptr, len, dtype };
    }

    // global model unavailable
    client.cached_model = None;
    PrimitiveModel {
        ptr: ptr::null_mut() as *mut c_void,
        len: 0_u64 as c_ulonglong,
        dtype: 0_u32 as c_uint,
    }
}

#[allow(unused_unsafe)]
#[allow(clippy::unnecessary_cast)]
#[no_mangle]
/// Registers the cached model as an updated local model.
///
/// This clears the cached model.
///
/// # Errors
/// Ignores null pointer `client`s and returns immediately.
///
/// Returns an error if there is no cached model to register.
///
/// The error codes are as following:
/// - `-1`: client didn't update due to null pointer
/// - `0`: no error
/// - `1`: client didn't update due missing cached model
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of the method is
/// undefined if the arguments don't point to valid objects.
///
/// The memory of the cached model is is either allocated by [`new_model()`] or [`get_model()`].
/// Therefore, the behavior of the method is undefined if the memory was modified in an invalid way.
pub unsafe extern "C" fn update_model(client: *mut FFIClient) -> c_int {
    if client.is_null() {
        return -1_i32 as c_int;
    }
    let client = unsafe {
        // safe if the raw pointer `client` comes from a valid allocation of a `Client`
        &mut (*client).client
    };
    client.local_model = match client.cached_model.take() {
        Some(CachedModel::F32(cached_model)) => {
            Some(Model::from_primitives_bounded(cached_model.into_iter()))
        }
        Some(CachedModel::F64(cached_model)) => {
            Some(Model::from_primitives_bounded(cached_model.into_iter()))
        }
        Some(CachedModel::I32(cached_model)) => {
            Some(Model::from_primitives_bounded(cached_model.into_iter()))
        }
        Some(CachedModel::I64(cached_model)) => {
            Some(Model::from_primitives_bounded(cached_model.into_iter()))
        }
        None => return 1_i32 as c_int,
    };
    0_i32 as c_int
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Destroys a [`Client`]'s cached primitive model and frees its allocated memory.
///
/// It is not necessary to call this function if [`update_model()`] or [`drop_client()`] is called
/// anyways.
///
/// # Errors
/// Ignores null pointer `client`s and returns immediately.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of the method is
/// undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn drop_model(client: *mut FFIClient) {
    if !client.is_null() {
        let client = unsafe {
            // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
            &mut (*client).client
        };
        client.cached_model.take();
    }
}

// Temporary Dart wrappers. Will be removed once booleans are supported in Dart FFI, see
// https://github.com/dart-lang/sdk/issues/36855.
pub use self::dart::*;

mod dart {
    use std::os::raw::c_uint;

    #[allow(unused_unsafe)]
    #[allow(clippy::unnecessary_cast)]
    #[no_mangle]
    #[doc(hidden)]
    pub unsafe extern "C" fn is_next_round_dart(client: *mut super::FFIClient) -> c_uint {
        if unsafe {
            // safe if the called function is sound
            super::is_next_round(client)
        } {
            1_u32 as c_uint
        } else {
            0_u32 as c_uint
        }
    }

    #[allow(unused_unsafe)]
    #[allow(clippy::unnecessary_cast)]
    #[no_mangle]
    #[doc(hidden)]
    pub unsafe extern "C" fn has_next_model_dart(client: *mut super::FFIClient) -> c_uint {
        if unsafe {
            // safe if the called function is sound
            super::has_next_model(client)
        } {
            1_u32 as c_uint
        } else {
            0_u32 as c_uint
        }
    }

    #[allow(unused_unsafe)]
    #[allow(clippy::unnecessary_cast)]
    #[no_mangle]
    #[doc(hidden)]
    pub unsafe extern "C" fn is_update_participant_dart(client: *mut super::FFIClient) -> c_uint {
        if unsafe {
            // safe if the called function is sound
            super::is_update_participant(client)
        } {
            1_u32 as c_uint
        } else {
            0_u32 as c_uint
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::CString, iter::FromIterator};

    use num::rational::Ratio;

    use super::*;

    #[test]
    fn test_new_client() {
        let client = unsafe { new_client(CString::new("0.0.0.0:0000").unwrap().as_ptr(), 10) };
        assert!(!client.is_null());
        unsafe { drop_client(client, 0) };
    }

    #[test]
    fn test_run_client() {
        // check for network error when running client without a service
        let client = unsafe { new_client(CString::new("0.0.0.0:0000").unwrap().as_ptr(), 10) };
        assert_eq!(unsafe { run_client(client) }, 6);
        unsafe { drop_client(client, 0) };
    }

    // define dummy model of length `len` where all values are set to `val`
    fn dummy_model(val: f64, len: usize) -> Model {
        Model::from_iter(vec![Ratio::from_float(val).unwrap(); len].into_iter())
    }

    macro_rules! test_new_model {
        ($prim:ty, $dtype:expr) => {
            paste::item! {
                #[allow(unused_unsafe)]
                #[test]
                fn [<test_new_model_ $prim>]() {
                    let client = unsafe { new_client(CString::new("0.0.0.0:0000").unwrap().as_ptr(), 10) };

                    // check that the new model is cached
                    let model = dummy_model(0., 10);
                    let prim_model = unsafe { new_model(client, $dtype as c_uint, 10 as c_ulonglong) };
                    if let Some(CachedModel::[<$prim:upper>](ref cached_model)) = unsafe { &mut *client }.client.cached_model {
                        assert_eq!(prim_model.ptr, cached_model.as_ptr() as *mut c_void);
                        assert_eq!(prim_model.len, cached_model.len() as c_ulonglong);
                        assert_eq!(prim_model.dtype, $dtype as c_uint);
                        assert_eq!(model, Model::from_primitives_bounded(cached_model.iter().cloned()));
                    } else {
                        panic!();
                    }
                    unsafe { drop_client(client, 0) };
                }
            }
        };
    }

    test_new_model!(f32, 1);
    test_new_model!(f64, 2);
    test_new_model!(i32, 3);
    test_new_model!(i64, 4);

    macro_rules! test_get_model {
        ($prim:ty, $dtype:expr) => {
            paste::item! {
                #[allow(unused_unsafe)]
                #[test]
                fn [<test_get_model_ $prim>]() {
                    let client = unsafe { new_client(CString::new("0.0.0.0:0000").unwrap().as_ptr(), 10) };

                    // check that the primitive model is null if the global model is unavailable
                    assert!(unsafe { &*client }.client.global_model.is_none());
                    let prim_model = unsafe { get_model(client, $dtype as c_uint) };
                    assert!(unsafe { &*client }.client.cached_model.is_none());
                    assert!(prim_model.ptr.is_null());
                    assert_eq!(prim_model.len, 0);
                    assert_eq!(prim_model.dtype, 0);

                    // check that the primitive model points to the cached model if the global model is available
                    let model = dummy_model(0., 10);
                    unsafe { &mut *client }.client.global_model = Some(model.clone());
                    let prim_model = unsafe { get_model(client, $dtype as c_uint) };
                    if let Some(CachedModel::[<$prim:upper>](ref cached_model)) = unsafe { &mut *client }.client.cached_model {
                        assert_eq!(prim_model.ptr, cached_model.as_ptr() as *mut c_void);
                        assert_eq!(prim_model.len, cached_model.len() as c_ulonglong);
                        assert_eq!(prim_model.dtype, $dtype as c_uint);
                        assert_eq!(model, Model::from_primitives_bounded(cached_model.iter().cloned()));
                    } else {
                        panic!();
                    }
                    unsafe { drop_client(client, 0) };
                }
            }
        };
    }

    test_get_model!(f32, 1);
    test_get_model!(f64, 2);
    test_get_model!(i32, 3);
    test_get_model!(i64, 4);

    macro_rules! test_update_model {
        ($prim:ty, $dtype:expr) => {
            paste::item! {
                #[test]
                fn [<test_update_model_ $prim>]() {
                    let client = unsafe { new_client(CString::new("0.0.0.0:0000").unwrap().as_ptr(), 10) };
                    let model = dummy_model(0., 10);
                    unsafe { &mut *client }.client.global_model = Some(model.clone());
                    let prim_model = unsafe { get_model(client, $dtype as c_uint) };

                    // check that the local model is updated from the cached model
                    if let Some(CachedModel::[<$prim:upper>](ref cached_model)) = unsafe { &mut *client }.client.cached_model {
                        assert_eq!(prim_model.ptr, cached_model.as_ptr() as *mut c_void);
                        assert_eq!(prim_model.len, cached_model.len() as c_ulonglong);
                        assert_eq!(prim_model.dtype, $dtype as c_uint);
                    } else {
                        panic!();
                    }
                    assert!(unsafe {  &*client }.client.local_model.is_none());
                    assert_eq!(unsafe { update_model(client) }, 0);
                    assert!(unsafe { &mut *client }.client.cached_model.is_none());
                    if let Some(ref local_model) = unsafe { &*client }.client.local_model {
                        assert_eq!(&model, local_model);
                    } else {
                        panic!();
                    }
                    unsafe { drop_client(client, 0) };
                }
            }
        };
    }

    test_update_model!(f32, 1);
    test_update_model!(f64, 2);
    test_update_model!(i32, 3);
    test_update_model!(i64, 4);
}
