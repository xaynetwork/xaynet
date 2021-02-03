use std::os::raw::{c_int, c_uint};

use ffi_support::{ByteBuffer, FfiStr};
use xaynet_core::crypto::{ByteObject, PublicSigningKey, SecretSigningKey, SigningKeyPair};
use zeroize::Zeroize;

use super::{
    ERR_CRYPTO_PUBLIC_KEY,
    ERR_CRYPTO_SECRET_KEY,
    ERR_INVALID_URL,
    ERR_NULLPTR,
    ERR_SETTINGS_KEYS,
    ERR_SETTINGS_URL,
    OK,
};
use crate::{Settings, SettingsError};

mod pv {
    use super::Settings;
    ffi_support::define_box_destructor!(Settings, _xaynet_ffi_settings_destroy);
}

/// Destroy the settings created by [`xaynet_ffi_settings_new()`].
///
/// # Return value
///
/// - [`OK`] on success
/// - [`ERR_NULLPTR`] if `buf` is NULL
///
/// # Safety
///
/// 1. When calling this method, you have to ensure that *either* the pointer is NULL
///    *or* all of the following is true:
///    - The pointer must be properly [aligned].
///    - It must be "dereferencable" in the sense defined in the [`std::ptr`] module
///      documentation.
/// 2. After destroying the `Settings`, the pointer becomes invalid and must not be
///    used.
/// 3. This function should only be called on a pointer that has been created by
///    [`xaynet_ffi_settings_new`].
///
/// [`std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_settings_destroy(settings: *mut Settings) -> c_int {
    if settings.is_null() {
        return ERR_NULLPTR;
    }
    pv::_xaynet_ffi_settings_destroy(settings);
    OK
}

/// Create new [`Settings`] and return a pointer to it.
///
/// # Safety
///
/// The `Settings` created by this function must be destroyed with
/// [`xaynet_ffi_settings_destroy()`]. Attempting to free the memory from the other side
/// of the FFI is UB.
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_settings_new() -> *mut Settings {
    Box::into_raw(Box::new(Settings::new()))
}

/// Set scalar setting.
///
/// # Return value
///
/// - [`OK`] if successful
/// - [`ERR_NULLPTR`] if `settings` is `NULL`
///
/// # Safety
///
/// When calling this method, you have to ensure that *either* the pointer is NULL *or*
/// all of the following is true:
/// - The pointer must be properly [aligned].
/// - It must be "dereferencable" in the sense defined in the [`std::ptr`] module
///   documentation.
///
/// [`std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_settings_set_scalar(
    settings: *mut Settings,
    numer: c_uint,
    denom: c_uint,
) -> c_int {
    match unsafe { settings.as_mut() } {
        Some(settings) => {
            settings.set_scalar(numer, denom);
            OK
        }
        None => ERR_NULLPTR,
    }
}

/// Set coordinator URL.
///
/// # Return value
///
/// - [`OK`] if successful
/// - [`ERR_INVALID_URL`] if `url` is not a valid string
/// - [`ERR_NULLPTR`] if `settings` is `NULL`
///
/// # Safety
///
/// When calling this method, you have to ensure that *either* the pointers are NULL
/// *or* all of the following is true:
/// - The pointers must be properly [aligned].
/// - They must be "dereferencable" in the sense defined in the [`std::ptr`] module
///   documentation.
///
/// [`std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_settings_set_url(
    settings: *mut Settings,
    url: FfiStr,
) -> c_int {
    let url = match url.as_opt_str() {
        Some(url) => url,
        None => return ERR_INVALID_URL,
    };
    match unsafe { settings.as_mut() } {
        Some(settings) => {
            settings.set_url(url.to_string());
            OK
        }
        None => ERR_NULLPTR,
    }
}

// TODO: add a way to save the key pair
/// A signing key pair
pub struct KeyPair {
    public: ByteBuffer,
    secret: ByteBuffer,
}

// TODO: document that crypto must be initialized.
/// Generate a new signing key pair that can be used in the [`Settings`]. **Before
/// calling this function you must initialize the crypto library with
/// [`xaynet_ffi_crypto_init()`]**.
///
/// The returned value contains a pointer to the secret key. For security reasons, you
/// must make sure that this buffer life is a short as possible, and call
/// [`xaynet_ffi_forget_key_pair`] to destroy it.
///
/// [`xaynet_ffi_crypto_init()`]: crate::ffi::xaynet_ffi_crypto_init
///
/// # Safety
///
/// This function is safe to call
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_generate_key_pair() -> *const KeyPair {
    let SigningKeyPair { public, secret } = SigningKeyPair::generate();
    let public_vec = public.as_slice().to_vec();
    let secret_vec = secret.as_slice().to_vec();
    let keys = KeyPair {
        public: ByteBuffer::from_vec(public_vec),
        // under the hood, ByteBuffer takes ownership of the memory
        // without copying/leaking anything. There's no need to zero
        // out anything yet
        secret: ByteBuffer::from_vec(secret_vec),
    };
    Box::into_raw(Box::new(keys))
}

