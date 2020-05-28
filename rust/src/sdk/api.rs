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

/// Generate a struct to hold the C equivalent of `&[N]` for a primitive data type `N` and
/// implements a consuming iterator for the struct. The arguments `$rust` and `$c` are the
/// corresponding Rust and C primitive data types.
macro_rules! Model {
    ($rust:ty, $c:ty $(,)?) => {
        paste::item! {
            #[derive(Clone, Copy)]
            #[repr(C)]
            /// A model of primitive data type represented as a C slice. It holds a raw pointer to
            /// the array of [`[<$rust>]`] values and its length.
            pub struct [<Model $rust:upper>] {
                ptr: *mut $c,
                len: c_ulong,
            }

            /// An iterator that moves out of a primitive [`[<Model $rust:upper>]`].
            pub struct [<IntoIter $rust:upper>] {
                model: [<Model $rust:upper>],
                count: isize,
            }

            impl IntoIterator for [<Model $rust:upper>] {
                type Item = $rust;
                type IntoIter = [<IntoIter $rust:upper>];

                /// Creates an [`[<IntoIter $rust:upper>]`] iterator from a primitive
                /// [`[<Model $rust:upper>]`].
                fn into_iter(self) -> Self::IntoIter {
                    Self::IntoIter {
                        model: self,
                        count: -1_isize,
                    }
                }
            }

            impl Iterator for [<IntoIter $rust:upper>] {
                type Item = $rust;

                /// Advances the iterator and returns the next [<$rust>] value. Returns `None` when
                /// the iteration is finished.
                ///
                /// # Safety
                /// The iterator iterates over an array by dereferencing from raw pointer and is
                /// therefore inherently unsafe, even though this can't be indicated in the function
                /// signature of the trait's method.
                ///
                /// # Panic
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

Model! {f32, c_float}
Model! {f64, c_double}
Model! {i32, c_int}
Model! {i64, c_long}

/// TODO: this is a mock, replace by sth like the `Client` from #397 or a wrapper around that.
///
/// The pointer to the cached primitive model, which gets allocated in `get_model()` and is stored
/// in `model_N`, is valid across the FFI-boundary until one of the following events happen:
/// - The model memory is freed via a call to `free_model()`.
/// - The model is updated via a call to `update_model()`.
/// - The round ends. (TODO: implement this point when a new round is observed)
pub struct Client {
    participant: Participant,

    // counting starts from 1, 0 means not seen yet
    current_round: u32,
    checked_round: u32,

    model_f32: Option<Box<Vec<f32>>>,
    model_f64: Option<Box<Vec<f64>>>,
    model_i32: Option<Box<Vec<i32>>>,
    model_i64: Option<Box<Vec<i64>>>,
}

#[no_mangle]
/// Check if the next round has started.
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
/// Check if the current role of the participant is `update`.
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

/// Generate a method to get the global model converted to primitives. The arguments `$rust` and
/// `$c` are the corresponding Rust and C primitive data types.
macro_rules! get_model {
    ($rust:ty, $c:ty $(,)?) => {
        paste::item! {
            #[no_mangle]
            /// Get the latest global [`[<Model $rust:upper>]`], which is valid until the current
            /// round ends. The model can be modified in place, for example for training, to avoid
            /// needless cloning.
            ///
            /// # Errors
            /// - Returns a [`[<Model $rust:upper>]`] with `null` pointer and `len` zero if no
            ///   global model is available.
            /// - Returns a [`[<Model $rust:upper>]`] with `null` pointer and `len` of the global
            ///   model if type casting fails.
            ///
            /// # Safety
            /// The method dereferences from the raw pointer arguments. Therefore, the behavior of
            /// the method is undefined if the arguments don't point to valid objects.
            pub unsafe extern "C" fn [<get_model_ $rust>](
                client: *mut Client,
            ) -> [<Model $rust:upper>] {
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
                    if client.[<model_ $rust>].is_none() {
                        // cache the primitive model if needed
                        client.[<model_ $rust>] = model
                            .into_primitives()
                            .map(|res| res.map_err(|_| ()))
                            .collect::<Result<Vec<$rust>, ()>>()
                            .map_or(None, |vec| Some(Box::new(vec)));
                    }

                    if let Some(ref mut model) = client.[<model_ $rust>] {
                        // conversion succeeded
                        let ptr = model.as_mut_ptr();
                        [<Model $rust:upper>] { ptr, len }
                    } else {
                        // conversion failed
                        let ptr = ptr::null_mut();
                        [<Model $rust:upper>] { ptr, len }
                    }

                } else {
                    // global model unavailable
                    let ptr = ptr::null_mut();
                    let len = 0_u64 as c_ulong;
                    [<Model $rust:upper>] { ptr, len }
                }
            }
        }
    };
}

get_model!(f32, c_float);
get_model!(f64, c_double);
get_model!(i32, c_int);
get_model!(i64, c_long);

/// Generate a method to register the updated local model. The argument `$rust` is the corresponding
/// Rust primitive data type.
macro_rules! update_model {
    ($rust:ty $(,)?) => {
        paste::item! {
            #[no_mangle]
            /// Register the updated local model.
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
                model: [<Model $rust:upper>],
            ) {
                if !client.is_null()
                    && !model.ptr.is_null()
                    && model.len != 0
                    && model.len <= usize::MAX as c_ulong
                    && model.len as usize * mem::size_of::<$rust>() <= usize::MAX
                {
                    let client = &mut *client;
                    if let Some(cached) = client.[<model_ $rust>].take() {
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
