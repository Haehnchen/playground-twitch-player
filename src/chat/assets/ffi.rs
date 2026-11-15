use super::{GAsyncReadyCallback, GCompareFunc, GDestroyNotify};
use std::ffi::{c_char, c_int, c_uint, c_void};
pub(super) const GTK_CONTENT_FIT_CONTAIN: c_int = 1;

#[repr(C)]
pub struct GArray {
    pub data: *mut c_char,
    pub len: c_uint,
}

#[repr(C)]
pub struct GAsyncResult {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GBytes {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GCancellable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkPaintable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkTexture {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GError {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GFile {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GHashTable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GObject {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkPicture {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextBuffer {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextChildAnchor {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextIter {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextView {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

unsafe extern "C" {

    pub(super) fn g_array_append_vals(
        array: *mut GArray,
        data: *const c_void,
        len: c_uint,
    ) -> *mut GArray;
    pub(super) fn g_array_new(
        zero_terminated: c_int,
        clear_: c_int,
        element_size: c_uint,
    ) -> *mut GArray;
    pub(super) fn g_array_set_clear_func(array: *mut GArray, clear_func: Option<GDestroyNotify>);
    pub(super) fn g_array_sort(array: *mut GArray, compare_func: Option<GCompareFunc>);
    pub(super) fn g_array_unref(array: *mut GArray);
    pub(super) fn g_bytes_new_take(data: *mut c_void, size: usize) -> *mut GBytes;
    pub(super) fn g_bytes_unref(bytes: *mut GBytes);
    pub(super) fn g_clear_error(error: *mut *mut GError);
    pub(super) fn g_file_load_contents_async(
        file: *mut GFile,
        cancellable: *mut GCancellable,
        callback: Option<GAsyncReadyCallback>,
        user_data: *mut c_void,
    );
    pub(super) fn g_file_load_contents_finish(
        file: *mut GFile,
        res: *mut GAsyncResult,
        contents: *mut *mut c_char,
        length: *mut usize,
        etag_out: *mut *mut c_char,
        error: *mut *mut GError,
    ) -> c_int;
    pub(super) fn g_file_new_for_uri(uri: *const c_char) -> *mut GFile;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_hash_table_destroy(hash_table: *mut GHashTable);
    pub(super) fn g_hash_table_insert(
        hash_table: *mut GHashTable,
        key: *mut c_void,
        value: *mut c_void,
    ) -> c_int;
    pub(super) fn g_hash_table_lookup(
        hash_table: *mut GHashTable,
        key: *const c_void,
    ) -> *mut c_void;
    pub(super) fn g_hash_table_new_full(
        hash_func: Option<unsafe extern "C" fn(*const c_void) -> c_uint>,
        key_equal_func: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
        key_destroy_func: Option<GDestroyNotify>,
        value_destroy_func: Option<GDestroyNotify>,
    ) -> *mut GHashTable;
    pub(super) fn g_object_ref(object: *mut c_void) -> *mut c_void;
    pub(super) fn g_object_unref(object: *mut c_void);
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;
    pub(super) fn g_str_equal(v1: *const c_void, v2: *const c_void) -> c_int;
    pub(super) fn g_str_hash(v: *const c_void) -> c_uint;

    pub(super) fn gdk_texture_new_from_bytes(
        bytes: *mut GBytes,
        error: *mut *mut GError,
    ) -> *mut GdkTexture;
    pub(super) fn gtk_picture_new() -> *mut GtkWidget;
    pub(super) fn gtk_picture_set_can_shrink(self_: *mut GtkPicture, can_shrink: c_int);
    pub(super) fn gtk_picture_set_content_fit(self_: *mut GtkPicture, content_fit: c_int);
    pub(super) fn gtk_picture_set_paintable(self_: *mut GtkPicture, paintable: *mut GdkPaintable);
    pub(super) fn gtk_text_buffer_insert(
        buffer: *mut GtkTextBuffer,
        iter: *mut GtkTextIter,
        text: *const c_char,
        len: c_int,
    );
    pub(super) fn gtk_text_buffer_insert_child_anchor(
        buffer: *mut GtkTextBuffer,
        iter: *mut GtkTextIter,
        anchor: *mut GtkTextChildAnchor,
    );
    pub(super) fn gtk_text_child_anchor_new() -> *mut GtkTextChildAnchor;
    pub(super) fn gtk_text_view_add_child_at_anchor(
        text_view: *mut GtkTextView,
        child: *mut GtkWidget,
        anchor: *mut GtkTextChildAnchor,
    );
    pub(super) fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    pub(super) fn gtk_widget_set_focusable(widget: *mut GtkWidget, focusable: c_int);
    pub(super) fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
}
