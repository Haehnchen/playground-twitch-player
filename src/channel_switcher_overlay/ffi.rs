//! GTK, GDK, and GLib bindings.

use crate::twitch_stream_info::{
    GAsyncReadyCallback, GAsyncResult, GCancellable, GError, GPtrArray,
};
use std::ffi::{c_char, c_double, c_int, c_uint, c_void};

pub(super) const FALSE: c_int = 0;
pub(super) const TRUE: c_int = 1;
pub(super) const G_SOURCE_REMOVE: c_int = 0;
pub(super) const G_SOURCE_CONTINUE: c_int = 1;
pub(super) const G_USEC_PER_SEC: i64 = 1_000_000;
pub(super) const G_IO_ERROR_FAILED: c_int = 0;
pub(super) const G_IO_ERROR_INVALID_ARGUMENT: c_int = 13;
pub(super) const G_IO_ERROR_CANCELLED: c_int = 19;
pub(super) const G_LOG_LEVEL_DEBUG: c_int = 1 << 7;
pub(super) const GTK_ALIGN_FILL: c_int = 0;
pub(super) const GTK_ALIGN_START: c_int = 1;
pub(super) const GTK_ALIGN_CENTER: c_int = 3;
pub(super) const GTK_CONTENT_FIT_COVER: c_int = 2;
pub(super) const GTK_ENTRY_ICON_SECONDARY: c_int = 1;
pub(super) const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
pub(super) const GTK_ORIENTATION_VERTICAL: c_int = 1;
pub(super) const GTK_OVERFLOW_HIDDEN: c_int = 1;
pub(super) const GTK_POLICY_AUTOMATIC: c_int = 1;
pub(super) const GTK_POLICY_NEVER: c_int = 2;
pub(super) const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: c_uint = 600;
pub(super) const GDK_BUTTON_PRIMARY: c_uint = 1;
pub(super) const GDK_COLORSPACE_RGB: c_int = 0;
pub(super) const GDK_INTERP_BILINEAR: c_int = 2;
pub(super) const GDK_MEMORY_R8G8B8A8: c_int = 5;
pub(super) const G_PRIORITY_DEFAULT: c_int = 0;
pub(super) const PANGO_ELLIPSIZE_END: c_int = 3;
pub(super) const PLAYER_WINDOW_ICON_CLOSE: c_int = 2;

#[repr(C)]
pub struct GraphenePoint {
    pub(super) x: f32,
    pub(super) y: f32,
}

