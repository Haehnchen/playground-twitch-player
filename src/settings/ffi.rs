use std::ffi::{c_char, c_int, c_uint, c_void};

pub use gtk4::glib::ffi::GError;

unsafe extern "C" {
    pub(super) fn g_file_get_contents(
        filename: *const c_char,
        contents: *mut *mut c_char,
        length: *mut usize,
        error: *mut *mut GError,
    ) -> c_int;
    pub(super) fn g_file_set_contents(
        filename: *const c_char,
        contents: *const c_char,
        length: isize,
        error: *mut *mut GError,
    ) -> c_int;
    pub(super) fn g_build_filenamev(args: *mut *mut c_char) -> *mut c_char;
    pub(super) fn g_file_error_from_errno(err_no: c_int) -> c_int;
    pub(super) fn g_file_error_quark() -> c_uint;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_get_user_config_dir() -> *const c_char;
    pub(super) fn g_mkdir_with_parents(pathname: *const c_char, mode: c_int) -> c_int;
    pub(super) fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    pub(super) fn g_strerror(errnum: c_int) -> *const c_char;

}
