//! A C-API to communicate model updates between a PET protocol participant and an application.
//!
//! # Safety
//! Many functions of this module are marked as `unsafe` to explicitly announce the possible
//! unsafety of the function body as well as the return value to the caller. At the same time,
//! each `unsafe fn` uses `unsafe` blocks to precisely pinpoint the sources of unsafety for
//! reviewers (redundancy warnings will be fixed by [#69173](https://github.com/rust-lang/rust/issues/69173)).
//!
//! **Note, that the `unsafe` code has not been externally audited yet!**
//!
//! # Workflow
//! 1. Initialize a client with `new_client()`, which will take care of the participant's PET
//!    protocol work as well as the networking with the coordinator.
//! 2. Optionally request status information:
//!     - `is_next_round()` indicates if another round of the PET protocol has started.
//!     - `is_update_participant()` indicates if this participant is eligible to submit a
//!       trained local model.
//! 3. Get the latest global model with `get_model_N()`, where `N` is the primitive data type.
//!    Currently, `f32`, `f64`, `i32` and `i64` are supported. The function returns a mutable
//!    slice to the primitive model, whereas the primitive model itself is cached within the
//!    client.
//! 4. Register a trained local model with `update_model_N()`, which takes either a slice to
//!    the cached primitive model or a slice to a foreign memory location.
//! 5. Destroy the client with `drop_client()`.
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

use std::{
    cmp::Ordering,
    iter::{FromIterator, IntoIterator, Iterator},
    mem,
    os::raw::{c_double, c_float, c_int, c_long, c_uint, c_ulong, c_void},
    ptr,
};

use num::{bigint::BigInt, rational::Ratio, traits::Zero};

use crate::{
    mask::model::{FromPrimitives, IntoPrimitives, Model},
    participant::{Participant, Task},
};

/// Generates a struct to hold the C equivalent of `&mut [N]` for a primitive data type `N`. Also,
/// implements a consuming iterator for the struct which is wrapped in a private submodule, because
/// safe traits and their safe methods can't be implemented as `unsafe` and this way we ensure that
/// the implementation is only ever used in an `unsafe fn`. The arguments `$prim_rust` and `$prim_c`
/// are the corresponding Rust and C primitive data types.
macro_rules! PrimModel {
    ($prim_rust:ty, $prim_c:ty $(,)?) => {
        paste::item! {
            #[derive(Clone, Copy)]
            #[repr(C)]
            /// A model of primitive data type represented as a mutable slice which can be accessed
            /// from C.
            pub struct [<PrimitiveModel $prim_rust:upper>] {
                /// A raw mutable pointer to an array of primitive values.
                pub ptr: *mut $prim_c,
                /// The length of that respective array.
                pub len: c_ulong,
            }
        }
    };
}

PrimModel! {f32, c_float}
PrimModel! {f64, c_double}
PrimModel! {i32, c_int}
PrimModel! {i64, c_long}

/// The iterators are wrapped in a private submodule, because safe traits and their safe methods
/// can't be implemented as `unsafe` and this way we can ensure that the implementation is only
/// ever used in an `unsafe fn`.
mod iter {
    use std::{mem, os::raw::c_ulong};

