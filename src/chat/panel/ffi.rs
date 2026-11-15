use super::{GDestroyNotify, GSourceFunc};
use std::ffi::{c_char, c_double, c_int, c_uint, c_void};
pub(super) const G_SOURCE_REMOVE: c_int = 0;
pub(super) const GDK_EVENT_PROPAGATE: c_int = 0;
pub(super) const GTK_EVENT_CONTROLLER_SCROLL_VERTICAL: c_int = 1;
pub(super) const GTK_ORIENTATION_VERTICAL: c_int = 1;
pub(super) const GTK_POLICY_AUTOMATIC: c_int = 1;
pub(super) const GTK_POLICY_NEVER: c_int = 2;
pub(super) const GTK_WRAP_WORD_CHAR: c_int = 3;
pub(super) const PANGO_WEIGHT_BOLD: c_int = 700;

#[repr(C)]
pub struct GHashTable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GSource {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkRGBA {
    pub(super) red: f32,
    pub(super) green: f32,
    pub(super) blue: f32,
    pub(super) alpha: f32,
}

#[repr(C)]
pub struct GtkAdjustment {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkBox {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkEventController {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkEventControllerScroll {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkScrolledWindow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextBuffer {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GtkTextIter {
    pub(super) dummy1: *mut c_void,
    pub(super) dummy2: *mut c_void,
    pub(super) dummy3: c_int,
    pub(super) dummy4: c_int,
    pub(super) dummy5: c_int,
    pub(super) dummy6: c_int,
    pub(super) dummy7: c_int,
    pub(super) dummy8: c_int,
    pub(super) dummy9: *mut c_void,
    pub(super) dummy10: *mut c_void,
    pub(super) dummy11: c_int,
    pub(super) dummy12: c_int,
    pub(super) dummy13: c_int,
    pub(super) dummy14: *mut c_void,
}

#[repr(C)]
pub struct GtkTextTag {
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
    pub(super) fn g_hash_table_size(hash_table: *mut GHashTable) -> c_uint;
    pub(super) fn g_idle_add_full(
        priority: c_int,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
    pub(super) fn g_main_context_find_source_by_id(
        context: *mut c_void,
        source_id: c_uint,
    ) -> *mut GSource;
    pub(super) fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *const c_void,
        data: *mut c_void,
        destroy_data: *mut c_void,
        connect_flags: c_int,
    ) -> usize;
    pub(super) fn g_signal_handler_disconnect(instance: *mut c_void, handler_id: usize);
    pub(super) fn g_source_destroy(source: *mut GSource);
    pub(super) fn g_str_equal(v1: *const c_void, v2: *const c_void) -> c_int;
    pub(super) fn g_str_hash(v: *const c_void) -> c_uint;
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;

    pub(super) fn g_object_ref_sink(object: *mut c_void) -> *mut c_void;
    pub(super) fn g_object_unref(object: *mut c_void);

    pub(super) fn gdk_rgba_parse(rgba: *mut GdkRGBA, spec: *const c_char) -> c_int;
    pub(super) fn gtk_adjustment_get_page_size(adjustment: *mut GtkAdjustment) -> c_double;
    pub(super) fn gtk_adjustment_get_upper(adjustment: *mut GtkAdjustment) -> c_double;
    pub(super) fn gtk_adjustment_get_value(adjustment: *mut GtkAdjustment) -> c_double;
    pub(super) fn gtk_adjustment_set_value(adjustment: *mut GtkAdjustment, value: c_double);
    pub(super) fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    pub(super) fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    pub(super) fn gtk_event_controller_scroll_new(flags: c_int) -> *mut GtkEventController;
    pub(super) fn gtk_scrolled_window_get_vadjustment(
        scrolled_window: *mut GtkScrolledWindow,
    ) -> *mut GtkAdjustment;
    pub(super) fn gtk_scrolled_window_new() -> *mut GtkWidget;
    pub(super) fn gtk_scrolled_window_set_child(
        scrolled_window: *mut GtkScrolledWindow,
        child: *mut GtkWidget,
    );
    pub(super) fn gtk_scrolled_window_set_policy(
        scrolled_window: *mut GtkScrolledWindow,
        hscrollbar_policy: c_int,
        vscrollbar_policy: c_int,
    );
    pub(super) fn gtk_text_buffer_create_tag(
        buffer: *mut GtkTextBuffer,
        tag_name: *const c_char,
        first_property_name: *const c_char,
        ...
    ) -> *mut GtkTextTag;
    pub(super) fn gtk_text_buffer_delete(
        buffer: *mut GtkTextBuffer,
        start: *mut GtkTextIter,
        end: *mut GtkTextIter,
    );
    pub(super) fn gtk_text_buffer_get_end_iter(buffer: *mut GtkTextBuffer, iter: *mut GtkTextIter);
    pub(super) fn gtk_text_buffer_get_start_iter(
        buffer: *mut GtkTextBuffer,
        iter: *mut GtkTextIter,
    );
    pub(super) fn gtk_text_buffer_insert(
        buffer: *mut GtkTextBuffer,
        iter: *mut GtkTextIter,
        text: *const c_char,
        len: c_int,
    );
    pub(super) fn gtk_text_buffer_insert_with_tags(
        buffer: *mut GtkTextBuffer,
        iter: *mut GtkTextIter,
        text: *const c_char,
        len: c_int,
        first_tag: *mut GtkTextTag,
        ...
    );
    pub(super) fn gtk_text_buffer_set_text(
        buffer: *mut GtkTextBuffer,
        text: *const c_char,
        len: c_int,
    );
    pub(super) fn gtk_text_iter_forward_line(iter: *mut GtkTextIter) -> c_int;
    pub(super) fn gtk_text_view_get_buffer(text_view: *mut GtkTextView) -> *mut GtkTextBuffer;
    pub(super) fn gtk_text_view_new() -> *mut GtkWidget;
    pub(super) fn gtk_text_view_set_bottom_margin(
        text_view: *mut GtkTextView,
        bottom_margin: c_int,
    );
    pub(super) fn gtk_text_view_set_cursor_visible(text_view: *mut GtkTextView, setting: c_int);
    pub(super) fn gtk_text_view_set_editable(text_view: *mut GtkTextView, setting: c_int);
    pub(super) fn gtk_text_view_set_left_margin(text_view: *mut GtkTextView, left_margin: c_int);
    pub(super) fn gtk_text_view_set_right_margin(text_view: *mut GtkTextView, right_margin: c_int);
    pub(super) fn gtk_text_view_set_top_margin(text_view: *mut GtkTextView, top_margin: c_int);
    pub(super) fn gtk_text_view_set_wrap_mode(text_view: *mut GtkTextView, wrap_mode: c_int);
    pub(super) fn gtk_widget_add_controller(
        widget: *mut GtkWidget,
        controller: *mut GtkEventController,
    );
    pub(super) fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    pub(super) fn gtk_widget_set_focusable(widget: *mut GtkWidget, focusable: c_int);
    pub(super) fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    pub(super) fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    pub(super) fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);

}
