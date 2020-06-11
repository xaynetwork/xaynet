//! A C-API to communicate model updates between a PET protocol participant and an application.
//!
//! # Workflow
//! 1. Initialize a [`Client`] with [`new_client()`], which will take care of the [`Participant`]'s
//!    PET protocol work as well as the networking with the [`Coordinator`].
//! 2. Optionally request status information:
//!     - [`is_next_round()`] indicates if another round of the PET protocol has started.
//!     - [`has_next_model()`] indicates if another global model is available.
//!     - [`is_update_participant()`] indicates if this [`Participant`] is eligible to submit a
//!       trained local model.
//! 3. Get the latest global model with [`get_model_N()`], where `N` is the primitive data type.
//!    Currently, [`f32`], [`f64`], [`i32`] and [`i64`] are supported. The function returns a
//!    mutable slice [`PrimitiveModelN`] to the primitive model, whereas the primitive model itself
//!    is cached within the [`Client`]. The slice is valid across the FFI-boundary until one of the
//!    following events happen:
//!     - [`update_model_N()`] and [`drop_model()`] each free the memory to which
//!       [`PrimitiveModelN`] points to.
//!     - [`drop_client()`] frees the memory of the [`Client`].
//!     - [`get_model_N()`] frees and reallocates the memory to which [`PrimitiveModelN`] points to
//!       if a new global model is available since the last call to [`get_model_N()`].
//! 4. Register a trained local model with [`update_model_N()`], which takes either a slice to
//!    the cached primitive model or a slice to a foreign memory location.
//! 5. Destroy the [`Client`] with [`drop_client()`].
//!
//! # Callbacks
//! Some functions of this module provide stateful callbacks, where the
//! `callback: unsafe extern "C" fn(*mut c_void, *const c_void)` is defined over void pointers
//! referencing structs for the `state` and `input` of the callback. The fields of these structs
//! are not constrained due to their void pointer nature. The `input` can be defined (anonymously)
//! on each side of the FFI-boundary, but it must still have the same layout on both sides,
//! otherwise accessing it will result in undefined behavior.
//!
//! **Note, that callbacks are at an experimental stage and the generalized `input` might be
//! replaced for concrete input types for certain functions in the future.**
//!
//! # Safety
//! Many functions of this module are marked as `unsafe` to explicitly announce the possible
//! unsafety of the function body as well as the return value to the caller. At the same time,
//! each `unsafe fn` uses `unsafe` blocks to precisely pinpoint the sources of unsafety for
//! reviewers (redundancy warnings will be fixed by [#69173](https://github.com/rust-lang/rust/issues/69173)).
//!
//! **Note, that the `unsafe` code has not been externally audited yet!**
//!
//! [`Coordinator`]: ../../coordinator/struct.Coordinator.html
//! [`get_model_N()`]: fn.get_model_f32.html
//! [`update_model_N()`]: fn.update_model_f32.html
//! [`Participant`]: ../../participant/struct.Participant.html
//! [`PrimitiveModelN`]: struct.PrimitiveModelF32.html

use std::{
    iter::{IntoIterator, Iterator},
    mem,
    os::raw::{c_double, c_float, c_int, c_long, c_ulong, c_void},
    ptr,
    slice,
};

use crate::{
    client::Client,
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
            #[doc = "A model of primitive data type [`"]
            #[doc = $doc0]
            #[doc = "`] represented as a mutable slice which can be accessed from C."]
            pub struct [<PrimitiveModel $prim_rust:upper>] {
                #[doc = "A raw mutable pointer to an array of primitive values `"]
                #[doc = $doc1]
                #[doc = "`."]
                pub ptr: *mut $prim_c,
                /// The length of that respective array.
                pub len: c_ulong,
            }
        }
    };
}

PrimModel! {f32, c_float, "f32", "[f32]"}
PrimModel! {f64, c_double, "f64", "[f64]"}
PrimModel! {i32, c_int, "i32", "[i32]"}
PrimModel! {i64, c_long, "i64", "[i64]"}