    /// Generates a consuming iterator for the primitive model.  The argument
    /// `$prim` is the corresponding Rust primitive data type.
    macro_rules! PrimIter {
        ($prim:ty $(,)?) => {
            paste::item! {
                use super::[<PrimitiveModel $prim:upper>];

                #[doc(hidden)]
                /// An iterator that moves out of a primitive model.
                pub struct [<IntoIter $prim:upper>] {
                    model: [<PrimitiveModel $prim:upper>],
                    count: isize,
                }

                #[doc(hidden)]
                impl IntoIterator for [<PrimitiveModel $prim:upper>] {
                    type Item = $prim;
                    type IntoIter = [<IntoIter $prim:upper>];

                    /// Creates an iterator from a primitive model.
                    fn into_iter(self) -> Self::IntoIter {
                        Self::IntoIter {
                            model: self,
                            count: -1_isize,
                        }
                    }
                }

                #[doc(hidden)]
                impl Iterator for [<IntoIter $prim:upper>] {
                    type Item = $prim;

                    /// Advances the iterator and returns the next primitive value. Returns `None`
                    /// when the iteration is finished.
                    ///
                    /// # Safety
                    /// The iterator iterates over an array by dereferencing from a raw pointer and
                    /// is therefore inherently unsafe, even though this can't be indicated in the
                    /// function signature of the trait's method. Therefore, this method must only
                    /// ever be used in an unsafe function.
                    ///
                    /// # Panics
                    /// The iterator panics if safety checks indicate undefined behavior.
                    fn next(&mut self) -> Option<Self::Item> {
                        if ((self.count + 1) as c_ulong) < self.model.len {
                            if self.count < isize::MAX
                                && (self.model.ptr as isize)
                                    .checked_add((self.count + 2) * mem::size_of::<$prim>() as isize)
                                    .is_some()
                            {
                                self.count += 1;
                            } else {
                                // TODO: add error handling
                                panic!("iterating further results in undefined behavior");
                            }
                            unsafe {
                                // safe if the pointer `ptr` comes from a valid allocation of a
                                // `Vec<$prim>`, whereas the safety of the offset arithmetics is
                                // ensured by the checks above
                                Some(*self.model.ptr.offset(self.count))
                            }
                        } else {
                            None
                        }
                    }
                }
            }
        };
    }

    PrimIter!(f32);
    PrimIter!(f64);
    PrimIter!(i32);
    PrimIter!(i64);
}

#[derive(Clone, Debug)]
/// A primitive model of data type `N` cached on the heap. The pointer `PrimitiveModelN` returned
/// from `get_model_N()` references this memory.
enum PrimitiveModel {
    F32(Vec<f32>),
    F64(Vec<f64>),
    I32(Vec<i32>),
    I64(Vec<i64>),
}

/// TODO: this is a mock, replace by sth like the `Client` from #397 or a wrapper around that.
///
/// The pointer `PrimitiveModelN` to the cached [`PrimitiveModel`] of primitive data type `N`,
/// which gets allocated and returned in `get_model_N()`, is valid across the FFI-boundary until
/// one of the following events happen:
/// - The [`PrimitiveModel`] memory is freed via a call to [`drop_model()`].
/// - The [`Client`] memory is freed via a call to [`drop_client()`].
/// - The model is updated via a call to `update_model_N()`.
/// - The round ends. (TODO: implement this point when a new round is observed)
pub struct Client {
    participant: Participant,

    // counting starts from 1, 0 means not seen yet
    current_round: u32,
    checked_round: u32,