/// De-allocate the buffers that contain the signing keys, and zero out the content of
/// the buffer that contains the secret key.
///
/// # Return value
///
/// - [`ERR_NULLPTR`] is `key_pair` is NULL
/// - [`OK`] otherwise
///
/// # Safety
///
/// When calling this method, you have to ensure that *either* the pointer is NULL *or*
/// all of the following is true:
/// - The pointer must be properly [aligned].
/// - It must be "dereferencable" in the sense defined in the [`std::ptr`] module
///   documentation.
///
/// [`std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_forget_key_pair(key_pair: *const KeyPair) -> c_int {
    if key_pair.is_null() {
        return ERR_NULLPTR;
    }
    let key_pair = unsafe { Box::from_raw(key_pair as *mut KeyPair) };
    // IMPORTANT: we need to free the ByteBuffer memory, since it does
    // not implement drop. We also take care of zero-ing the memory
    // for the secret key.
    key_pair.secret.destroy_into_vec().zeroize();
    key_pair.public.destroy_into_vec();
    OK
}

/// Set participant signing keys.
///
/// # Return value
///
/// - [`OK`] if successful
/// - [`ERR_NULLPTR`] if `settings` or `key_pair` is `NULL`
/// - [`ERR_CRYPTO_PUBLIC_KEY`] if the given `key_pair` contains an invalid public key
/// - [`ERR_CRYPTO_SECRET_KEY`] if the given `key_pair` contains an invalid secret key
///
/// # Safety
///
/// When calling this method, you have to ensure that *either* the pointers are NULL
/// *or* all of the following is true:
/// - The pointers must be properly [aligned].
/// - They must be "dereferencable" in the sense defined in the [`std::ptr`] module
///   documentation.
///
/// [`std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_settings_set_keys(
    settings: *mut Settings,
    key_pair: *const KeyPair,
) -> c_int {
    let key_pair = match unsafe { key_pair.as_ref() } {
        Some(key_pair) => key_pair,
        None => return ERR_NULLPTR,
    };

    let secret_slice = key_pair.secret.as_slice();
    if secret_slice.len() != SecretSigningKey::LENGTH {
        return ERR_CRYPTO_SECRET_KEY;
    }
    let secret = SecretSigningKey::from_slice_unchecked(secret_slice);

    let public_slice = key_pair.public.as_slice();
    if public_slice.len() != PublicSigningKey::LENGTH {
        return ERR_CRYPTO_PUBLIC_KEY;
    }
    let public = PublicSigningKey::from_slice_unchecked(public_slice);

    match unsafe { settings.as_mut() } {
        Some(settings) => {
            settings.set_keys(SigningKeyPair { public, secret });
            OK
        }
        None => ERR_NULLPTR,
    }
}

/// Check whether the given settings are valid and can be used to instantiate a
/// participant (see [`xaynet_ffi_participant_new()`]).
///
/// # Return value
///
/// - [`OK`] on success
/// - [`ERR_SETTINGS_URL`] if the URL has not been set
/// - [`ERR_SETTINGS_KEYS`] if the signing keys have not been set
///
/// # Safety
///
/// When calling this method, you have to ensure that *either* the pointer is NULL *or*
/// all of the following is true:
///
/// - The pointer must be properly [aligned].
/// - It must be "dereferencable" in the sense defined in the [`std::ptr`] module
///   documentation.
///
/// [`xaynet_ffi_participant_new()`]: crate::ffi::xaynet_ffi_participant_new
/// [`std::ptr`]: https://doc.rust-lang.org/std/ptr/index.html#safety
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[no_mangle]
pub unsafe extern "C" fn xaynet_ffi_check_settings(settings: *const Settings) -> c_int {
    match unsafe { settings.as_ref() } {
        Some(settings) => match settings.check() {
            Ok(()) => OK,
            Err(SettingsError::MissingUrl) => ERR_SETTINGS_URL,
            Err(SettingsError::MissingKeys) => ERR_SETTINGS_KEYS,
        },
        None => ERR_NULLPTR,
    }
}
