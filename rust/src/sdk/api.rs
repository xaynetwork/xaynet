//! A C-API to communicate model updates between a PET protocol participant and an application.
//!
//! # Workflow
//! 1. Initialize a [`Client`] with [`new_client()`], which will take care of the [`Participant`]'s
//!    PET protocol work as well as the networking with the [`Coordinator`].
//! 2. Start the execution of tasks of the [`Client`] with [`run_client()`].
//! 3. Optionally request status information:
//!     - [`is_next_round()`] indicates if another round of the PET protocol has started.
//!     - [`has_next_model()`] indicates if another global model is available.
//!     - [`is_update_participant()`] indicates if this [`Participant`] is eligible to submit a
//!       trained local model.
//! 4. Create a new model initialized with zeros with [`new_model_N()`] or get the latest global
//!    model with [`get_model_N()`], where `N` is the primitive data type. Currently, [`f32`],
//!    [`f64`], [`i32`] and [`i64`] are supported. The function returns a mutable slice
//!    [`PrimitiveModelN`] to the primitive model, whereas the primitive model itself is cached
//!    within the [`Client`]. The slice is valid across the FFI-boundary until one of the
//!    following events happen:
//!    - [`new_model_N()`] reallocates the memory to which [`PrimitiveModelN`] points to.
//!    - [`get_model_N()`] reallocates the memory to which [`PrimitiveModelN`] points to if a new
//!      global model is available since the last call to [`get_model_N()`].
//!    - [`update_model_N()`] frees the memory to which [`PrimitiveModelN`] points to.
//!    - [`drop_model()`] frees the memory to which [`PrimitiveModelN`] points to.
//!    - [`drop_client()`] frees the memory of the [`Client`] including the model.
//!    The cached model can be modified in place, for example for training.
//! 5. Register a trained local model with [`update_model_N()`].
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
//! [`new_model_N()`]: fn.new_model_f32.html
//! [`get_model_N()`]: fn.get_model_f32.html
//! [`update_model_N()`]: fn.update_model_f32.html
//! [`Participant`]: ../../participant/struct.Participant.html
//! [`PrimitiveModelN`]: struct.PrimitiveModelF32.html
//! [#69173]: https://github.com/rust-lang/rust/issues/69173

use std::{
    iter::{IntoIterator, Iterator},
    mem,
    os::raw::{c_double, c_float, c_int, c_long, c_ulong, c_void},
    panic,
    ptr,
};

use tokio::{
    runtime::{Builder, Runtime},
    time::Duration,
};

use crate::{
    client::{Client, ClientError},
    mask::model::{FromPrimitives, IntoPrimitives, Model},
    participant::Task,
};

/// Generates a struct to hold the C equivalent of `&mut [N]` for a primitive data type `N`. Also,
/// implements a consuming iterator for the struct which is wrapped in a private submodule, because
/// safe traits and their safe methods can't be implemented as `unsafe` and this way we ensure that
/// the implementation is only ever used in an `unsafe fn`. The arguments `$prim_rust` and `$prim_c`
/// are the corresponding Rust and C primitive data types and `$doc0`, `$doc1` are a type links for
/// the documentation.
macro_rules! PrimModel {
    ($prim_rust:ty, $prim_c:ty, $doc0:expr, $doc1:expr $(,)?) => {
        paste::item! {
            #[derive(Clone, Copy)]
            #[repr(C)]
            #[doc = "A model of primitive data type"]
            #[doc = $doc0]
            #[doc = "represented as a mutable slice which can be accessed from C."]
            pub struct [<PrimitiveModel $prim_rust:upper>] {
                #[doc = "A raw mutable pointer to an array of primitive values"]
                #[doc = $doc1]
                #[doc = "."]
                pub ptr: *mut $prim_c,
                /// The length of that respective array.
                pub len: c_ulong,
            }
        }
    };
}

