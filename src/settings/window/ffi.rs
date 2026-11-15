use super::{GAsyncReadyCallback, GDestroyNotify, GSourceFunc};
use crate::twitch::stream_info::{GCancellable, GError};
use std::ffi::{c_char, c_double, c_int, c_uint, c_void};
pub(super) const G_IO_ERROR_CANCELLED: c_int = 19;
pub(super) const G_PRIORITY_DEFAULT_IDLE: c_int = 200;
pub(super) const G_SOURCE_REMOVE: c_int = 0;
pub(super) const GTK_ALIGN_FILL: c_int = 0;
pub(super) const GTK_ALIGN_CENTER: c_int = 3;
pub(super) const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
pub(super) const GTK_ORIENTATION_VERTICAL: c_int = 1;
pub(super) const GTK_POLICY_AUTOMATIC: c_int = 1;
pub(super) const GTK_POLICY_NEVER: c_int = 2;
pub(super) const GTK_SELECTION_SINGLE: c_int = 1;

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
    pub(super) fn g_idle_add_full(
        priority: c_int,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
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
    pub(super) fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *const c_void,
        data: *mut c_void,
        destroy_data: *mut c_void,
        connect_flags: c_int,
    ) -> usize;
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;

    pub(super) fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    pub(super) fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    pub(super) fn gtk_box_remove(box_: *mut GtkBox, child: *mut GtkWidget);
    pub(super) fn gtk_button_new() -> *mut GtkWidget;
    pub(super) fn gtk_button_new_with_label(label: *const c_char) -> *mut GtkWidget;
    pub(super) fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    pub(super) fn gtk_button_set_has_frame(button: *mut GtkButton, has_frame: c_int);
    pub(super) fn gtk_button_set_label(button: *mut GtkButton, label: *const c_char);
    pub(super) fn gtk_check_button_get_active(check_button: *mut GtkCheckButton) -> c_int;
    pub(super) fn gtk_check_button_new_with_label(label: *const c_char) -> *mut GtkWidget;
    pub(super) fn gtk_check_button_set_active(check_button: *mut GtkCheckButton, setting: c_int);
    pub(super) fn gtk_editable_get_text(editable: *mut GtkEditable) -> *const c_char;
    pub(super) fn gtk_editable_select_region(
        editable: *mut GtkEditable,
        start_pos: c_int,
        end_pos: c_int,
    );
    pub(super) fn gtk_editable_set_text(editable: *mut GtkEditable, text: *const c_char);
    pub(super) fn gtk_entry_new() -> *mut GtkWidget;
    pub(super) fn gtk_entry_set_placeholder_text(entry: *mut GtkEntry, text: *const c_char);
    pub(super) fn gtk_entry_set_visibility(entry: *mut GtkEntry, visible: c_int);
    pub(super) fn gtk_label_new(str: *const c_char) -> *mut GtkWidget;
    pub(super) fn gtk_label_set_markup(label: *mut GtkLabel, str: *const c_char);
    pub(super) fn gtk_label_set_text(label: *mut GtkLabel, str: *const c_char);
    pub(super) fn gtk_label_set_use_markup(label: *mut GtkLabel, setting: c_int);
    pub(super) fn gtk_label_set_wrap(label: *mut GtkLabel, wrap: c_int);
    pub(super) fn gtk_label_set_xalign(label: *mut GtkLabel, xalign: c_double);
    pub(super) fn gtk_list_box_append(box_: *mut GtkListBox, child: *mut GtkWidget);
    pub(super) fn gtk_list_box_new() -> *mut GtkWidget;
    pub(super) fn gtk_list_box_row_new() -> *mut GtkWidget;
    pub(super) fn gtk_list_box_row_set_child(row: *mut GtkListBoxRow, child: *mut GtkWidget);
    pub(super) fn gtk_list_box_select_row(box_: *mut GtkListBox, row: *mut GtkListBoxRow);
    pub(super) fn gtk_list_box_set_selection_mode(box_: *mut GtkListBox, mode: c_int);
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
    pub(super) fn gtk_separator_new(orientation: c_int) -> *mut GtkWidget;
    pub(super) fn gtk_stack_add_named(
        stack: *mut GtkStack,
        child: *mut GtkWidget,
        name: *const c_char,
    ) -> *mut c_void;
    pub(super) fn gtk_stack_new() -> *mut GtkWidget;
    pub(super) fn gtk_stack_set_visible_child_name(stack: *mut GtkStack, name: *const c_char);
    pub(super) fn gtk_uri_launcher_launch(
        self_: *mut GtkUriLauncher,
        parent: *mut GtkWindow,
        cancellable: *mut GCancellable,
        callback: Option<GAsyncReadyCallback>,
        user_data: *mut c_void,
    );
    pub(super) fn gtk_uri_launcher_new(uri: *const c_char) -> *mut GtkUriLauncher;
    pub(super) fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    pub(super) fn gtk_widget_get_first_child(widget: *mut GtkWidget) -> *mut GtkWidget;
    pub(super) fn gtk_widget_get_next_sibling(widget: *mut GtkWidget) -> *mut GtkWidget;
    pub(super) fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    pub(super) fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    pub(super) fn gtk_widget_set_sensitive(widget: *mut GtkWidget, sensitive: c_int);
    pub(super) fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    pub(super) fn gtk_widget_set_tooltip_text(widget: *mut GtkWidget, text: *const c_char);
    pub(super) fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
    pub(super) fn gtk_widget_set_visible(widget: *mut GtkWidget, visible: c_int);
    pub(super) fn gtk_window_close(window: *mut GtkWindow);
    pub(super) fn gtk_window_new() -> *mut GtkWidget;
    pub(super) fn gtk_window_present(window: *mut GtkWindow);
    pub(super) fn gtk_window_set_child(window: *mut GtkWindow, child: *mut GtkWidget);
    pub(super) fn gtk_window_set_default_size(window: *mut GtkWindow, width: c_int, height: c_int);
    pub(super) fn gtk_window_set_focus(window: *mut GtkWindow, focus: *mut GtkWidget);
    pub(super) fn gtk_window_set_modal(window: *mut GtkWindow, modal: c_int);
    pub(super) fn gtk_window_set_title(window: *mut GtkWindow, title: *const c_char);
    pub(super) fn gtk_window_set_transient_for(window: *mut GtkWindow, parent: *mut GtkWindow);

}
