use crate::ffi::{ERR_NULLPTR, OK};
use std::os::raw::c_int;
use xaynet_core::mask::DataType;

mod pv {
    use super::ModelConfig;
    ffi_support::define_box_destructor!(ModelConfig, _xaynet_ffi_model_config_destroy);
}

/// Destroy the model configuration created by [`xaynet_ffi_participant_model_config()`].
///
/// # Return value
///
/// - [`OK`] on success
/// - [`ERR_NULLPTR`] if `model_config` is NULL
///
/// # Safety
///
/// 1. When calling this method, you have to ensure that *either* the pointer is NULL
///    *or* all of the following is true:
///    - The pointer must be properly [aligned].
///    - It must be "dereferencable" in the sense defined in the [`::std::ptr`] module
///      documentation.
/// 2. After destroying the `ModelConfig`, the pointer becomes invalid and must not be
///    used.
/// 3. This function should only be called on a pointer that has been created by
///    [`xaynet_ffi_participant_model_config()`].
///
/// [`::std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_model_config_destroy(model_config: *mut ModelConfig) -> c_int {
    if model_config.is_null() {
        return ERR_NULLPTR;
    }
    pv::_xaynet_ffi_model_config_destroy(model_config);
    OK
}

#[repr(C)]
/// The model configuration of the model that is expected in [`xaynet_ffi_participant_set_model`].
pub struct ModelConfig {
    // The expected data type of the model.
    pub data_type: DataType,
    // the expected length of the model.
    pub len: u64,
}

impl Into<ModelConfig> for xaynet_sdk::ModelConfig {
    fn into(self) -> ModelConfig {
        ModelConfig {
            data_type: self.data_type,
            len: self.len as u64,
        }
    }
}