#[derive(Clone, Debug)]
/// A primitive model of data type `N` cached on the heap. The pointer `PrimitiveModelN` returned
/// from `get_model_N()` references this memory.
pub(crate) enum PrimitiveModel {
    F32(Vec<f32>),
    F64(Vec<f64>),
    I32(Vec<i32>),
    I64(Vec<i64>),
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Creates a new [`Client`].
///
/// Takes a `period` in seconds for which the [`Client`] will try to poll the coordinator for new
/// broadcasted FL round data.
///
/// Takes a `callback` function pointer and a void pointer to the `state` of the callback. The
/// underlying function is defined over void pointers referencing (anonymous) structs for the
/// `state` and `input` arguments of the callback.
///
/// # Safety
/// The method depends on the safety of the `callback` and on the consistent definition and layout
/// of its `input` across the FFI-boundary.
pub unsafe extern "C" fn new_client(
    period: c_ulong,
    callback: unsafe extern "C" fn(*mut c_void, *const c_void),
    state: *mut c_void,
) -> *mut Client {
    if period == 0 {
        // TODO: add error handling
        panic!("polling period must be positive")
    }
    let client = if let Ok(client) = Client::new(period as u64) {
        client
    } else {
        // TODO: add error handling
        panic!("participant initialization failed")
    };

    // TODO: actually start the client, requires tokio running

    #[repr(C)]
    struct Input {
        _round_started: bool,
        _participant_initialized: bool,
        _model_cached: bool,
    }
    let input = &Input {
        _round_started: client.has_new_coord_pk_since_last_check,
        _participant_initialized: true,
        _model_cached: client.cached_model.is_some(),
    } as *const _ as *const c_void;
    unsafe {
        // safe if the `callback` is sound and the same definition and layout is used for `Input`
        // across the FFI-boundary by the caller
        callback(state, input)
    };

    Box::into_raw(Box::new(client))
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Destroys a [`Client`] and frees its allocated memory.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn drop_client(client: *mut Client) {
    if !client.is_null() {
        unsafe {
            // safe if the raw pointer `client` comes from a valid allocation of a `Client`
            Box::from_raw(client);
        }
    }
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Checks if the next round has started.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn is_next_round(client: *mut Client) -> bool {
    if client.is_null() {
        // TODO: add error handling
        panic!("invalid client");
    }
    let client = unsafe {
        // safe if the raw pointer `client` comes from a valid allocation of a `Client`
        &mut *client
    };
    mem::replace(&mut client.has_new_coord_pk_since_last_check, false)
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Checks if the next global model is available.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn has_next_model(client: *mut Client) -> bool {
    if client.is_null() {
        // TODO: add error handling
        panic!("invalid client");
    }
    let client = unsafe {
        // safe if the raw pointer `client` comes from a valid allocation of a `Client`
        &mut *client
    };
    mem::replace(&mut client.has_new_global_model_since_last_check, false)
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Checks if the current role of the participant is [`Update`].
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
///
/// [`Update`]: ../../participant/enum.Task.html#variant.Update
pub unsafe extern "C" fn is_update_participant(client: *mut Client) -> bool {
    if client.is_null() {
        // TODO: add error handling
        panic!("invalid client");
    }
    let client = unsafe {
        // safe if the raw pointer `client` comes from a valid allocation of a `Client`
        &*client
    };
    client.participant.task == Task::Update
}

/// Generates a method to get the global model converted to primitives. The arguments `$prim_rust`
/// and `$prim_c` are the corresponding Rust and C primitive data types and `$doc` is a type link
/// for the documentation.
macro_rules! get_model {
    ($prim_rust:ty, $prim_c:ty, $doc:expr $(,)?) => {
        paste::item! {
            #[allow(unused_unsafe)]
            #[no_mangle]
            #[doc = "Gets a mutable slice"]
            #[doc = $doc]
            #[doc = "to the latest global model.\n"]
            #[doc = "\n"]
            #[doc = "The global model gets converted and cached as a primitive model, which is"]
            #[doc = "valid until the current round ends. The cached model can be modified in"]
            #[doc = "place, for example for training.\n"]
            #[doc = "\n"]
            #[doc = "# Errors\n"]
            #[doc = "- Returns a"]
            #[doc = $doc]
            #[doc = "with `null` pointer and `len` zero if no global model is available or"]
            #[doc = "deserialization of the global model fails.\n"]
            #[doc = "- Returns a"]
            #[doc = $doc]
            #[doc = "with `null` pointer and `len` of the global model if the conversion of the"]
            #[doc = "global model into the chosen primitive data type fails.\n"]
            #[doc = "\n"]
            #[doc = "# Safety\n"]
            #[doc = "The method dereferences from the raw pointer arguments. Therefore, the"]
            #[doc = "behavior of the method is undefined if the arguments don't point to valid"]
            #[doc = "objects."]
            pub unsafe extern "C" fn [<get_model_ $prim_rust>](
                client: *mut Client,
            ) -> [<PrimitiveModel $prim_rust:upper>] {
                if client.is_null() {
                    // TODO: add error handling
                    panic!("invalid client");
                }
                let client = unsafe {
                    // safe if the raw pointer `client` comes from a valid allocation of a `Client`
                    &mut *client
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

get_model!(f32, c_float, "[`PrimitiveModelF32`]");
get_model!(f64, c_double, "[`PrimitiveModelF64`]");
get_model!(i32, c_int, "[`PrimitiveModelI32`]");
get_model!(i64, c_long, "[`PrimitiveModelI64`]");

/// Generates a method to register the updated local model. The argument `$prim` is the
/// corresponding Rust primitive data type and `$doc` is a type link for the documentation.
macro_rules! update_model {
    ($prim:ty, $doc:expr $(,)?) => {
        paste::item! {
            #[allow(unused_unsafe)]
            #[no_mangle]
            #[doc = "Registers the updated local model.\n"]
            #[doc = "\n"]
            #[doc = "A `model` which doesn't point to memory allocated by"]
            #[doc = $doc]
            #[doc = "requires additional copying while beeing iterated over.\n"]
            #[doc = "\n"]
            #[doc = "# Safety\n"]
            #[doc = "The method dereferences from the raw pointer arguments. Therefore, the"]
            #[doc = "behavior of the method is undefined if the arguments don't point to valid"]
            #[doc = "objects.\n"]
            #[doc = "\n"]
            #[doc = "The `model` points to memory which is either allocated by"]
            #[doc = $doc]
            #[doc = "and then modified or which isn't allocated by"]
            #[doc = $doc]
            #[doc = ". Therefore, the behavior of the method is undefined if any of the"]
            #[doc = "[slice safety conditions](https://doc.rust-lang.org/std/slice/fn.from_raw_parts.html#safety)"]
            #[doc = " are violated for `model`."]
            pub unsafe extern "C" fn [<update_model_ $prim>](
                client: *mut Client,
                model: [<PrimitiveModel $prim:upper>],
            ) {
                if client.is_null()
                    || model.ptr.is_null()
                    || model.len == 0
                    || model.len > (isize::MAX as usize / mem::size_of::<$prim>()) as c_ulong
                {
                    // TODO: add error handling
                    panic!("invalid primitive model");
                }

                let client = unsafe {
                    // safe if the raw pointer `client` comes from a valid allocation of a `Client`
                    &mut *client
                };
                if let Some(PrimitiveModel::[<$prim:upper>](cached_model)) = client.cached_model.take() {
                    if ptr::eq(model.ptr as *const _, cached_model.as_ptr())
                        && model.len as usize == cached_model.len()
                    {
                        // cached model was updated
                        client.local_model = Some(Model::from_primitives_bounded(cached_model.into_iter()));
                    }
                } else {
                    // other model was updated
                    client.local_model = Some(Model::from_primitives_bounded(unsafe {
                        // safe if `model` fulfills the slice safety conditions
                        slice::from_raw_parts(model.ptr as *const _, model.len as usize)
                    }.into_iter().copied()));
                }
            }
        }
    };
}

update_model!(f32, "[`get_model_f32()`]");
update_model!(f64, "[`get_model_f64()`]");
update_model!(i32, "[`get_model_i32()`]");
update_model!(i64, "[`get_model_i64()`]");

#[allow(unused_unsafe)]
#[no_mangle]
/// Destroys a [`Client`]'s cached primitive model and frees its allocated memory.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn drop_model(client: *mut Client) {
    if client.is_null() {
        // TODO: add error handling
        panic!("invalid client");
    }
    let client = unsafe {
        // safe if the raw pointer `client` comes from a valid allocation of a `Client`
        &mut *client
    };
    client.cached_model.take();
}

#[cfg(test)]
mod tests {
    use std::{iter::FromIterator, sync::Arc};

    use num::rational::Ratio;

    use super::*;
    use crate::service::SerializedGlobalModel;

    #[tokio::test]
    async fn test_new_client() {
        #[repr(C)]
        struct State {
            participant_initialized_without_caching: bool,
        }

        #[allow(unused_unsafe)]
        #[no_mangle]
        unsafe extern "C" fn callback(state: *mut c_void, input: *const c_void) {
            #[repr(C)]
            struct Input {
                round_started: bool,
                participant_initialized: bool,
                model_cached: bool,
            }

            let state = unsafe { &mut *(state as *mut State) };
            let input = unsafe { &*(input as *const Input) };
            state.participant_initialized_without_caching =
                !input.round_started && input.participant_initialized && !input.model_cached;
        }

        let mut state = State {
            participant_initialized_without_caching: false,
        };
        let client = unsafe { new_client(10, callback, &mut state as *mut _ as *mut c_void) };
        unsafe { drop_client(client) };
        assert!(state.participant_initialized_without_caching);
    }

    unsafe extern "C" fn dummy_callback(_state: *mut c_void, _input: *const c_void) {}

    fn dummy_model(val: f64, len: usize) -> (Model, SerializedGlobalModel) {
        let model = Model::from_iter(vec![Ratio::from_float(val).unwrap(); len].into_iter());
        let serialized_model = Arc::new(bincode::serialize(&model).unwrap());
        (model, serialized_model)
    }

    macro_rules! test_get_model {
        ($prim:ty) => {
            paste::item! {
                #[allow(unused_unsafe)]
                #[tokio::test]
                async fn [<test_get_model_ $prim>]() {
                    let client = unsafe { new_client(10, dummy_callback, ptr::null_mut() as *mut c_void) };

                    // check that the primitive model is null if the global model is unavailable
                    assert!(unsafe { &*client }.global_model.is_none());
                    let prim_model = unsafe { [<get_model_ $prim>](client) };
                    assert!(unsafe { &*client }.cached_model.is_none());
                    assert!(prim_model.ptr.is_null());
                    assert_eq!(prim_model.len, 0);

                    // check that the primitive model points to the cached model if the global model is available
                    let (model, serialized_model) = dummy_model(0., 10);
                    unsafe { &mut *client }.global_model = Some(serialized_model);
                    let prim_model = unsafe { [<get_model_ $prim>](client) };
                    if let Some(PrimitiveModel::[<$prim:upper>](ref cached_model)) = unsafe { &mut *client }.cached_model {
                        assert_eq!(prim_model.ptr as *const _, cached_model.as_ptr());
                        assert_eq!(prim_model.len as usize, cached_model.len());
                        assert_eq!(model, Model::from_primitives_bounded(cached_model.iter().cloned()));
                    } else {
                        panic!();
                    }
                    unsafe { drop_client(client) };
                }
            }
        };
    }

    test_get_model!(f32);
    test_get_model!(f64);
    test_get_model!(i32);
    test_get_model!(i64);

    macro_rules! test_update_cached_model {
        ($prim:ty) => {
            paste::item! {
                #[tokio::test]
                async fn [<test_update_cached_model_ $prim>]() {
                    let client = unsafe { new_client(10, dummy_callback, ptr::null_mut() as *mut c_void) };
                    let (model, serialized_model) = dummy_model(0., 10);
                    unsafe { &mut *client }.global_model = Some(serialized_model);
                    let prim_model = unsafe { [<get_model_ $prim>](client) };

                    // check that the local model is updated from the cached model
                    if let Some(PrimitiveModel::[<$prim:upper>](ref cached_model)) = unsafe { &mut *client }.cached_model {
                        assert_eq!(prim_model.ptr as *const _, cached_model.as_ptr());
                        assert_eq!(prim_model.len as usize, cached_model.len());
                    } else {
                        panic!();
                    }
                    assert!(unsafe {  &*client }.local_model.is_none());
                    unsafe { [<update_model_ $prim>](client, prim_model) };
                    assert!(unsafe { &mut *client }.cached_model.is_none());
                    if let Some(ref local_model) = unsafe { &*client }.local_model {
                        assert_eq!(&model, local_model);
                    }
                    unsafe { drop_client(client) };
                }
            }
        };
    }

    test_update_cached_model!(f32);
    test_update_cached_model!(f64);
    test_update_cached_model!(i32);
    test_update_cached_model!(i64);

    macro_rules! test_update_noncached_model {
        ($prim:ty) => {
            paste::item! {
                #[tokio::test]
                async fn [<test_update_noncached_model_ $prim>]() {
                    let client = unsafe { new_client(10, dummy_callback, ptr::null_mut() as *mut c_void) };
                    let (model, serialized_model) = dummy_model(0., 10);
                    unsafe { &mut *client }.global_model = Some(serialized_model);
                    let prim_model = unsafe { [<get_model_ $prim>](client) };
                    let (other_model, _) = dummy_model(0.5, 10);
                    let mut vec = other_model.clone()
                        .into_primitives()
                        .map(|res| res.map_err(|_| ()))
                        .collect::<Result<Vec<$prim>, ()>>()
                        .unwrap();
                    let other_prim_model = [<PrimitiveModel $prim:upper>] {
                        ptr: vec.as_mut_ptr(),
                        len: vec.len() as c_ulong,
                    };

                    // check that the local model is updated from the other noncached model
                    if let Some(PrimitiveModel::[<$prim:upper>](ref cached_model)) = unsafe { &mut *client }.cached_model {
                        assert_eq!(prim_model.ptr as *const _, cached_model.as_ptr());
                        assert_eq!(prim_model.len as usize, cached_model.len());
                        assert_ne!(other_prim_model.ptr as *const _, cached_model.as_ptr());
                        assert_eq!(other_prim_model.len as usize, cached_model.len());
                    } else {
                        panic!();
                    }
                    assert!(unsafe {  &*client }.local_model.is_none());
                    unsafe { [<update_model_ $prim>](client, other_prim_model) };
                    assert!(unsafe { &mut *client }.cached_model.is_none());
                    if let Some(ref local_model) = unsafe { &*client }.local_model {
                        assert_ne!(&model, local_model);
                        assert_eq!(&other_model, local_model);
                    }
                    unsafe { drop_client(client) };
                }
            }
        };
    }

    test_update_noncached_model!(f32);
    test_update_noncached_model!(f64);
    test_update_noncached_model!(i32);
    test_update_noncached_model!(i64);
}
