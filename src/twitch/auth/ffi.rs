use super::{GAsyncReadyCallback, GDestroyNotify, GTaskThreadFunc};
use crate::twitch::stream_info::{GCancellable, GError};
use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};
pub(super) const G_IO_ERROR_FAILED: c_int = 0;
pub(super) const G_IO_ERROR_TIMED_OUT: c_int = 24;
pub(super) const G_USEC_PER_SEC: i64 = 1_000_000;

#[repr(C)]
pub struct GTask {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GBytes {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SoupSession {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SoupMessage {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SoupMessageHeaders {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub(super) fn g_bytes_get_data(bytes: *mut GBytes, size: *mut usize) -> *const c_void;
    pub(super) fn g_bytes_new_static(data: *const c_void, size: usize) -> *mut GBytes;
    pub(super) fn g_bytes_unref(bytes: *mut GBytes);
    pub(super) fn g_cancellable_set_error_if_cancelled(
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> c_int;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_get_monotonic_time() -> i64;
    pub(super) fn g_io_error_quark() -> c_uint;
    pub(super) fn g_object_set(object: *mut c_void, first_property_name: *const c_char, ...);
    pub(super) fn g_object_unref(object: *mut c_void);
    pub(super) fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;
    pub(super) fn g_strndup(str: *const c_char, n: usize) -> *mut c_char;
    pub(super) fn g_uri_escape_string(
        unescaped: *const c_char,
        reserved_chars_allowed: *const c_char,
        allow_utf8: c_int,
    ) -> *mut c_char;
    pub(super) fn g_usleep(microseconds: c_ulong);

    pub(super) fn g_task_is_valid(result: *mut c_void, source_object: *mut c_void) -> c_int;
    pub(super) fn g_task_new(
        source_object: *mut c_void,
        cancellable: *mut GCancellable,
        callback: GAsyncReadyCallback,
        callback_data: *mut c_void,
    ) -> *mut GTask;
    pub(super) fn g_task_propagate_pointer(
        task: *mut GTask,
        error: *mut *mut GError,
    ) -> *mut c_void;
    pub(super) fn g_task_return_error(task: *mut GTask, error: *mut GError);
    pub(super) fn g_task_return_pointer(
        task: *mut GTask,
        result: *mut c_void,
        result_destroy: Option<GDestroyNotify>,
    );
    pub(super) fn g_task_run_in_thread(task: *mut GTask, task_func: Option<GTaskThreadFunc>);
    pub(super) fn g_task_set_task_data(
        task: *mut GTask,
        task_data: *mut c_void,
        task_data_destroy: Option<GDestroyNotify>,
    );

    pub(super) fn soup_message_get_request_headers(
        msg: *mut SoupMessage,
    ) -> *mut SoupMessageHeaders;
    pub(super) fn soup_message_get_status(msg: *mut SoupMessage) -> c_uint;
    pub(super) fn soup_message_headers_append(
        headers: *mut SoupMessageHeaders,
        name: *const c_char,
        value: *const c_char,
    );
    pub(super) fn soup_message_new(
        method: *const c_char,
        uri_string: *const c_char,
    ) -> *mut SoupMessage;
    pub(super) fn soup_message_set_request_body_from_bytes(
        msg: *mut SoupMessage,
        content_type: *const c_char,
        bytes: *mut GBytes,
    );
    pub(super) fn soup_session_new() -> *mut SoupSession;
    pub(super) fn soup_session_send_and_read(
        session: *mut SoupSession,
        msg: *mut SoupMessage,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut GBytes;
}
