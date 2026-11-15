use super::{FALSE, GClosureNotify, GDestroyNotify, GSourceFunc, GType};
use crate::settings::GError;
use std::ffi::{c_char, c_double, c_int, c_uint, c_void};
pub(super) const G_PRIORITY_DEFAULT: c_int = 0;
pub(super) const G_SOURCE_REMOVE: c_int = 0;
pub(super) const G_SOURCE_CONTINUE: c_int = 1;
pub(super) const G_APPLICATION_NON_UNIQUE: c_int = 1 << 5;
pub(super) const G_LOG_LEVEL_DEBUG: c_int = 1 << 7;
pub(super) const GTK_ALIGN_FILL: c_int = 0;
pub(super) const GTK_ALIGN_START: c_int = 1;
pub(super) const GTK_ALIGN_END: c_int = 2;
pub(super) const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
pub(super) const GTK_ORIENTATION_VERTICAL: c_int = 1;
pub(super) const GTK_PHASE_CAPTURE: c_int = 1;
pub(super) const GTK_POS_BOTTOM: c_int = 3;
pub(super) const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: c_uint = 600;
pub(super) const GDK_BUTTON_PRIMARY: c_int = 1;
pub(super) const GDK_EVENT_PROPAGATE: c_int = FALSE;
pub(super) const GDK_SURFACE_EDGE_NORTH_WEST: c_int = 0;
pub(super) const GDK_SURFACE_EDGE_NORTH: c_int = 1;
pub(super) const GDK_SURFACE_EDGE_NORTH_EAST: c_int = 2;
pub(super) const GDK_SURFACE_EDGE_WEST: c_int = 3;
pub(super) const GDK_SURFACE_EDGE_EAST: c_int = 4;
pub(super) const GDK_SURFACE_EDGE_SOUTH_WEST: c_int = 5;
pub(super) const GDK_SURFACE_EDGE_SOUTH: c_int = 6;
pub(super) const GDK_SURFACE_EDGE_SOUTH_EAST: c_int = 7;

