use crate::ffi::{ERR_NULLPTR, OK};
use std::os::raw::c_int;
use xaynet_core::mask::DataType;

mod pv {
    use super::LocalModelConfig;
    ffi_support::define_box_destructor!(LocalModelConfig, _xaynet_ffi_local_model_config_destroy);
}

/// Destroy the model configuration created by [`xaynet_ffi_participant_local_model_config()`].
///
/// # Return value
///
/// - [`OK`] on success
/// - [`ERR_NULLPTR`] if `local_model_config` is NULL
///
/// # Safety
///
/// 1. When calling this method, you have to ensure that *either* the pointer is NULL
///    *or* all of the following is true:
///    - The pointer must be properly [aligned].
///    - It must be "dereferencable" in the sense defined in the [`::std::ptr`] module
///      documentation.
/// 2. After destroying the `LocalModelConfig`, the pointer becomes invalid and must not be
///    used.
/// 3. This function should only be called on a pointer that has been created by
///    [`xaynet_ffi_participant_local_model_config()`].
///
/// [`::std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_local_model_config_destroy(
    local_model_config: *mut LocalModelConfig,
) -> c_int {
    if local_model_config.is_null() {
        return ERR_NULLPTR;
    }
    pv::_xaynet_ffi_local_model_config_destroy(local_model_config);
    OK
}

#[repr(C)]
/// The model configuration of the model that is expected in [`xaynet_ffi_participant_set_model`].
pub struct LocalModelConfig {
    /// The expected data type of the model.
    pub data_type: ModelDataType,
    /// the expected length of the model.
    pub len: u64,
}

impl Into<LocalModelConfig> for xaynet_sdk::LocalModelConfig {
    fn into(self) -> LocalModelConfig {
        LocalModelConfig {
            data_type: self.data_type.into(),
            len: self.len as u64,
        }
    }
}

#[repr(u8)]
/// The original primitive data type of the numerical values to be masked.
pub enum ModelDataType {
    /// Numbers of type f32.
    F32 = 0,
    /// Numbers of type f64.
    F64 = 1,
    /// Numbers of type i32.
    I32 = 2,
    /// Numbers of type i64.
    I64 = 3,
}

impl Into<ModelDataType> for DataType {
    fn into(self) -> ModelDataType {
        match self {
            DataType::F32 => ModelDataType::F32,
            DataType::F64 => ModelDataType::F64,
            DataType::I32 => ModelDataType::I32,
            DataType::I64 => ModelDataType::I64,
        }
    }
}
