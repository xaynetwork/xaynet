use std::{
    convert::TryFrom,
    os::raw::{c_int, c_uchar, c_uint, c_void},
    ptr,
    slice,
};

use ffi_support::{ByteBuffer, FfiStr};
use xaynet_core::mask::{DataType, FromPrimitives, IntoPrimitives, Model};

use super::{
    LocalModelConfig,
    ERR_GLOBALMODEL_CONVERT,
    ERR_GLOBALMODEL_DATATYPE,
    ERR_GLOBALMODEL_IO,
    ERR_GLOBALMODEL_LEN,
    ERR_NULLPTR,
    ERR_SETMODEL_DATATYPE,
    ERR_SETMODEL_MODEL,
    GLOBALMODEL_NONE,
    OK,
};
use crate::{into_primitives, Participant, Settings, Task};

mod pv {
    use super::Participant;
    ffi_support::define_box_destructor!(Participant, _xaynet_ffi_participant_destroy);
}

/// Destroy the participant created by [`xaynet_ffi_participant_new()`] or
/// [`xaynet_ffi_participant_restore()`].
///
/// # Return value
///
/// - [`OK`] on success
/// - [`ERR_NULLPTR`] if `participant` is NULL
///
/// # Safety
///
/// 1. When calling this method, you have to ensure that *either* the pointer is NULL
///    *or* all of the following is true:
///    - The pointer must be properly [aligned].
///    - It must be "dereferencable" in the sense defined in the [`::std::ptr`] module
///      documentation.
/// 2. After destroying the `Participant`, the pointer becomes invalid and must not be
///    used.
/// 3. This function should only be called on a pointer that has been created by
///    [`xaynet_ffi_participant_new()`] or [`xaynet_ffi_participant_restore()`]
///
/// [`::std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_participant_destroy(participant: *mut Participant) -> c_int {
    if participant.is_null() {
        return ERR_NULLPTR;
    }
    pv::_xaynet_ffi_participant_destroy(participant);
    OK
}

/// The participant is not taking part in the sum or update task
pub const PARTICIPANT_TASK_NONE: c_int = 1;
/// The participant is not taking part in the sum task
pub const PARTICIPANT_TASK_SUM: c_int = 1 << 1;
/// The participant is not taking part in the update task
pub const PARTICIPANT_TASK_UPDATE: c_int = 1 << 2;
/// The participant is expected to set the model it trained
pub const PARTICIPANT_SHOULD_SET_MODEL: c_int = 1 << 3;
/// The participant is expected to set the model it trained
pub const PARTICIPANT_MADE_PROGRESS: c_int = 1 << 4;
/// A new global model is available
pub const PARTICIPANT_NEW_GLOBALMODEL: c_int = 1 << 5;