PrimModel! {f32, c_float, "[`f32`]", "`[f32]`"}
PrimModel! {f64, c_double, "[`f64`]", "`[f64]`"}
PrimModel! {i32, c_int, "[`i32`]", "`[i32]`"}
PrimModel! {i64, c_long, "[`i64`]", "`[i64]`"}

#[derive(Clone, Debug)]
/// A primitive model of data type `N` cached on the heap.
///
/// The pointer [`PrimitiveModelN`] returned from [`get_model_N()`] references this memory.
///
/// [`PrimitiveModelN`]: struct.PrimitiveModelF32.html
/// [`get_model_N()`]: fn.get_model_f32.html
pub(crate) enum PrimitiveModel {
    F32(Vec<f32>),
    F64(Vec<f64>),
    I32(Vec<i32>),
    I64(Vec<i64>),
}

/// A wrapper for a [`Client`] within a [`Runtime`].
///
/// This is returned from [`new_client()`]. See the [workflow] on how to use it.
///
/// [workflow]: index.html#workflow
pub struct FFIClient {
    client: Client,
    runtime: Runtime,
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Creates a new [`Client`] within a [`Runtime`].
///
/// Takes a `period` in seconds for which the [`Client`] will try to poll the coordinator for new
/// broadcasted FL round data. Specifies the `min_threads` and `max_threads` available to the
/// [`Runtime`].
///
/// # Errors
/// Returns a null pointer in case of invalid arguments or if the initialization of the runtime
/// or the client fails. The following must hold true for arguments to be valid:
/// - `period` > 0
/// - 0 < `min_thread` <= `max_thread` <= 32,768
///
/// # Safety
/// The method depends on the safety of the `callback` and on the consistent definition and layout
/// of its `input` across the FFI-boundary.
pub unsafe extern "C" fn new_client(
    period: c_ulong,
    min_threads: c_ulong,
    max_threads: c_ulong,
) -> *mut FFIClient {
    if period == 0 || min_threads == 0 || max_threads < min_threads || max_threads > 32_768 {
        return ptr::null_mut() as *mut FFIClient;
    }
    let runtime = if let Ok(runtime) = Builder::new()
        .threaded_scheduler()
        .core_threads(min_threads as usize)
        .max_threads(max_threads as usize)
        .thread_name("xain-fl-client-runtime-worker")
        .enable_all()
        .build()
    {
        runtime
    } else {
        return ptr::null_mut() as *mut FFIClient;
    };
    let client = if let Ok(client) = runtime.enter(move || Client::new(period)) {
        client
    } else {
        return ptr::null_mut() as *mut FFIClient;
    };
    Box::into_raw(Box::new(FFIClient { runtime, client }))
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Starts the [`Client`].
///
/// The [`Client`]'s tasks are executed in an asynchronous [`Runtime`].
///
/// Takes a `callback(state, code)` function pointer and a void pointer to the `state` of the
/// callback. The callback will be triggered when the client must be stopped because of a panic or
/// error or when the client terminates successfully. See the [errors] section for the error `code`
/// definitions.
///
/// # Errors
/// Ignores null pointer `client`s and triggers the callback immediately. Triggers the callback
/// with one of the following error codes in case of a [`Client`] panic or error:
/// - `-1`: client didn't start due to null pointer
/// - `0`: no error (only for clients with finite number of FL rounds)
/// - `1`: client panicked due to unexpected/unhandled error
/// - `2`: client stopped due to error [`ParticipantInitErr`]
/// - `3`: client stopped due to error [`ParticipantErr`]
/// - `4`: client stopped due to error [`DeserialiseErr`]
/// - `5`: client stopped due to error [`GeneralErr`]
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
///
/// If the callback is triggered because of a panicking client, it is the users responsibility
/// to not access possibly invalid state and to drop the client. If certain parts of the state
/// can be guaranteed to be valid, they may be read before dropping.
///
/// [errors]: #errors
/// [`ParticipantInitErr`]: ../../client/enum.ClientError.html#variant.ParticipantInitErr
/// [`ParticipantErr`]: ../../client/enum.ClientError.html#variant.ParticipantErr
/// [`DeserialiseErr`]: ../../client/enum.ClientError.html#variant.DeserialiseErr
/// [`GeneralErr`]: ../../client/enum.ClientError.html#variant.GeneralErr
pub unsafe extern "C" fn run_client(
    client: *mut FFIClient,
    callback: unsafe extern "C" fn(*mut c_void, c_int),
    state: *mut c_void,
) {
    if client.is_null() {
        return unsafe {
            // safe if the `callback` is sound
            callback(state, -1_i32 as c_int)
        };
    }
    let (runtime, client) = unsafe {
        // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
        (&(*client).runtime, &mut (*client).client)
    };
    let code = match panic::catch_unwind(unsafe {
        // even though `&mut Client` is `!UnwindSafe` we can assert this because the user will be
        // notified about a panic immediately to be able to safely act accordingly
        panic::AssertUnwindSafe(|| runtime.handle().block_on(client.start()))
    }) {
        Ok(Ok(_)) => 0_i32,
        Err(_) => 1_i32,
        Ok(Err(ClientError::ParticipantInitErr(_))) => 2_i32,
        Ok(Err(ClientError::ParticipantErr(_))) => 3_i32,
        Ok(Err(ClientError::DeserialiseErr(_))) => 4_i32,
        Ok(Err(ClientError::GeneralErr)) => 5_i32,
    } as c_int;
    unsafe {
        // safe if the `callback` is sound
        callback(state, code)
    };
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Stops and destroys a [`Client`] and frees its allocated memory.
///
/// Tries to gracefully stop the client for `timeout` seconds by blocking the current thread before
/// shutting it down forcefully (cf. the remarks about memory safety of the [runtime shutdown] in
/// case of elapsed timeout). Usually, no timeout (i.e. 0 seconds) suffices, but stopping might take
/// indefinitely if the client performs long blocking tasks.
///
/// # Errors
/// Ignores null pointer `client`s and returns immediately.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
///
/// [runtime shutdown]: https://docs.rs/tokio/0.2.21/tokio/runtime/struct.Runtime.html#method.shutdown_timeout
pub unsafe extern "C" fn drop_client(client: *mut FFIClient, timeout: c_ulong) {
    if !client.is_null() {
        let client = unsafe {
            // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
            Box::from_raw(client)
        };
        if timeout as usize != 0 {
            client
                .runtime
                .shutdown_timeout(Duration::from_secs(timeout));
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
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
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
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
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
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
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

/// Generates a function to create a zero-initialized primitive model. The arguments `$prim_rust`
/// and `$prim_c` are the corresponding Rust and C primitive data types and `$docN` are type links
/// for the documentation.
macro_rules! new_model {
    ($prim_rust:ty, $prim_c:ty, $doc0:expr, $doc1:expr $(,)?) => {
        paste::item! {
            #[allow(unused_unsafe)]
            #[no_mangle]
            #[doc = "Gets a mutable slice"]
            #[doc = $doc1]
            #[doc = "to a zero-initialized model of primitive data type"]
            #[doc = $doc0]
            #[doc = "of given length `len`.\n"]
            #[doc = "\n"]
            #[doc = "The new model gets cached, which overwrites any existing cached model. The"]
            #[doc = "cache and slice are valid as described in step 4 of the [workflow]. The"]
            #[doc = "cached model can be modified in place, for example for training.\n"]
            #[doc = "\n"]
            #[doc = "# Errors\n"]
            #[doc = "Ignores null pointer `client`s and returns a"]
            #[doc = $doc1]
            #[doc = "with null pointer and length zero immediately.\n"]
            #[doc = "\n"]
            #[doc = "Returns a"]
            #[doc = $doc1]
            #[doc = "with null pointer and length zero if the model is not representable in"]
            #[doc = "memory due to the given length `len`.\n"]
            #[doc = "\n"]
            #[doc = "# Safety\n"]
            #[doc = "The method dereferences from the raw pointer arguments. Therefore, the"]
            #[doc = "behavior of the method is undefined if the arguments don't point to valid"]
            #[doc = "objects.\n"]
            #[doc = "\n"]
            #[doc = "[workflow]: index.html#workflow"]
            pub unsafe extern "C" fn [<new_model_ $prim_rust>](
                client: *mut FFIClient,
                len: c_ulong,
            ) -> [<PrimitiveModel $prim_rust:upper>] {
                if client.is_null()
                    || len == 0
                    || len > (isize::MAX as usize / mem::size_of::<$prim_rust>()) as c_ulong
                {
                    let len = 0_u64 as c_ulong;
                    let ptr = ptr::null_mut() as *mut $prim_c;
                    [<PrimitiveModel $prim_rust:upper>] { ptr, len }
                } else {
                    let client = unsafe {
                        // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
                        &mut (*client).client
                    };
                    let mut primitive_model = vec![0 as $prim_rust; len as usize];
                    let ptr = primitive_model.as_mut_ptr() as *mut $prim_c;
                    client.cached_model = Some(PrimitiveModel::[<$prim_rust:upper>](primitive_model));
                    [<PrimitiveModel $prim_rust:upper>] { ptr, len }
                }
            }
        }
    };
}

new_model!(f32, c_float, "[`f32`]", "[`PrimitiveModelF32`]");
new_model!(f64, c_double, "[`f64`]", "[`PrimitiveModelF64`]");
new_model!(i32, c_int, "[`i32`]", "[`PrimitiveModelI32`]");
new_model!(i64, c_long, "[`i64`]", "[`PrimitiveModelI64`]");

/// Generates a function to get the global model converted to primitives. The arguments `$prim_rust`
/// and `$prim_c` are the corresponding Rust and C primitive data types and `$docN` are type links
/// for the documentation.
macro_rules! get_model {
    ($prim_rust:ty, $prim_c:ty, $doc0:expr, $doc1:expr $(,)?) => {
        paste::item! {
            #[allow(unused_unsafe)]
            #[no_mangle]
            #[doc = "Gets a mutable slice"]
            #[doc = $doc1]
            #[doc = "to the latest global model converted to primitive data type"]
            #[doc = $doc0]
            #[doc = ".\n\n"]
            #[doc = "The global model gets cached, which overwrites any existing cached model. The"]
            #[doc = "cache and slice are valid as described in step 4 of the [workflow]. The"]
            #[doc = "cached model can be modified in place, for example for training.\n"]
            #[doc = "\n"]
            #[doc = "# Errors\n"]
            #[doc = "Ignores null pointer `client`s and returns a"]
            #[doc = $doc1]
            #[doc = "with null pointer and length zero immediately.\n"]
            #[doc = "\n"]
            #[doc = "Returns a"]
            #[doc = $doc1]
            #[doc = "with null pointer and length zero if no global model is available or"]
            #[doc = "deserialization of the global model fails.\n"]
            #[doc = "\n"]
            #[doc = "Returns a"]
            #[doc = $doc1]
            #[doc = "with null pointer and length of the global model if the conversion of the"]
            #[doc = "global model into the primitive data type"]
            #[doc = $doc0]
            #[doc = "fails.\n"]
            #[doc = "\n"]
            #[doc = "# Safety\n"]
            #[doc = "The method dereferences from the raw pointer arguments. Therefore, the"]
            #[doc = "behavior of the method is undefined if the arguments don't point to valid"]
            #[doc = "objects.\n"]
            #[doc = "\n"]
            #[doc = "[workflow]: index.html#workflow"]
            pub unsafe extern "C" fn [<get_model_ $prim_rust>](
                client: *mut FFIClient,
            ) -> [<PrimitiveModel $prim_rust:upper>] {
                if client.is_null() {
                    let len = 0_u64 as c_ulong;
                    let ptr = ptr::null_mut() as *mut $prim_c;
                    return [<PrimitiveModel $prim_rust:upper>] { ptr, len };
                }
                let client = unsafe {
                    // safe if the raw pointer `client` comes from a valid allocation of a `FFIClient`
                    &mut (*client).client
                };

                // global model available
                if let Some(ref global_model) = client.global_model {
                    if let Some(PrimitiveModel::[<$prim_rust:upper>](ref mut cached_model)) = client.cached_model {
                        if !client.has_new_global_model_since_last_cache {
                            // global model is already cached as a primitive model
                            let ptr = cached_model.as_mut_ptr() as *mut $prim_c;
                            let len = cached_model.len() as c_ulong;
                            return [<PrimitiveModel $prim_rust:upper>] { ptr, len };
                        }
                    }

                    // deserialize and convert the global model to a primitive model and cache it
                    client.has_new_global_model_since_last_cache = false;
                    if let Ok(deserialized_model) = bincode::deserialize::<Model>(&global_model) {
                        // deserialization succeeded
                        let len = deserialized_model.len() as c_ulong;
                        if let Ok(mut primitive_model) = deserialized_model
                            .into_primitives()
                            .map(|res| res.map_err(|_| ()))
                            .collect::<Result<Vec<$prim_rust>, ()>>()
                        {
                            // conversion succeeded
                            let ptr = primitive_model.as_mut_ptr() as *mut $prim_c;
                            client.cached_model = Some(PrimitiveModel::[<$prim_rust:upper>](primitive_model));
                            return [<PrimitiveModel $prim_rust:upper>] { ptr, len };
                        } else {
                            // conversion failed
                            let ptr = ptr::null_mut() as *mut $prim_c;
                            client.cached_model = None;
                            return [<PrimitiveModel $prim_rust:upper>] { ptr, len };
                        }
                    }
                }

                // global model unavailable or deserialization failed
                let len = 0_u64 as c_ulong;
                let ptr = ptr::null_mut() as *mut $prim_c;
                client.cached_model = None;
                [<PrimitiveModel $prim_rust:upper>] { ptr, len }
            }
        }
    };
}

get_model!(f32, c_float, "[`f32`]", "[`PrimitiveModelF32`]");
get_model!(f64, c_double, "[`f64`]", "[`PrimitiveModelF64`]");
get_model!(i32, c_int, "[`i32`]", "[`PrimitiveModelI32`]");
get_model!(i64, c_long, "[`i64`]", "[`PrimitiveModelI64`]");

/// Generates a function to register the updated local model. The argument `$prim` is the
/// corresponding Rust primitive data type and `$docN` are type links for the documentation.
macro_rules! update_model {
    ($prim:ty, $doc0:expr, $doc1:expr, $doc2:expr $(,)?) => {
        paste::item! {
            #[allow(unused_unsafe)]
            #[no_mangle]
            #[doc = "Registers the updated local model of primitive data type"]
            #[doc = $doc0]
            #[doc = ".\n\n"]
            #[doc = "# Errors\n"]
            #[doc = "Ignores null pointer `client`s and returns immediately.\n"]
            #[doc = "\n"]
            #[doc = "Returns an error if the cached model is not of primitive data type"]
            #[doc = $doc0]
            #[doc = "or if there is no cached model at all.\n"]
            #[doc = "\n"]
            #[doc = "The error codes are as following:\n"]
            #[doc = "- `-1`: client didn't update due to null pointer\n"]
            #[doc = "- `0`: no error\n"]
            #[doc = "- `1`: client didn't update due missing cache\n"]
            #[doc = "- `2`: client didn't update due wrongly typed cache\n"]
            #[doc = "\n"]
            #[doc = "# Safety\n"]
            #[doc = "The method dereferences from the raw pointer arguments. Therefore, the"]
            #[doc = "behavior of the method is undefined if the arguments don't point to valid"]
            #[doc = "objects.\n"]
            #[doc = "\n"]
            #[doc = "The `model` points to memory which is either allocated by"]
            #[doc = $doc1]
            #[doc = "or by"]
            #[doc = $doc2]
            #[doc = "and then modified. Therefore, the behavior of the method is undefined if the"]
            #[doc = "memory was modified in an invalid way."]
            pub unsafe extern "C" fn [<update_model_ $prim>](
                client: *mut FFIClient,
            ) -> c_int {
                if client.is_null() {
                    return -1_i32 as c_int;
                }
                let client = unsafe {
                    // safe if the raw pointer `client` comes from a valid allocation of a `Client`
                    &mut (*client).client
                };
                if client.cached_model.is_none() {
                    return 1_i32 as c_int;
                }
                if let Some(PrimitiveModel::[<$prim:upper>](cached_model)) = client.cached_model.take() {
                    client.local_model = Some(Model::from_primitives_bounded(cached_model.into_iter()));
                    0_i32 as c_int
                } else {
                    2_i32 as c_int
                }
            }
        }
    };
}

update_model!(f32, "[`f32`]", "[`new_model_f32()`]", "[`get_model_f32()`]");
update_model!(f64, "[`f64`]", "[`new_model_f32()`]", "[`get_model_f64()`]");
update_model!(i32, "[`i32`]", "[`new_model_f32()`]", "[`get_model_i32()`]");
update_model!(i64, "[`i64`]", "[`new_model_f32()`]", "[`get_model_i64()`]");

#[allow(unused_unsafe)]
#[no_mangle]
/// Destroys a [`Client`]'s cached primitive model and frees its allocated memory.
///
/// It is not necessary to call this function if [`drop_client()`] is called anyways.
///
/// # Errors
/// Ignores null pointer `client`s and returns immediately.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn drop_model(client: *mut FFIClient) {
    if !client.is_null() {
        let client = unsafe {
            // safe if the raw pointer `client` comes from a valid allocation of a `Client`
            &mut (*client).client
        };
        client.cached_model.take();
    }
}

#[cfg(test)]
mod tests {
    use std::{iter::FromIterator, sync::Arc};

    use num::rational::Ratio;

    use super::*;
    use crate::service::SerializedGlobalModel;

    #[test]
    fn test_new_client() {
        let client = unsafe { new_client(10, 1, 4) };
        assert!(!client.is_null());
        unsafe { drop_client(client, 0) };
    }

    #[test]
    fn test_run_client() {
        // define state and callback
        #[repr(C)]
        struct State {
            client_crashed: bool,
            error_code: c_int,
        };
        #[allow(unused_unsafe)]
        #[no_mangle]
        unsafe extern "C" fn callback(state: *mut c_void, code: c_int) {
            if code != 0 {
                let state = unsafe { &mut *(state as *mut State) };
                state.client_crashed = true;
                state.error_code = code;
            }
        }

        // check that the client panics when running it without a service
        let client = unsafe { new_client(10, 1, 4) };
        let mut state = State {
            client_crashed: false,
            error_code: 0_i32 as c_int,
        };
        unsafe { run_client(client, callback, &mut state as *mut _ as *mut c_void) };
        assert!(state.client_crashed);
        assert_eq!(state.error_code, 1);
        unsafe { drop_client(client, 0) };
    }

    // define dummy model of length `len` where all values are set to `val`
    fn dummy_model(val: f64, len: usize) -> (Model, SerializedGlobalModel) {
        let model = Model::from_iter(vec![Ratio::from_float(val).unwrap(); len].into_iter());
        let serialized_model = Arc::new(bincode::serialize(&model).unwrap());
        (model, serialized_model)
    }

    macro_rules! test_new_model {
        ($prim:ty) => {
            paste::item! {
                #[allow(unused_unsafe)]
                #[test]
                fn [<test_new_model_ $prim>]() {
                    let client = unsafe { new_client(10, 1, 4) };

                    // check that the new model is cached
                    let (model, _) = dummy_model(0., 10);
                    let prim_model = unsafe { [<new_model_ $prim>](client, 10 as c_ulong) };
                    if let Some(PrimitiveModel::[<$prim:upper>](ref cached_model)) = unsafe { &mut *client }.client.cached_model {
                        assert_eq!(prim_model.ptr as *const _, cached_model.as_ptr());
                        assert_eq!(prim_model.len as usize, cached_model.len());
                        assert_eq!(model, Model::from_primitives_bounded(cached_model.iter().cloned()));
                    } else {
                        panic!();
                    }
                    unsafe { drop_client(client, 0) };
                }
            }
        };
    }

    test_new_model!(f32);
    test_new_model!(f64);
    test_new_model!(i32);
    test_new_model!(i64);

    macro_rules! test_get_model {
        ($prim:ty) => {
            paste::item! {
                #[allow(unused_unsafe)]
                #[test]
                fn [<test_get_model_ $prim>]() {
                    let client = unsafe { new_client(10, 1, 4) };

                    // check that the primitive model is null if the global model is unavailable
                    assert!(unsafe { &*client }.client.global_model.is_none());
                    let prim_model = unsafe { [<get_model_ $prim>](client) };
                    assert!(unsafe { &*client }.client.cached_model.is_none());
                    assert!(prim_model.ptr.is_null());
                    assert_eq!(prim_model.len, 0);

                    // check that the primitive model points to the cached model if the global model is available
                    let (model, serialized_model) = dummy_model(0., 10);
                    unsafe { &mut *client }.client.global_model = Some(serialized_model);
                    let prim_model = unsafe { [<get_model_ $prim>](client) };
                    if let Some(PrimitiveModel::[<$prim:upper>](ref cached_model)) = unsafe { &mut *client }.client.cached_model {
                        assert_eq!(prim_model.ptr as *const _, cached_model.as_ptr());
                        assert_eq!(prim_model.len as usize, cached_model.len());
                        assert_eq!(model, Model::from_primitives_bounded(cached_model.iter().cloned()));
                    } else {
                        panic!();
                    }
                    unsafe { drop_client(client, 0) };
                }
            }
        };
    }

    test_get_model!(f32);
    test_get_model!(f64);
    test_get_model!(i32);
    test_get_model!(i64);

    macro_rules! test_update_model {
        ($prim:ty) => {
            paste::item! {
                #[test]
                fn [<test_update_model_ $prim>]() {
                    let client = unsafe { new_client(10, 1, 4) };
                    let (model, serialized_model) = dummy_model(0., 10);
                    unsafe { &mut *client }.client.global_model = Some(serialized_model);
                    let prim_model = unsafe { [<get_model_ $prim>](client) };

                    // check that the local model is updated from the cached model
                    if let Some(PrimitiveModel::[<$prim:upper>](ref cached_model)) = unsafe { &mut *client }.client.cached_model {
                        assert_eq!(prim_model.ptr as *const _, cached_model.as_ptr());
                        assert_eq!(prim_model.len as usize, cached_model.len());
                    } else {
                        panic!();
                    }
                    assert!(unsafe {  &*client }.client.local_model.is_none());
                    assert_eq!(unsafe { [<update_model_ $prim>](client) }, 0);
                    assert!(unsafe { &mut *client }.client.cached_model.is_none());
                    if let Some(ref local_model) = unsafe { &*client }.client.local_model {
                        assert_eq!(&model, local_model);
                    }
                    unsafe { drop_client(client, 0) };
                }
            }
        };
    }

    test_update_model!(f32);
    test_update_model!(f64);
    test_update_model!(i32);
    test_update_model!(i64);
}
