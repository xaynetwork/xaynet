use std::{ffi::CStr, os::raw::c_char, panic, ptr};

use crate::client::SyncClient;

pub unsafe extern "C" fn new_client(address: *const c_char) -> *mut SyncClient {
    if address.is_null() {
        return ptr::null_mut() as *mut SyncClient;
    }
    let address = if let Ok(address) = unsafe {
        // safe if the raw pointer `address` comes from a null-terminated C-string
        CStr::from_ptr(address)
    }
    .to_str()
    {
        address
    } else {
        return ptr::null_mut() as *mut SyncClient;
    };
    Box::into_raw(Box::new(SyncClient::new(address)))
}

pub unsafe extern "C" fn start_client(client: *mut SyncClient) {
    if !client.is_null() {
        let client = &mut *client;
        if let Err(_) = panic::catch_unwind(panic::AssertUnwindSafe(|| client.start())) {
            error!("client panicked");
        }
    }
}

pub unsafe extern "C" fn stop_client(client: *mut SyncClient) {
    if !client.is_null() {
        let client = &mut *client;
        if let Err(_) = panic::catch_unwind(panic::AssertUnwindSafe(|| client.stop())) {
            error!("client panicked");
        }
    }
}

pub unsafe extern "C" fn drop_client(client: *mut SyncClient) {
    if !client.is_null() {
        Box::from_raw(client);
    }
}
