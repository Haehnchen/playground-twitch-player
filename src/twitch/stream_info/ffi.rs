use super::{GAsyncReadyCallback, GDestroyNotify, GTaskThreadFunc};
use std::ffi::{c_char, c_int, c_uint, c_void};
pub(super) const G_IO_ERROR_FAILED: c_int = 0;

pub use gtk4::gio::ffi::{GAsyncResult, GCancellable, GTask};

pub use gtk4::glib::ffi::{GDateTime, GError};

unsafe extern "C" {
    pub(super) fn g_date_time_difference(end: *mut GDateTime, begin: *mut GDateTime) -> i64;
    pub(super) fn g_date_time_new_from_iso8601(
        text: *const c_char,
        default_tz: *mut c_void,
    ) -> *mut GDateTime;
    pub(super) fn g_date_time_new_now_utc() -> *mut GDateTime;
    pub(super) fn g_date_time_unref(datetime: *mut GDateTime);
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_io_error_quark() -> c_uint;
    pub(super) fn g_object_unref(object: *mut c_void);
    pub(super) fn g_quark_from_static_string(string: *const c_char) -> c_uint;
    pub(super) fn g_random_int_range(begin: c_int, end: c_int) -> c_int;
    pub(super) fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;
    pub(super) fn g_task_is_valid(result: *mut c_void, source_object: *mut c_void) -> c_int;
    pub(super) fn g_task_new(
        source_object: *mut c_void,
        cancellable: *mut GCancellable,
        callback: Option<GAsyncReadyCallback>,
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
    pub(super) fn g_uri_escape_string(
        unescaped: *const c_char,
        reserved_chars_allowed: *const c_char,
        allow_utf8: c_int,
    ) -> *mut c_char;

}

unsafe extern "C" {
    pub(super) fn g_cancellable_is_cancelled(cancellable: *mut GCancellable) -> c_int;
}