/// Instantiate a new participant with the given settings. The participant must be
/// destroyed with [`xaynet_ffi_participant_destroy`].
///
/// # Return value
///
/// - a NULL pointer if `settings` is NULL or if the participant creation failed
/// - a valid pointer to a [`Participant`] otherwise
///
/// # Safety
///
/// When calling this method, you have to ensure that *either* the pointer is NULL *or*
/// all of the following is true:
///
/// - The pointer must be properly [aligned].
/// - It must be "dereferencable" in the sense defined in the [`::std::ptr`] module
///   documentation.
///
/// After destroying the participant with [`xaynet_ffi_participant_destroy`] becomes
/// invalid and must not be used.
///
/// [`::std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_participant_new(settings: *const Settings) -> *mut Participant {
    let settings = match unsafe { settings.as_ref() } {
        Some(settings) => settings.clone(),
        None => return std::ptr::null_mut(),
    };

    match Participant::new(settings) {
        Ok(participant) => Box::into_raw(Box::new(participant)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Drive the participant internal state machine. Every tick, the state machine
/// attempts to perform a small work unit.
///
/// # Return value
///
/// - [`ERR_NULLPTR`] is `participant` is NULL
/// - a bitflag otherwise, with the following flags:
///   - [`PARTICIPANT_MADE_PROGRESS`]: if set, this flag indicates that the participant
///     internal state machine was able to make some progress, and that the participant
///     state changed. This information can be used as an indication for saving the
///     participant state for instance. If the flag is not set, the state machine was
///     not able to make progress. There are many potential causes for this, including:
///       - the participant is not taking part to the current training round and is just
///         waiting for a new one to start
///       - the Xaynet coordinator is not reachable or has not published some
///         information the participant is waiting for
///       - the state machine is waiting for the model to be set (see
///         [`xaynet_ffi_participant_set_model()`])
///   - [`PARTICIPANT_TASK_NONE`], [`PARTICIPANT_TASK_SUM`] and
///     [`PARTICIPANT_TASK_UPDATE`]: these flags are mutually exclusive, and indicate
///     which task the participant has been selected for, for the current round. If
///     [`PARTICIPANT_TASK_NONE`] is set, then the participant will just wait for a new
///     round to start. If [`PARTICIPANT_TASK_UPDATE`] is set, then the participant has
///     been selected to update the global model, and should prepare to provide a new
///     model once the [`PARTICIPANT_SHOULD_SET_MODEL`] flag is set.
///   - [`PARTICIPANT_SHOULD_SET_MODEL`]: if set, then the participant should set its
///     model, by calling [`xaynet_ffi_participant_set_model()`]
///   - [`PARTICIPANT_NEW_GLOBALMODEL`]: if set, the participant can fetch the new global
///     model, by calling [`xaynet_ffi_participant_global_model()`]
///
/// # Safety
///
/// When calling this method, you have to ensure that *either* the pointer is NULL *or*
/// all of the following is true:
///
/// - The pointer must be properly [aligned].
/// - It must be "dereferencable" in the sense defined in the [`::std::ptr`] module
///   documentation.
///
/// After destroying the participant with [`xaynet_ffi_participant_destroy`] becomes
/// invalid and must not be used.
///
/// [`::std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_participant_tick(participant: *mut Participant) -> c_int {
    let participant = match unsafe { participant.as_mut() } {
        Some(participant) => participant,
        None => return ERR_NULLPTR,
    };

    participant.tick();

    let mut flags: c_int = 0;
    match participant.task() {
        Task::None => flags |= PARTICIPANT_TASK_NONE,
        Task::Sum => flags |= PARTICIPANT_TASK_SUM,
        Task::Update => flags |= PARTICIPANT_TASK_UPDATE,
    };
    if participant.should_set_model() {
        flags |= PARTICIPANT_SHOULD_SET_MODEL;
    }
    if participant.made_progress() {
        flags |= PARTICIPANT_MADE_PROGRESS;
    }
    if participant.new_global_model() {
        flags |= PARTICIPANT_NEW_GLOBALMODEL;
    }
    flags
}

/// Serialize the participant state and return a buffer that contains the serialized
/// participant.
///
/// # Safety
///
/// 1. When calling this method, you have to ensure that *either* the pointer is NULL
///    *or* all of the following is true:
///    - The pointer must be properly [aligned].
///    - It must be "dereferencable" in the sense defined in the [`::std::ptr`] module
///      documentation.
/// 2. the `ByteBuffer` created by this function must be destroyed with
///    [`xaynet_ffi_participant_destroy`]. Attempting to free the memory from the other
///    side of the FFI is UB.
/// 3. This function destroys the participant. Therefore, **the pointer becomes invalid
///    and must not be used anymore**. Instead, a new participant should be created,
///    either with [`xaynet_ffi_participant_new()`] or
///    [`xaynet_ffi_participant_restore()`]
///
/// [`::std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
///
/// # Example
///
/// To save the participant into a file:
///
/// ```c
///  const ByteBuffer *save_buf = xaynet_ffi_participant_save(participant);
///  assert(save_buf);
///
///  char *path = "./participant.bin";
///  FILE *f = fopen(path, "w");
///  fwrite(save_buf->data, 1, save_buf->len, f);
///  fclose(f);
/// ```
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_participant_save(
    participant: *mut Participant,
) -> *const ByteBuffer {
    let participant: Participant = match unsafe { participant.as_mut() } {
        Some(ptr) => unsafe { *Box::from_raw(ptr) },
        None => return std::ptr::null(),
    };

    Box::into_raw(Box::new(ByteBuffer::from_vec(participant.save())))
}

/// Restore the participant from a buffer that contained its serialized state.
///
/// # Return value
///
/// - a NULL pointer on failure
/// - a pointer to the restored participant on success
///
/// # Safety
///
/// When calling this method, you have to ensure that *either* the pointers are NULL
/// *or* all of the following is true:
/// - The pointers must be properly [aligned].
/// - They must be "dereferencable" in the sense defined in the [`::std::ptr`] module
///   documentation.
///
/// [`::std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
///
/// # Example
///
/// To restore a participant from a file:
///
/// ```c
/// f = fopen("./participant.bin", "r");
/// fseek(f, 0L, SEEK_END);
/// int fsize = ftell(f);
/// fseek(f, 0L, SEEK_SET);
/// ByteBuffer buf = {
///     .len = fsize,
///     .data = (uint8_t *)malloc(fsize),
/// };
/// int n_read = fread(buf.data, 1, fsize, f);
/// assert(n_read == fsize);
/// fclose(f);
/// Participant *restored =
///     xaynet_ffi_participant_restore("http://localhost:8081", &buf);
/// free(buf.data);
/// ```
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_participant_restore(
    url: FfiStr,
    buffer: *const ByteBuffer,
) -> *mut Participant {
    let url = match url.as_opt_str() {
        Some(url) => url,
        None => return ptr::null_mut(),
    };

    let buffer: &ByteBuffer = match unsafe { buffer.as_ref() } {
        Some(ptr) => ptr,
        None => return ptr::null_mut(),
    };

    if let Ok(participant) = Participant::restore(buffer.as_slice(), url) {
        Box::into_raw(Box::new(participant))
    } else {
        ptr::null_mut()
    }
}

/// Set the participant's model. Usually this should be called when the value returned
/// by [`xaynet_ffi_participant_tick()`] contains the [`PARTICIPANT_SHOULD_SET_MODEL`]
/// flag, but it can be called anytime. The model just won't be sent to the coordinator
/// until it's time.
///
/// - `buffer` should be a pointer to a buffer that contains the model
/// - `data_type` specifies the type of the model weights (see [`DataType`]). The C header
///   file generated by this crate provides an enum corresponding to the parameters: `DataType`.
/// - `len` is the number of weights the model has
///
/// # Return value
///
/// - [`OK`] if the model is set successfully
/// - [`ERR_NULLPTR`] if `participant` is NULL
/// - [`ERR_SETMODEL_DATATYPE`] if the datatype is invalid
/// - [`ERR_SETMODEL_MODEL`] if the model is invalid
///
/// # Safety
///
/// 1. When calling this method, you have to ensure that *either* the pointer is NULL
///    *or* all of the following is true:
///    - The pointer must be properly [aligned].
///    - It must be "dereferencable" in the sense defined in the [`::std::ptr`] module
///      documentation.
/// 2. If `len` or `data_type` do not match the model in `buffer`, this method will
///    result in a buffer over-read.
///
/// [`::std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_participant_set_model(
    participant: *mut Participant,
    buffer: *const c_void,
    data_type: c_uchar,
    len: c_uint,
) -> c_int {
    let participant = match unsafe { participant.as_mut() } {
        Some(participant) => participant,
        None => return ERR_NULLPTR,
    };

    if buffer.is_null() {
        return ERR_NULLPTR;
    }

    let data_type = match DataType::try_from(data_type) {
        Ok(data_type) => data_type,
        Err(_) => return ERR_SETMODEL_DATATYPE,
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
        participant.set_model(m);
        OK
    } else {
        ERR_SETMODEL_MODEL
    }
}

