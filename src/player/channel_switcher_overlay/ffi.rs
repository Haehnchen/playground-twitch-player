//! GTK, GDK, and GLib bindings.

use std::ffi::{c_char, c_int, c_uint, c_void};

pub(super) const FALSE: c_int = 0;
pub(super) const TRUE: c_int = 1;
pub(super) const G_SOURCE_REMOVE: c_int = 0;
pub(super) const G_SOURCE_CONTINUE: c_int = 1;

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

pub(super) const G_PRIORITY_DEFAULT: c_int = 0;
pub(super) const PANGO_ELLIPSIZE_END: c_int = 3;
pub(super) const PLAYER_WINDOW_ICON_CLOSE: c_int = 2;

pub use gtk4::graphene::ffi::graphene_point_t as GraphenePoint;

pub use gtk4::glib::ffi::GBytes;

pub use gtk4::gdk::ffi::{GdkDisplay, GdkPaintable, GdkTexture};

pub use gtk4::gdk_pixbuf::ffi::GdkPixbuf;

pub use gtk4::gio::ffi::{GFile, GInputStream, GTask};

pub use gtk4::glib::gobject_ffi::{GObject, GTypeInstance};

pub use gtk4::ffi::{
    GtkBox, GtkButton, GtkCssProvider, GtkEditable, GtkEntry, GtkGesture, GtkGestureClick,
    GtkGestureSingle, GtkGrid, GtkLabel, GtkOverlay, GtkPicture, GtkRoot, GtkScrolledWindow,
    GtkSearchEntry, GtkStyleProvider, GtkWidget,
};

pub(super) type GDestroyNotify = unsafe extern "C" fn(*mut c_void);
pub(super) type GClosureNotify = unsafe extern "C" fn(*mut c_void, *mut c_void);
pub(super) type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;

unsafe extern "C" {
    pub(super) fn g_ascii_strcasecmp(str1: *const c_char, str2: *const c_char) -> c_int;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_get_monotonic_time() -> i64;
    #[cfg(test)]
    pub(super) fn g_idle_add_full(
        priority: c_int,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
    pub(super) fn g_object_add_weak_pointer(
        object: *mut GObject,
        weak_pointer_location: *mut *mut c_void,
    );
    pub(super) fn g_object_get_data(object: *mut GObject, key: *const c_char) -> *mut c_void;
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
    pub(super) fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *const c_void,
        data: *mut c_void,
        destroy_data: Option<GClosureNotify>,
        connect_flags: c_int,
    ) -> usize;
    pub(super) fn g_source_remove(tag: c_uint) -> c_int;
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;
    pub(super) fn g_strdup_printf(format: *const c_char, ...) -> *mut c_char;
    pub(super) fn g_timeout_add_full(
        priority: c_int,
        interval: c_uint,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
    pub(super) fn g_utf8_casefold(str: *const c_char, len: isize) -> *mut c_char;

    pub(super) fn gdk_display_get_default() -> *mut GdkDisplay;

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

pub(super) use gtk4::ffi::gtk_widget_add_controller;