#[repr(C)]
pub(super) struct GdkDevice {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GdkDisplay {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GdkEvent {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GdkEventSequence {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GdkSurface {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GdkToplevel {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GObject {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GParamSpec {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GSource {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GTypeInstance {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkApplication {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkBox {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkButton {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkLabel {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkPopover {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkCssProvider {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkEventController {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkEventControllerKey {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkEventControllerMotion {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkGesture {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkGestureClick {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkGestureSingle {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkIconTheme {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkNative {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkOverlay {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkStyleProvider {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkWidget {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkWindow {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub(super) fn g_application_run(
        application: *mut c_void,
        argc: c_int,
        argv: *mut *mut c_char,
    ) -> c_int;
    pub(super) fn g_ascii_strcasecmp(str1: *const c_char, str2: *const c_char) -> c_int;
    pub(super) fn g_build_filename(first_element: *const c_char, ...) -> *mut c_char;
    pub(super) fn g_canonicalize_filename(
        filename: *const c_char,
        relative_to: *const c_char,
    ) -> *mut c_char;
    pub(super) fn g_clear_error(error: *mut *mut GError);
    pub(super) fn g_file_read_link(filename: *const c_char, error: *mut *mut GError)
    -> *mut c_char;
    pub(super) fn g_file_set_contents(
        filename: *const c_char,
        contents: *const c_char,
        length: isize,
        error: *mut *mut GError,
    ) -> c_int;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_get_current_dir() -> *mut c_char;
    pub(super) fn g_get_user_data_dir() -> *const c_char;
    pub(super) fn g_log(log_domain: *const c_char, log_level: c_int, format: *const c_char, ...);
    pub(super) fn g_main_context_find_source_by_id(
        context: *mut c_void,
        source_id: c_uint,
    ) -> *mut GSource;
    pub(super) fn g_mkdir_with_parents(pathname: *const c_char, mode: c_int) -> c_int;
    pub(super) fn g_object_add_weak_pointer(
        object: *mut GObject,
        weak_pointer_location: *mut *mut c_void,
    );
    pub(super) fn g_object_get_data(object: *mut GObject, key: *const c_char) -> *mut c_void;
    pub(super) fn g_object_remove_weak_pointer(
        object: *mut GObject,
        weak_pointer_location: *mut *mut c_void,
    );
    pub(super) fn g_object_set_data(object: *mut GObject, key: *const c_char, data: *mut c_void);
    pub(super) fn g_object_set_data_full(
        object: *mut GObject,
        key: *const c_char,
        data: *mut c_void,
        destroy: Option<GDestroyNotify>,
    );
    pub(super) fn g_object_unref(object: *mut c_void);
    pub(super) fn g_path_is_absolute(file_name: *const c_char) -> c_int;
    pub(super) fn g_set_application_name(application_name: *const c_char);
    pub(super) fn g_set_prgname(prgname: *const c_char);
    pub(super) fn g_setenv(
        variable: *const c_char,
        value: *const c_char,
        overwrite: c_int,
    ) -> c_int;
    pub(super) fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *const c_void,
        data: *mut c_void,
        destroy_data: Option<GClosureNotify>,
        connect_flags: c_int,
    ) -> usize;
    pub(super) fn g_source_destroy(source: *mut GSource);
    pub(super) fn g_strcmp0(str1: *const c_char, str2: *const c_char) -> c_int;
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;
    pub(super) fn g_strdup_printf(format: *const c_char, ...) -> *mut c_char;
    pub(super) fn g_timeout_add_full(
        priority: c_int,
        interval: c_uint,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
    pub(super) fn g_type_check_instance_is_a(
        instance: *mut GTypeInstance,
        iface_type: GType,
    ) -> c_int;

    pub(super) fn gdk_display_get_default() -> *mut GdkDisplay;
    pub(super) fn gdk_event_get_device(event: *mut GdkEvent) -> *mut GdkDevice;
    pub(super) fn gdk_event_get_position(
        event: *mut GdkEvent,
        x: *mut c_double,
        y: *mut c_double,
    ) -> c_int;
    pub(super) fn gdk_event_get_time(event: *mut GdkEvent) -> c_uint;
    pub(super) fn gdk_toplevel_begin_resize(
        toplevel: *mut GdkToplevel,
        edge: c_int,
        device: *mut GdkDevice,
        button: c_int,
        x: c_double,
        y: c_double,
        timestamp: c_uint,
    );
    pub(super) fn gdk_toplevel_get_type() -> GType;

    pub(super) fn gtk_application_new(
        application_id: *const c_char,
        flags: c_int,
    ) -> *mut GtkApplication;
    pub(super) fn gtk_application_window_new(application: *mut GtkApplication) -> *mut GtkWidget;
    pub(super) fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    pub(super) fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    pub(super) fn gtk_button_new() -> *mut GtkWidget;
    pub(super) fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    pub(super) fn gtk_css_provider_load_from_string(
        css_provider: *mut GtkCssProvider,
        string: *const c_char,
    );
    pub(super) fn gtk_css_provider_new() -> *mut GtkCssProvider;
    pub(super) fn gtk_event_controller_key_new() -> *mut GtkEventController;
    pub(super) fn gtk_event_controller_motion_new() -> *mut GtkEventController;
    pub(super) fn gtk_event_controller_set_propagation_phase(
        controller: *mut GtkEventController,
        phase: c_int,
    );
    pub(super) fn gtk_gesture_click_new() -> *mut GtkGesture;
    pub(super) fn gtk_gesture_get_last_event(
        gesture: *mut GtkGesture,
        sequence: *mut GdkEventSequence,
    ) -> *mut GdkEvent;
    pub(super) fn gtk_gesture_get_last_updated_sequence(
        gesture: *mut GtkGesture,
    ) -> *mut GdkEventSequence;
    pub(super) fn gtk_gesture_single_set_button(gesture: *mut GtkGestureSingle, button: c_uint);
    pub(super) fn gtk_icon_theme_add_search_path(
        icon_theme: *mut GtkIconTheme,
        path: *const c_char,
    );
    pub(super) fn gtk_icon_theme_get_for_display(display: *mut GdkDisplay) -> *mut GtkIconTheme;
    pub(super) fn gtk_label_new(str: *const c_char) -> *mut GtkWidget;
    pub(super) fn gtk_label_set_xalign(label: *mut GtkLabel, xalign: f32);
    pub(super) fn gtk_native_get_surface(self_: *mut GtkNative) -> *mut GdkSurface;
    pub(super) fn gtk_overlay_add_overlay(overlay: *mut GtkOverlay, widget: *mut GtkWidget);
    pub(super) fn gtk_overlay_get_type() -> GType;
    pub(super) fn gtk_overlay_new() -> *mut GtkWidget;
    pub(super) fn gtk_overlay_set_child(overlay: *mut GtkOverlay, child: *mut GtkWidget);
    pub(super) fn gtk_popover_new() -> *mut GtkWidget;
    pub(super) fn gtk_popover_popdown(popover: *mut GtkPopover);
    pub(super) fn gtk_popover_popup(popover: *mut GtkPopover);
    pub(super) fn gtk_popover_set_child(popover: *mut GtkPopover, child: *mut GtkWidget);
    pub(super) fn gtk_popover_set_has_arrow(popover: *mut GtkPopover, has_arrow: c_int);
    pub(super) fn gtk_popover_set_position(popover: *mut GtkPopover, position: c_int);
    pub(super) fn gtk_separator_new(orientation: c_int) -> *mut GtkWidget;
    pub(super) fn gtk_style_context_add_provider_for_display(
        display: *mut GdkDisplay,
        provider: *mut GtkStyleProvider,
        priority: c_uint,
    );
    pub(super) fn gtk_widget_add_controller(widget: *mut GtkWidget, controller: *mut c_void);
    pub(super) fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    pub(super) fn gtk_widget_get_height(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_get_native(widget: *mut GtkWidget) -> *mut GtkNative;
    pub(super) fn gtk_widget_get_width(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_set_parent(widget: *mut GtkWidget, parent: *mut GtkWidget);
    pub(super) fn gtk_widget_set_cursor_from_name(widget: *mut GtkWidget, name: *const c_char);
    pub(super) fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    pub(super) fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    pub(super) fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    pub(super) fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
    pub(super) fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
    pub(super) fn gtk_widget_set_visible(widget: *mut GtkWidget, visible: c_int);
    pub(super) fn gtk_widget_unparent(widget: *mut GtkWidget);
    pub(super) fn gtk_window_close(window: *mut GtkWindow);
    pub(super) fn gtk_window_fullscreen(window: *mut GtkWindow);
    pub(super) fn gtk_window_get_type() -> GType;
    pub(super) fn gtk_window_is_fullscreen(window: *mut GtkWindow) -> c_int;
    pub(super) fn gtk_window_is_maximized(window: *mut GtkWindow) -> c_int;
    pub(super) fn gtk_window_maximize(window: *mut GtkWindow);
    pub(super) fn gtk_window_minimize(window: *mut GtkWindow);
    pub(super) fn gtk_window_present(window: *mut GtkWindow);
    pub(super) fn gtk_window_set_child(window: *mut GtkWindow, child: *mut GtkWidget);
    pub(super) fn gtk_window_set_decorated(window: *mut GtkWindow, setting: c_int);
    pub(super) fn gtk_window_set_default_icon_name(name: *const c_char);
    pub(super) fn gtk_window_set_default_size(window: *mut GtkWindow, width: c_int, height: c_int);
    pub(super) fn gtk_window_set_icon_name(window: *mut GtkWindow, name: *const c_char);
    pub(super) fn gtk_window_set_title(window: *mut GtkWindow, title: *const c_char);
    pub(super) fn gtk_window_unfullscreen(window: *mut GtkWindow);

}
