//! A C-API to communicate model updates between a PET protocol participant and an application.
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

use std::{
    cmp::Ordering,
    iter::{FromIterator, IntoIterator, Iterator},
    mem,
    os::raw::{c_double, c_float, c_int, c_long, c_ulong},
    ptr,
};

use num::{bigint::BigInt, rational::Ratio, traits::Zero};

use crate::{
    mask::model::{FromPrimitives, IntoPrimitives, Model},
    participant::{Participant, Task},
};

/// Generates a struct to hold the C equivalent of `&mut [N]` for a primitive data type `N` and
/// implements a consuming iterator for the struct. The arguments `$rust` and `$c` are the
/// corresponding Rust and C primitive data types.
macro_rules! PrimModel {
    ($rust:ty, $c:ty $(,)?) => {
        paste::item! {
            #[derive(Clone, Copy)]
            #[repr(C)]
            /// A model of primitive data type represented as a mutable slice which can be accessed
            /// from C. It holds a raw pointer `ptr` to the array of primitive values and its length
            /// `len`.
            pub struct [<PrimitiveModel $rust:upper>] {
                ptr: *mut $c,
                len: c_ulong,
            }

            /// An iterator that moves out of a primitive model.
            pub struct [<IntoIter $rust:upper>] {
                model: [<PrimitiveModel $rust:upper>],
                count: isize,
            }

            impl IntoIterator for [<PrimitiveModel $rust:upper>] {
                type Item = $rust;
                type IntoIter = [<IntoIter $rust:upper>];

                /// Creates an iterator from a primitive model.
                fn into_iter(self) -> Self::IntoIter {
                    Self::IntoIter {
                        model: self,
                        count: -1_isize,
                    }
                }
            }

            impl Iterator for [<IntoIter $rust:upper>] {
                type Item = $rust;

                /// Advances the iterator and returns the next primitive value. Returns `None` when
                /// the iteration is finished.
                ///
                /// # Safety
                /// The iterator iterates over an array by dereferencing from a raw pointer and is
                /// therefore inherently unsafe, even though this can't be indicated in the function
                /// signature of the trait's method.
                ///
                /// # Panics
                /// The iterator panics if safety checks indicate undefined behavior.
                fn next(&mut self) -> Option<Self::Item> {
                    if ((self.count + 1) as c_ulong) < self.model.len {
                        if self.count < isize::MAX
                            && (self.model.ptr as isize)
                                .checked_add((self.count + 2) * mem::size_of::<$rust>() as isize)
                                .is_some()
                        {
                            self.count += 1;
                        } else {
                            // TODO: add error handling
                            panic!("iterating further results in undefined behavior");
                        }
                        unsafe {
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

PrimModel! {f32, c_float}
PrimModel! {f64, c_double}
PrimModel! {i32, c_int}
PrimModel! {i64, c_long}

/// A cached primitive model stored on the heap. The mutable slice returned from `get_model_N()`
/// points to this.
pub enum PrimitiveModel {
    F32(Vec<f32>),
    F64(Vec<f64>),
    I32(Vec<i32>),
    I64(Vec<i64>),
}

/// TODO: this is a mock, replace by sth like the `Client` from #397 or a wrapper around that.
///
/// The pointer to the cached [`PrimitiveModel`], which gets allocated in `get_model_N()` and is
/// stored in `model`, is valid across the FFI-boundary until one of the following events happen:
/// - The model memory is freed via a call to `free_model()`.
/// - The client memory is freed via a call to `drop_client()`.
/// - The model is updated via a call to `update_model()`.
/// - The round ends. (TODO: implement this point when a new round is observed)
pub struct Client {
    participant: Participant,

    // counting starts from 1, 0 means not seen yet
    current_round: u32,
    checked_round: u32,

    // cached primitive model
    model: Option<PrimitiveModel>,
}

#[no_mangle]
/// Creates a new [`Client`].
pub extern "C" fn new_client() -> *mut Client {
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
    Box::into_raw(Box::new(client))
}

#[no_mangle]
/// Destroys a [`Client`] and frees its allocated memory.
///
/// # Safety
/// The method dereferences from the raw pointer arguments. Therefore, the behavior of
/// the method is undefined if the arguments don't point to valid objects.
pub unsafe extern "C" fn drop_client(client: *mut Client) {
    if !client.is_null() {
        Box::from_raw(client);
    }
}

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
    let client = &mut *client;
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
    let client = &mut *client;
    client.participant.task == Task::Update
}

/// Generates a method to get the global model converted to primitives. The arguments `$rust` and
/// `$c` are the corresponding Rust and C primitive data types.
macro_rules! get_model {
    ($rust:ty, $c:ty $(,)?) => {
        paste::item! {
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
            pub unsafe extern "C" fn [<get_model_ $rust>](
                client: *mut Client,
            ) -> [<PrimitiveModel $rust:upper>] {
                if client.is_null() {
                    // TODO: add error handling
                    panic!("invalid client");
                }
                let client = &mut *client;

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
                            .collect::<Result<Vec<$rust>, ()>>()
                            .map_or(None, |vec| Some(PrimitiveModel::[<$rust:upper>](vec)));
                    }

                    if let Some(PrimitiveModel::[<$rust:upper>](ref mut model)) = client.model {
                        // conversion succeeded
                        let ptr = model.as_mut_ptr();
                        [<PrimitiveModel $rust:upper>] { ptr, len }
                    } else {
                        // conversion failed
                        let ptr = ptr::null_mut();
                        [<PrimitiveModel $rust:upper>] { ptr, len }
                    }

                } else {
                    // global model unavailable
                    let ptr = ptr::null_mut();
                    let len = 0_u64 as c_ulong;
                    [<PrimitiveModel $rust:upper>] { ptr, len }
                }
            }
        }
    };
}

get_model!(f32, c_float);
get_model!(f64, c_double);
get_model!(i32, c_int);
get_model!(i64, c_long);

/// Generates a method to register the updated local model. The argument `$rust` is the
/// corresponding Rust primitive data type.
macro_rules! update_model {
    ($rust:ty $(,)?) => {
        paste::item! {
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
            pub unsafe extern "C" fn [<update_model_ $rust>](
                client: *mut Client,
                model: [<PrimitiveModel $rust:upper>],
            ) {
                if !client.is_null()
                    && !model.ptr.is_null()
                    && model.len != 0
                    && model.len <= usize::MAX as c_ulong
                    && model.len as usize * mem::size_of::<$rust>() <= usize::MAX
                {
                    let client = &mut *client;
                    if let Some(PrimitiveModel::[<$rust:upper>](cached)) = client.model.take() {
                        // cached model was updated
                        if ptr::eq(model.ptr, cached.as_ptr())
                            && model.len as usize == cached.len()
                        {
                            // TODO: use the model when the client sends the update message
                            let _local_model = Model::from_primitives_bounded(cached.into_iter());
                        }
                    } else {
                        // other model was updated
                        // TODO: use the model when the client sends the update message
                        let _local_model = Model::from_primitives_bounded(model.into_iter());
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
    let client = &mut *client;
    client.model.take();
}