    // cached primitive model
    model: Option<PrimitiveModel>,
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Creates a new [`Client`]. Takes a `callback` function pointer and a void pointer to the `state`
/// of the callback. The underlying function is defined over void pointers referencing (anonymous)
/// structs for the `state` and `input` arguments of the callback.
///
/// # Safety
/// The method depends on the safety of the `callback` and on the consistent definition and layout
/// of its `input` across the FFI-boundary.
pub unsafe extern "C" fn new_client(
    callback: unsafe extern "C" fn(*mut c_void, *const c_void),
    state: *mut c_void,
) -> *mut Client {
    let client = Client {
        participant: if let Ok(participant) = Participant::new() {
            participant
        } else {
            // TODO: add error handling
            panic!("participant initialization failed")
        },
        current_round: 0,
        checked_round: 0,
        model: None,
    };

    #[repr(C)]
    struct Input {
        _current_round: c_uint,
        _checked_round: c_uint,
        _participant_initialized: bool,
        _model_cached: bool,
    }
    let input = &Input {
        _current_round: client.current_round as c_uint,
        _checked_round: client.checked_round as c_uint,
        _participant_initialized: true,
        _model_cached: client.model.is_some(),
    } as *const Input as *const c_void;
    unsafe {
        // safe if the `callback` is safe and the same definition and layout is used for `Input`
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
    // TODO: increment the `current_round` if the client sees a new coordinator pk
    // as a result of `client.handle.get_round_parameters().await`
    match client.checked_round.cmp(&client.current_round) {
        // new round since the last check
        Ordering::Less => {
            client.checked_round = client.current_round;
            true
        }
        // still same round since the last check
        Ordering::Equal => false,
        // should only ever happen if the client is reused for another FL use case
        Ordering::Greater => panic!("restart the participant for a new FL use case"),
    }
}

#[allow(unused_unsafe)]
#[no_mangle]
/// Checks if the current role of the participant is [`Task::Update`].
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
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
/// and `$prim_c` are the corresponding Rust and C primitive data types.
macro_rules! get_model {
    ($prim_rust:ty, $prim_c:ty $(,)?) => {
        paste::item! {
            #[allow(unused_unsafe)]
            #[no_mangle]
            /// Gets the latest global model converted as a primitive model, which is valid until
            /// the current round ends. The model can be modified in place, for example for
            /// training.
            ///
            /// # Errors
            /// - Returns a primitive model with `null` pointer and `len` zero if no global model is
            ///   available.
            /// - Returns a primitive model with `null` pointer and `len` of the global model if
            ///   type casting fails.
            ///
            /// # Safety
            /// The method dereferences from the raw pointer arguments. Therefore, the behavior of
            /// the method is undefined if the arguments don't point to valid objects.
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

                // TODO: this is a mock, get the model when the client retrieves the round
                // parameters as a result of `client.handle.get_round_parameters().await`
                let global_model = Some(Model::from_iter(
                    vec![Ratio::<BigInt>::zero(); 10].into_iter(),
                ));

                if let Some(model) = global_model {
                    // global model available
                    let len = model.len() as c_ulong;
                    if client.model.is_none() {
                        // cache the primitive model if needed
                        client.model = model
                            .into_primitives()
                            .map(|res| res.map_err(|_| ()))
                            .collect::<Result<Vec<$prim_rust>, ()>>()
                            .map_or(None, |vec| Some(PrimitiveModel::[<$prim_rust:upper>](vec)));
                    }

                    if let Some(PrimitiveModel::[<$prim_rust:upper>](ref mut model)) = client.model {
                        // conversion succeeded
                        let ptr = model.as_mut_ptr() as *mut $prim_c;
                        [<PrimitiveModel $prim_rust:upper>] { ptr, len }
                    } else {
                        // conversion failed
                        let ptr = ptr::null_mut() as *mut $prim_c;
                        [<PrimitiveModel $prim_rust:upper>] { ptr, len }
                    }

                } else {
                    // global model unavailable
                    let ptr = ptr::null_mut() as *mut $prim_c;
                    let len = 0_u64 as c_ulong;
                    [<PrimitiveModel $prim_rust:upper>] { ptr, len }
                }
            }
        }
    };
}

get_model!(f32, c_float);
get_model!(f64, c_double);
get_model!(i32, c_int);
get_model!(i64, c_long);

/// Generates a method to register the updated local model. The argument `$prim` is the
/// corresponding Rust primitive data type.
macro_rules! update_model {
    ($prim:ty $(,)?) => {
        paste::item! {
            #[allow(unused_unsafe)]
            #[no_mangle]
            /// Registers the updated local model.
            ///
            /// # Safety
            /// The method dereferences from the raw pointer arguments. Therefore, the behavior of
            /// the method is undefined if the arguments don't point to valid objects.
            ///
            /// The `model` points to memory which is either allocated by `get_model()` and then
            /// modified or which isn't allocated by `get_model()`. Therefore, the behavior of the
            /// method is undefined if any of the [slice safety conditions](https://doc.rust-lang.org/std/slice/fn.from_raw_parts.html#safety)
            /// are violated.
            pub unsafe extern "C" fn [<update_model_ $prim>](
                client: *mut Client,
                model: [<PrimitiveModel $prim:upper>],
            ) {
                if !client.is_null()
                    && !model.ptr.is_null()
                    && model.len != 0
                    && model.len <= (usize::MAX / mem::size_of::<$prim>()) as c_ulong
                {
                    let client = unsafe {
                        // safe if the raw pointer `client` comes from a valid allocation of a `Client`
                        &mut *client
                    };
                    if let Some(PrimitiveModel::[<$prim:upper>](cached)) = client.model.take() {
                        // cached model was updated
                        if ptr::eq(model.ptr as *const $prim, cached.as_ptr())
                            && model.len as usize == cached.len()
                        {
                            // TODO: use the model when the client sends the update message
                            let _local_model = Model::from_primitives_bounded(cached.into_iter());
                        }
                    } else {
                        // other model was updated
                        // TODO: use the model when the client sends the update message
                        let _local_model = Model::from_primitives_bounded(unsafe {
                            // safe if the slice `model` comes from a valid allocation of a `Vec<$prim>`
                            model.into_iter()
                        });
                    }
                } else {
                    // TODO: add error handling
                    panic!("invalid primitive model");
                }
            }
        }
    };
}

update_model!(f32);
update_model!(f64);
update_model!(i32);
update_model!(i64);

#[allow(unused_unsafe)]
#[no_mangle]
/// Destroys a cached [`PrimitiveModel`] and frees its allocated memory.
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
    client.model.take();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_client() {
        #[repr(C)]
        struct State {
            participant_initialized_without_caching: bool,
        }

        #[allow(unused_unsafe)]
        unsafe extern "C" fn callback(state: *mut c_void, input: *const c_void) {
            #[repr(C)]
            struct Input {
                current_round: c_uint,
                checked_round: c_uint,
                participant_initialized: bool,
                model_cached: bool,
            }

            let state = unsafe { &mut *(state as *mut State) };
            let input = unsafe { &*(input as *const Input) };
            state.participant_initialized_without_caching = input.current_round == 0
                && input.checked_round == 0
                && input.participant_initialized
                && !input.model_cached;
        }

        let mut state = State {
            participant_initialized_without_caching: false,
        };
        let client = unsafe { new_client(callback, &mut state as *mut State as *mut c_void) };
        unsafe { drop_client(client) };
        assert!(state.participant_initialized_without_caching);
    }

