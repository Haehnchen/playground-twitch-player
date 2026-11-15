use super::{GAsyncReadyCallback, GDestroyNotify, GTaskThreadFunc};
use crate::twitch::stream_info::{GCancellable, GError, GPtrArray};
use std::ffi::{c_char, c_int, c_uint, c_void};
pub(super) const G_IO_ERROR_FAILED: c_int = 0;
pub(super) const G_IO_ERROR_CANCELLED: c_int = 19;
pub(super) const G_USEC_PER_SEC: i64 = 1_000_000;

#[repr(C)]
pub struct GTask {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub(super) fn g_clear_error(error: *mut *mut GError);
    pub(super) fn g_error_matches(error: *const GError, domain: c_uint, code: c_int) -> c_int;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_get_real_time() -> i64;
    pub(super) fn g_io_error_quark() -> c_uint;
    pub(super) fn g_malloc0(n_bytes: usize) -> *mut c_void;
    pub(super) fn g_object_unref(object: *mut c_void);
    pub(super) fn g_propagate_prefixed_error(
        dest: *mut *mut GError,
        src: *mut GError,
        format: *const c_char,
        ...
    );
    pub(super) fn g_ptr_array_add(array: *mut GPtrArray, data: *mut c_void);
    pub(super) fn g_ptr_array_new_with_free_func(
        element_free_func: Option<GDestroyNotify>,
    ) -> *mut GPtrArray;
    pub(super) fn g_ptr_array_unref(array: *mut GPtrArray);
    pub(super) fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;
    pub(super) fn g_strfreev(str_array: *mut *mut c_char);

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

}
