mod ffi;
mod parser;
mod requests;

pub use self::ffi::GTask;
pub use self::requests::twitch_auth_refresh_token;

use self::ffi::{
    g_object_unref, g_task_is_valid, g_task_new, g_task_propagate_pointer, g_task_run_in_thread,
    g_task_set_task_data,
};
use self::requests::{
    device_code_data_free, poll_device_token_worker, poll_token_data_free,
    request_device_code_worker,
};
use crate::twitch::stream_info::{GAsyncResult, GCancellable, GError};
use std::ffi::{CStr, CString, c_char, c_uint, c_void};
use std::ptr;

pub struct TwitchAuthDeviceCode {
    pub device_code: CString,
    pub user_code: CString,
    pub verification_uri: CString,
    pub expires_in: c_uint,
    pub interval: c_uint,
}

pub struct TwitchAuthToken {
    pub access_token: CString,
    pub refresh_token: CString,
    pub expires_in: c_uint,
}

struct DeviceCodeData {
    client_id: CString,
}

struct PollTokenData {
    client_id: CString,
    device_code: CString,
    expires_in: c_uint,
    interval: c_uint,
}

struct AuthResponse {
    status: c_uint,
    body: gtk4::glib::Bytes,
}

pub type GAsyncReadyCallback =
    Option<unsafe extern "C" fn(*mut c_void, *mut GAsyncResult, *mut c_void)>;

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

type GTaskThreadFunc =
    unsafe extern "C" fn(*mut GTask, *mut c_void, *mut c_void, *mut GCancellable);

/// # Safety
/// - Null or owned module result.
/// - Free once; no remaining borrowers. Not a stack struct or borrowed array entry.
pub unsafe fn twitch_auth_device_code_free(code: *mut TwitchAuthDeviceCode) {
    unsafe {
        if code.is_null() {
            return;
        }

        drop(Box::from_raw(code));
    }
}

/// # Safety
/// - Null or owned module result.
/// - Free once; no remaining borrowers. Not a stack struct or borrowed array entry.
pub unsafe fn twitch_auth_token_free(token: *mut TwitchAuthToken) {
    unsafe {
        if token.is_null() {
            return;
        }

        drop(Box::from_raw(token));
    }
}

/// # Safety
/// - `cancel`: null or live `GCancellable`.
/// - Callback: declared C ABI, no unwind; data valid through completion.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn twitch_auth_request_device_code_async(
    client_id: *const c_char,
    cancel: *mut GCancellable,
    callback: GAsyncReadyCallback,
    user_data: *mut c_void,
) {
    unsafe {
        if client_id.is_null() || *client_id == 0 {
            return;
        }

        let data = Box::new(DeviceCodeData {
            client_id: CStr::from_ptr(client_id).to_owned(),
        });
        let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
        g_task_set_task_data(
            task,
            Box::into_raw(data) as *mut c_void,
            Some(device_code_data_free),
        );
        g_task_run_in_thread(task, Some(request_device_code_worker));
        g_object_unref(task as *mut c_void);
    }
}

/// # Safety
/// - `result`: matching async result, consumed once on originating main context.
/// - `error`: null or writable, initially null `GError*` slot.
/// - Result owned; release with matching module free function.
pub unsafe fn twitch_auth_request_device_code_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut TwitchAuthDeviceCode {
    unsafe {
        if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
            return ptr::null_mut();
        }

        g_task_propagate_pointer(result as *mut GTask, error) as *mut TwitchAuthDeviceCode
    }
}

/// # Safety
/// - `cancel`: null or live `GCancellable`.
/// - Callback: declared C ABI, no unwind; data valid through completion.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
/// - `code`: null or live device-code result with valid C strings.
pub unsafe fn twitch_auth_poll_device_token_async(
    client_id: *const c_char,
    code: *const TwitchAuthDeviceCode,
    cancel: *mut GCancellable,
    callback: GAsyncReadyCallback,
    user_data: *mut c_void,
) {
    unsafe {
        if client_id.is_null()
            || *client_id == 0
            || code.is_null()
            || (*code).device_code.is_empty()
        {
            return;
        }

        let data = Box::new(PollTokenData {
            client_id: CStr::from_ptr(client_id).to_owned(),
            device_code: (*code).device_code.clone(),
            expires_in: (*code).expires_in,
            interval: (*code).interval,
        });
        let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
        g_task_set_task_data(
            task,
            Box::into_raw(data) as *mut c_void,
            Some(poll_token_data_free),
        );
        g_task_run_in_thread(task, Some(poll_device_token_worker));
        g_object_unref(task as *mut c_void);
    }
}

/// # Safety
/// - `result`: matching async result, consumed once on originating main context.
/// - `error`: null or writable, initially null `GError*` slot.
/// - Result owned; release with matching module free function.
pub unsafe fn twitch_auth_poll_device_token_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut TwitchAuthToken {
    unsafe {
        if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
            return ptr::null_mut();
        }

        g_task_propagate_pointer(result as *mut GTask, error) as *mut TwitchAuthToken
    }
}