    unsafe extern "C" fn dummy_callback(_state: *mut c_void, _input: *const c_void) {}

    macro_rules! test_get_model {
        ($prim:ty) => {
            paste::item! {
                #[allow(unused_unsafe)]
                #[test]
                fn [<test_get_model_ $prim>]() {
                    let client = unsafe { new_client(dummy_callback, ptr::null_mut() as *mut c_void) };
                    let model = unsafe { [<get_model_ $prim>](client) };
                    if let Some(PrimitiveModel::[<$prim:upper>](ref cached)) = unsafe { &mut *client }.model {
                        assert_eq!(
                            Model::from_primitives_bounded(unsafe { model.into_iter() }),
                            Model::from_primitives_bounded(cached.clone().into_iter()),
                        );
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
                #[test]
                fn [<test_update_cached_model_ $prim>]() {
                    let client = unsafe { new_client(dummy_callback, ptr::null_mut() as *mut c_void) };
                    let model = unsafe { [<get_model_ $prim>](client) };
                    if let Some(PrimitiveModel::[<$prim:upper>](ref cached)) = unsafe { &mut *client }.model {
                        assert_eq!(cached.as_ptr(), model.ptr as *const $prim);
                        assert_eq!(cached.len(), model.len as usize);
                    } else {
                        panic!();
                    }
                    unsafe { [<update_model_ $prim>](client, model) };
                    assert!(unsafe { &mut *client }.model.is_none());
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
                #[test]
                fn [<test_update_noncached_model_ $prim>]() {
                    let client = unsafe { new_client(dummy_callback, ptr::null_mut() as *mut c_void) };
                    let model = unsafe { [<get_model_ $prim>](client) };
                    let mut vec = Model::from_iter(vec![Ratio::<BigInt>::zero(); model.len as usize].into_iter())
                        .into_primitives()
                        .map(|res| res.map_err(|_| ()))
                        .collect::<Result<Vec<$prim>, ()>>()
                        .unwrap();
                    let model = [<PrimitiveModel $prim:upper>] {
                        ptr: vec.as_mut_ptr(),
                        len: vec.len() as c_ulong,
                    };
                    if let Some(PrimitiveModel::[<$prim:upper>](ref cached)) = unsafe { &mut *client }.model {
                        assert_ne!(cached.as_ptr(), model.ptr as *const $prim);
                        assert_eq!(cached.len(), model.len as usize);
                    } else {
                        panic!();
                    }
                    unsafe { [<update_model_ $prim>](client, model) };
                    assert!(unsafe { &mut *client }.model.is_none());
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
