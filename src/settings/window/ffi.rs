use super::{GAsyncReadyCallback, GDestroyNotify};
use crate::twitch::stream_info::{GCancellable, GError};
use std::ffi::{c_char, c_int, c_uint, c_void};
pub(super) const G_IO_ERROR_CANCELLED: c_int = 19;

#[repr(C)]
pub struct GObject {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkBox {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkButton {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkCheckButton {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkEditable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkEntry {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkLabel {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkListBox {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkListBoxRow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkScrolledWindow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkStack {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkUriLauncher {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWindow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub(super) fn g_cancellable_cancel(cancellable: *mut GCancellable);
    pub(super) fn g_cancellable_new() -> *mut GCancellable;
    pub(super) fn g_clear_error(error: *mut *mut GError);
    pub(super) fn g_error_matches(error: *mut GError, domain: c_uint, code: c_int) -> c_int;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_get_real_time() -> i64;
    pub(super) fn g_io_error_quark() -> c_uint;
    pub(super) fn g_markup_escape_text(text: *const c_char, length: isize) -> *mut c_char;
    pub(super) fn g_object_get_data(object: *mut GObject, key: *const c_char) -> *mut c_void;
    pub(super) fn g_object_ref(object: *mut c_void) -> *mut c_void;
    pub(super) fn g_object_set_data_full(
        object: *mut GObject,
        key: *const c_char,
        data: *mut c_void,
        destroy: Option<GDestroyNotify>,
    );
    pub(super) fn g_object_unref(object: *mut c_void);

    pub(super) fn gtk_button_set_label(button: *mut GtkButton, label: *const c_char);
    pub(super) fn gtk_label_set_markup(label: *mut GtkLabel, str: *const c_char);
    pub(super) fn gtk_label_set_text(label: *mut GtkLabel, str: *const c_char);
    pub(super) fn gtk_label_set_use_markup(label: *mut GtkLabel, setting: c_int);
    pub(super) fn gtk_uri_launcher_launch(
        self_: *mut GtkUriLauncher,
        parent: *mut GtkWindow,
        cancellable: *mut GCancellable,
        callback: Option<GAsyncReadyCallback>,
        user_data: *mut c_void,
    );
    pub(super) fn gtk_uri_launcher_new(uri: *const c_char) -> *mut GtkUriLauncher;
    pub(super) fn gtk_widget_set_sensitive(widget: *mut GtkWidget, sensitive: c_int);

}