/// Return the latest global model from the coordinator.
///
/// - `buffer` is the array in which the global model should be copied.
/// - `data_type` specifies the type of the model weights (see [`DataType`]). The C header
///   file generated by this crate provides an enum corresponding to the parameters: `DataType`.
/// - `len` is the number of weights the model has
///
/// # Return Value
///
/// - [`OK`] if the model is set successfully
/// - [`ERR_NULLPTR`] if `participant` or the `buffer` is NULL
/// - [`GLOBALMODEL_NONE`] if no model exists
/// - [`ERR_GLOBALMODEL_IO`] if the communication with the coordinator failed
/// - [`ERR_GLOBALMODEL_DATATYPE`] if the datatype is invalid
/// - [`ERR_GLOBALMODEL_LEN`] if the length of the buffer does not match the length of the model
/// - [`ERR_GLOBALMODEL_CONVERT`] if the conversion of the model failed
///
/// # Note
///
///   It is **not** guaranteed, that the model configuration returned by
///   [`xaynet_ffi_participant_local_model_config`] corresponds to the configuration of
///   the global model. This means that the global model can have a different length / data type
///   than it is defined in model configuration. That both model configurations are the same is
///   only guaranteed if the model config **never** changes on the coordinator side.
///
/// # Safety
///
/// 1. When calling this method, you have to ensure that *either* the pointer is NULL
///    *or* all of the following is true:
///    - The pointer must be properly [aligned].
///    - It must be "dereferencable" in the sense defined in the [`::std::ptr`] module
///      documentation.
/// 2. If `len` or `data_type` do not match the model in `buffer`, this method will
///    result in a buffer over-read.
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_participant_global_model(
    participant: *mut Participant,
    buffer: *mut c_void,
    data_type: c_uchar,
    len: c_uint,
) -> c_int {
    let participant = match unsafe { participant.as_mut() } {
        Some(participant) => participant,
        None => return ERR_NULLPTR,
    };

    if buffer.is_null() {
        return ERR_NULLPTR;
    }

    let global_model = match participant.global_model() {
        Ok(Some(model)) => model,
        Ok(None) => return GLOBALMODEL_NONE,
        Err(_) => return ERR_GLOBALMODEL_IO,
    };

    let data_type = match DataType::try_from(data_type) {
        Ok(data_type) => data_type,
        Err(_) => return ERR_GLOBALMODEL_DATATYPE,
    };

    let len = len as usize;
    if len != global_model.len() {
        return ERR_GLOBALMODEL_LEN;
    }

    match data_type {
        DataType::F32 => into_primitives!(global_model, buffer, f32, len),
        DataType::F64 => into_primitives!(global_model, buffer, f64, len),
        DataType::I32 => into_primitives!(global_model, buffer, i32, len),
        DataType::I64 => into_primitives!(global_model, buffer, i64, len),
    }
}

#[macro_export]
macro_rules! into_primitives {
    ($global_model:expr, $buffer:expr, $data_type:ty, $len:expr) => {{
        if let Ok(global_model) = $global_model
            .into_primitives()
            .collect::<Result<Vec<$data_type>, _>>()
        {
            let buffer = unsafe { slice::from_raw_parts_mut($buffer as *mut $data_type, $len) };
            buffer.copy_from_slice(global_model.as_slice());
            OK
        } else {
            ERR_GLOBALMODEL_CONVERT
        }
    }};
}

/// Return the local model configuration of the model that is expected in the
/// [`xaynet_ffi_participant_set_model`] function.
///
/// # Safety
///
/// 1. When calling this method, you have to ensure that *either* the pointer is NULL
///    *or* all of the following is true:
///    - The pointer must be properly [aligned].
///    - It must be "dereferencable" in the sense defined in the [`::std::ptr`] module
///      documentation.
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_participant_local_model_config(
    participant: *const Participant,
) -> *mut LocalModelConfig {
    let participant = match unsafe { participant.as_ref() } {
        Some(ptr) => ptr,
        None => return std::ptr::null_mut(),
    };

    Box::into_raw(Box::new(participant.local_model_config().into()))
}