#[repr(C)]
pub struct GBytes {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkDisplay {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkPaintable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkPixbuf {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkTexture {
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
pub struct GInputStream {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GTask {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GObject {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GTypeInstance {
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
pub struct GtkCssProvider {
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
pub struct GtkGesture {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkGestureClick {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkGestureSingle {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkGrid {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkLabel {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkOverlay {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkPicture {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkRoot {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkScrolledWindow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkSearchEntry {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkStyleProvider {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

pub(super) type GDestroyNotify = unsafe extern "C" fn(*mut c_void);
pub(super) type GClosureNotify = unsafe extern "C" fn(*mut c_void, *mut c_void);
pub(super) type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;
pub(super) type GTaskThreadFunc =
    unsafe extern "C" fn(*mut GTask, *mut c_void, *mut c_void, *mut GCancellable);

unsafe extern "C" {
    pub(super) fn g_ascii_strcasecmp(str1: *const c_char, str2: *const c_char) -> c_int;
    pub(super) fn g_bytes_new(data: *const c_void, size: usize) -> *mut GBytes;
    pub(super) fn g_bytes_new_take(data: *mut c_void, size: usize) -> *mut GBytes;
    pub(super) fn g_bytes_unref(bytes: *mut GBytes);
    pub(super) fn g_cancellable_cancel(cancellable: *mut GCancellable);
    pub(super) fn g_cancellable_new() -> *mut GCancellable;
    pub(super) fn g_clear_error(error: *mut *mut GError);
    pub(super) fn g_error_matches(error: *const GError, domain: c_uint, code: c_int) -> c_int;
    pub(super) fn g_file_load_contents_async(
        file: *mut GFile,
        cancellable: *mut GCancellable,
        callback: Option<GAsyncReadyCallback>,
        user_data: *mut c_void,
    );
    pub(super) fn g_file_load_contents_finish(
        file: *mut GFile,
        result: *mut GAsyncResult,
        contents: *mut *mut c_char,
        length: *mut usize,
        etag_out: *mut *mut c_char,
        error: *mut *mut GError,
    ) -> c_int;
    pub(super) fn g_file_new_for_uri(uri: *const c_char) -> *mut GFile;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_get_monotonic_time() -> i64;
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
    pub(super) fn g_hash_table_destroy(hash_table: *mut GHashTable);
    pub(super) fn g_hash_table_remove(hash_table: *mut GHashTable, key: *const c_void) -> c_int;
    pub(super) fn g_hash_table_remove_all(hash_table: *mut GHashTable);
    #[cfg(test)]
    pub(super) fn g_idle_add_full(
        priority: c_int,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
    pub(super) fn g_io_error_quark() -> c_uint;
    pub(super) fn g_log(log_domain: *const c_char, log_level: c_int, format: *const c_char, ...);
    pub(super) fn g_memory_input_stream_new_from_bytes(bytes: *mut GBytes) -> *mut GInputStream;
    pub(super) fn g_object_add_weak_pointer(
        object: *mut GObject,
        weak_pointer_location: *mut *mut c_void,
    );
    pub(super) fn g_object_get_data(object: *mut GObject, key: *const c_char) -> *mut c_void;
    pub(super) fn g_object_ref(object: *mut c_void) -> *mut c_void;
    pub(super) fn g_object_ref_sink(object: *mut c_void) -> *mut c_void;
    pub(super) fn g_object_remove_weak_pointer(
        object: *mut GObject,
        weak_pointer_location: *mut *mut c_void,
    );
    pub(super) fn g_object_set_data_full(
        object: *mut GObject,
        key: *const c_char,
        data: *mut c_void,
        destroy: Option<GDestroyNotify>,
    );
    pub(super) fn g_object_unref(object: *mut c_void);
    pub(super) fn g_ptr_array_add(array: *mut GPtrArray, data: *mut c_void);
    pub(super) fn g_ptr_array_new_with_free_func(
        element_free_func: Option<GDestroyNotify>,
    ) -> *mut GPtrArray;
    pub(super) fn g_ptr_array_ref(array: *mut GPtrArray) -> *mut GPtrArray;
    pub(super) fn g_ptr_array_set_size(array: *mut GPtrArray, length: c_uint);
    pub(super) fn g_ptr_array_unref(array: *mut GPtrArray);
    pub(super) fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    pub(super) fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *const c_void,
        data: *mut c_void,
        destroy_data: Option<GClosureNotify>,
        connect_flags: c_int,
    ) -> usize;
    pub(super) fn g_source_remove(tag: c_uint) -> c_int;
    pub(super) fn g_str_equal(v1: *const c_void, v2: *const c_void) -> c_int;
    pub(super) fn g_str_hash(v: *const c_void) -> c_uint;
    pub(super) fn g_strcmp0(str1: *const c_char, str2: *const c_char) -> c_int;
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;
    pub(super) fn g_strdup_printf(format: *const c_char, ...) -> *mut c_char;
    pub(super) fn g_strfreev(str_array: *mut *mut c_char);
    pub(super) fn g_timeout_add_full(
        priority: c_int,
        interval: c_uint,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
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
    pub(super) fn g_utf8_casefold(str: *const c_char, len: isize) -> *mut c_char;

    pub(super) fn gdk_display_get_default() -> *mut GdkDisplay;
    pub(super) fn gdk_memory_texture_new(
        width: c_int,
        height: c_int,
        format: c_int,
        bytes: *mut GBytes,
        stride: usize,
    ) -> *mut GdkTexture;
    pub(super) fn gdk_pixbuf_composite(
        src: *mut GdkPixbuf,
        dest: *mut GdkPixbuf,
        dest_x: c_int,
        dest_y: c_int,
        dest_width: c_int,
        dest_height: c_int,
        offset_x: c_double,
        offset_y: c_double,
        scale_x: c_double,
        scale_y: c_double,
        interp_type: c_int,
        overall_alpha: c_int,
    );
    pub(super) fn gdk_pixbuf_fill(pixbuf: *mut GdkPixbuf, pixel: c_uint);
    pub(super) fn gdk_pixbuf_get_height(pixbuf: *const GdkPixbuf) -> c_int;
    pub(super) fn gdk_pixbuf_get_pixels(pixbuf: *const GdkPixbuf) -> *mut u8;
    pub(super) fn gdk_pixbuf_get_rowstride(pixbuf: *const GdkPixbuf) -> c_int;
    pub(super) fn gdk_pixbuf_get_width(pixbuf: *const GdkPixbuf) -> c_int;
    pub(super) fn gdk_pixbuf_new(
        colorspace: c_int,
        has_alpha: c_int,
        bits_per_sample: c_int,
        width: c_int,
        height: c_int,
    ) -> *mut GdkPixbuf;
    pub(super) fn gdk_pixbuf_new_from_stream(
        stream: *mut GInputStream,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut GdkPixbuf;

    pub(super) fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    pub(super) fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    pub(super) fn gtk_button_new() -> *mut GtkWidget;
    pub(super) fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    pub(super) fn gtk_css_provider_load_from_string(
        css_provider: *mut GtkCssProvider,
        string: *const c_char,
    );
    pub(super) fn gtk_css_provider_new() -> *mut GtkCssProvider;
    pub(super) fn gtk_editable_get_text(editable: *mut GtkEditable) -> *const c_char;
    pub(super) fn gtk_editable_set_text(editable: *mut GtkEditable, text: *const c_char);
    pub(super) fn gtk_entry_new() -> *mut GtkWidget;
    pub(super) fn gtk_entry_set_icon_from_icon_name(
        entry: *mut GtkEntry,
        icon_pos: c_int,
        icon_name: *const c_char,
    );
    pub(super) fn gtk_entry_set_icon_tooltip_text(
        entry: *mut GtkEntry,
        icon_pos: c_int,
        tooltip: *const c_char,
    );
    pub(super) fn gtk_entry_set_placeholder_text(entry: *mut GtkEntry, text: *const c_char);
    pub(super) fn gtk_gesture_click_new() -> *mut GtkGesture;
    pub(super) fn gtk_gesture_single_set_button(gesture: *mut GtkGestureSingle, button: c_uint);
    pub(super) fn gtk_grid_attach(
        grid: *mut GtkGrid,
        child: *mut GtkWidget,
        column: c_int,
        row: c_int,
        width: c_int,
        height: c_int,
    );
    pub(super) fn gtk_grid_new() -> *mut GtkWidget;
    pub(super) fn gtk_grid_remove(grid: *mut GtkGrid, child: *mut GtkWidget);
    pub(super) fn gtk_grid_set_column_spacing(grid: *mut GtkGrid, spacing: c_uint);
    pub(super) fn gtk_grid_set_row_spacing(grid: *mut GtkGrid, spacing: c_uint);
    pub(super) fn gtk_label_new(str: *const c_char) -> *mut GtkWidget;
    pub(super) fn gtk_label_set_ellipsize(label: *mut GtkLabel, mode: c_int);
    pub(super) fn gtk_label_set_max_width_chars(label: *mut GtkLabel, n_chars: c_int);
    pub(super) fn gtk_label_set_single_line_mode(label: *mut GtkLabel, single_line_mode: c_int);
    pub(super) fn gtk_label_set_wrap(label: *mut GtkLabel, wrap: c_int);
    pub(super) fn gtk_label_set_xalign(label: *mut GtkLabel, xalign: f32);
    pub(super) fn gtk_overlay_add_overlay(overlay: *mut GtkOverlay, widget: *mut GtkWidget);
    pub(super) fn gtk_overlay_remove_overlay(overlay: *mut GtkOverlay, widget: *mut GtkWidget);
    pub(super) fn gtk_picture_new() -> *mut GtkWidget;
    pub(super) fn gtk_picture_set_can_shrink(picture: *mut GtkPicture, can_shrink: c_int);
    pub(super) fn gtk_picture_set_content_fit(picture: *mut GtkPicture, content_fit: c_int);
    pub(super) fn gtk_picture_set_paintable(picture: *mut GtkPicture, paintable: *mut GdkPaintable);
    pub(super) fn gtk_scrolled_window_new() -> *mut GtkWidget;
    pub(super) fn gtk_scrolled_window_set_child(
        scrolled_window: *mut GtkScrolledWindow,
        child: *mut GtkWidget,
    );
    pub(super) fn gtk_scrolled_window_set_max_content_height(
        scrolled_window: *mut GtkScrolledWindow,
        height: c_int,
    );
    pub(super) fn gtk_scrolled_window_set_max_content_width(
        scrolled_window: *mut GtkScrolledWindow,
        width: c_int,
    );
    pub(super) fn gtk_scrolled_window_set_min_content_width(
        scrolled_window: *mut GtkScrolledWindow,
        width: c_int,
    );
    pub(super) fn gtk_scrolled_window_set_policy(
        scrolled_window: *mut GtkScrolledWindow,
        hscrollbar_policy: c_int,
        vscrollbar_policy: c_int,
    );
    pub(super) fn gtk_scrolled_window_set_propagate_natural_height(
        scrolled_window: *mut GtkScrolledWindow,
        propagate: c_int,
    );
    pub(super) fn gtk_scrolled_window_set_propagate_natural_width(
        scrolled_window: *mut GtkScrolledWindow,
        propagate: c_int,
    );
    pub(super) fn gtk_search_entry_new() -> *mut GtkWidget;
    pub(super) fn gtk_search_entry_set_placeholder_text(
        entry: *mut GtkSearchEntry,
        text: *const c_char,
    );
    pub(super) fn gtk_style_context_add_provider_for_display(
        display: *mut GdkDisplay,
        provider: *mut GtkStyleProvider,
        priority: c_uint,
    );
    pub(super) fn gtk_widget_add_controller(widget: *mut GtkWidget, controller: *mut c_void);
    pub(super) fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    pub(super) fn gtk_widget_compute_point(
        widget: *mut GtkWidget,
        target: *mut GtkWidget,
        point: *const GraphenePoint,
        out_point: *mut GraphenePoint,
    ) -> c_int;
    pub(super) fn gtk_widget_get_first_child(widget: *mut GtkWidget) -> *mut GtkWidget;
    pub(super) fn gtk_widget_get_height(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_get_next_sibling(widget: *mut GtkWidget) -> *mut GtkWidget;
    pub(super) fn gtk_widget_get_root(widget: *mut GtkWidget) -> *mut GtkRoot;
    pub(super) fn gtk_widget_get_visible(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_get_width(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_grab_focus(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_set_focusable(widget: *mut GtkWidget, focusable: c_int);
    pub(super) fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    pub(super) fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    pub(super) fn gtk_widget_set_margin_end(widget: *mut GtkWidget, margin: c_int);
    pub(super) fn gtk_widget_set_margin_start(widget: *mut GtkWidget, margin: c_int);
    pub(super) fn gtk_widget_set_margin_top(widget: *mut GtkWidget, margin: c_int);
    pub(super) fn gtk_widget_set_overflow(widget: *mut GtkWidget, overflow: c_int);
    pub(super) fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    pub(super) fn gtk_widget_set_tooltip_text(widget: *mut GtkWidget, text: *const c_char);
    pub(super) fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
    pub(super) fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
    pub(super) fn gtk_widget_set_visible(widget: *mut GtkWidget, visible: c_int);

}
