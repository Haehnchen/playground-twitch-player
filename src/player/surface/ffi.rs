use super::{FALSE, GClosureNotify, GDestroyNotify, GSourceFunc, GType, TRUE};
use crate::twitch::stream_info::{GCancellable, GError};
use std::ffi::{c_char, c_double, c_int, c_uint, c_void};
pub(super) const MPV_MAINLOOP_PRIORITY: c_int = -100;
pub(super) const G_PRIORITY_DEFAULT: c_int = 0;
pub(super) const G_SOURCE_REMOVE: c_int = 0;
pub(super) const G_SOURCE_CONTINUE: c_int = 1;
pub(super) const G_IO_ERROR_CANCELLED: c_int = 19;
pub(super) const G_LOG_LEVEL_DEBUG: c_int = 1 << 7;
pub(super) const G_LOG_LEVEL_WARNING: c_int = 1 << 4;
pub(super) const GTK_ALIGN_FILL: c_int = 0;
pub(super) const GTK_ALIGN_START: c_int = 1;
pub(super) const GTK_ALIGN_END: c_int = 2;
pub(super) const GTK_ALIGN_CENTER: c_int = 3;
pub(super) const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
pub(super) const GTK_ORIENTATION_VERTICAL: c_int = 1;
pub(super) const GTK_PHASE_CAPTURE: c_int = 1;
pub(super) const GTK_PHASE_BUBBLE: c_int = 2;
pub(super) const GTK_PHASE_TARGET: c_int = 3;
pub(super) const GTK_EVENT_CONTROLLER_SCROLL_VERTICAL: c_int = 1;
pub(super) const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: c_uint = 600;
pub(super) const GDK_BUTTON_PRIMARY: c_uint = 1;
pub(super) const GDK_BUTTON_SECONDARY: c_uint = 3;
pub(super) const GDK_EVENT_PROPAGATE: c_int = FALSE;
pub(super) const GDK_EVENT_STOP: c_int = TRUE;
pub(super) const GDK_MOTION_NOTIFY: c_int = 1;
pub(super) const GDK_BUTTON_PRESS: c_int = 2;
pub(super) const GDK_BUTTON_RELEASE: c_int = 3;
pub(super) const GDK_BUTTON1_MASK: c_uint = 1 << 8;
pub(super) const GDK_CONTROL_MASK: c_uint = 1 << 2;
pub(super) const GDK_KEY_M_UPPER: c_uint = 0x04d;
pub(super) const GDK_KEY_M_LOWER: c_uint = 0x06d;
pub(super) const GL_COLOR_BUFFER_BIT: c_uint = 0x0000_4000;
pub(super) const GL_FRAMEBUFFER_BINDING: c_uint = 0x8CA6;
pub(super) const PANGO_ELLIPSIZE_END: c_int = 3;

pub(super) use gtk4::gdk::ffi::{
    GdkDevice, GdkDisplay, GdkEvent, GdkGLContext, GdkSurface, GdkToplevel,
};

pub(super) use gtk4::glib::gobject_ffi::{GObject, GParamSpec, GTypeInstance};

pub(super) use gtk4::glib::ffi::GSource;

pub(super) use gtk4::ffi::{
    GtkBox, GtkButton, GtkCssProvider, GtkEventController, GtkEventControllerLegacy,
    GtkEventControllerMotion, GtkEventControllerScroll, GtkGLArea, GtkGesture, GtkGestureClick,
    GtkGestureSingle, GtkGrid, GtkGridLayoutChild, GtkLabel, GtkLayoutChild, GtkLayoutManager,
    GtkNative, GtkOverlay, GtkPaned, GtkPopover, GtkRange, GtkScale, GtkStyleProvider,
    gtk_widget_add_controller,
};

pub use gtk4::ffi::{GtkWidget, GtkWindow};

unsafe extern "C" {
    pub(super) static epoxy_eglGetProcAddress: unsafe extern "C" fn(*const c_char) -> *mut c_void;
    pub(super) static epoxy_glClear: unsafe extern "C" fn(c_uint);
    pub(super) static epoxy_glClearColor: unsafe extern "C" fn(f32, f32, f32, f32);
    pub(super) static epoxy_glGetIntegerv: unsafe extern "C" fn(c_uint, *mut c_int);

    pub(super) fn g_atomic_int_compare_and_exchange(
        atomic: *mut c_int,
        oldval: c_int,
        newval: c_int,
    ) -> c_int;
    pub(super) fn g_atomic_int_set(atomic: *mut c_int, newval: c_int);
    pub(super) fn g_cancellable_cancel(cancellable: *mut GCancellable);
    pub(super) fn g_cancellable_new() -> *mut GCancellable;
    pub(super) fn g_clear_error(error: *mut *mut GError);
    pub(super) fn g_error_matches(error: *const GError, domain: c_uint, code: c_int) -> c_int;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_io_error_quark() -> c_uint;
    pub(super) fn g_idle_add_full(
        priority: c_int,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
    pub(super) fn g_log(log_domain: *const c_char, log_level: c_int, format: *const c_char, ...);
    pub(super) fn g_main_context_find_source_by_id(
        context: *mut c_void,
        source_id: c_uint,
    ) -> *mut GSource;
    pub(super) fn g_object_add_weak_pointer(
        object: *mut GObject,
        weak_pointer_location: *mut *mut c_void,
    );
    pub(super) fn g_object_get_data(object: *mut GObject, key: *const c_char) -> *mut c_void;
    pub(super) fn g_object_remove_weak_pointer(
        object: *mut GObject,
        weak_pointer_location: *mut *mut c_void,
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
    pub(super) fn g_source_destroy(source: *mut GSource);
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;
    pub(super) fn g_strdup_printf(format: *const c_char, ...) -> *mut c_char;
    pub(super) fn g_timeout_add_full(
        priority: c_int,
        interval: c_uint,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
    pub(super) fn g_timeout_add_seconds_full(
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

    pub(super) fn gdk_button_event_get_button(event: *mut GdkEvent) -> c_uint;
    pub(super) fn gdk_display_get_default() -> *mut GdkDisplay;
    pub(super) fn gdk_event_get_device(event: *mut GdkEvent) -> *mut GdkDevice;
    pub(super) fn gdk_event_get_event_type(event: *mut GdkEvent) -> c_int;
    pub(super) fn gdk_event_get_modifier_state(event: *mut GdkEvent) -> c_uint;
    pub(super) fn gdk_event_get_position(
        event: *mut GdkEvent,
        x: *mut c_double,
        y: *mut c_double,
    ) -> c_int;
    pub(super) fn gdk_event_get_time(event: *mut GdkEvent) -> c_uint;
    pub(super) fn gdk_toplevel_begin_move(
        toplevel: *mut GdkToplevel,
        device: *mut GdkDevice,
        button: c_uint,
        x: c_double,
        y: c_double,
        timestamp: c_uint,
    );
    pub(super) fn gdk_toplevel_get_type() -> GType;

    pub(super) fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    pub(super) fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    pub(super) fn gtk_button_new() -> *mut GtkWidget;
    pub(super) fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    pub(super) fn gtk_css_provider_load_from_string(
        css_provider: *mut GtkCssProvider,
        string: *const c_char,
    );
    pub(super) fn gtk_css_provider_new() -> *mut GtkCssProvider;
    pub(super) fn gtk_event_controller_legacy_new() -> *mut GtkEventController;
    pub(super) fn gtk_event_controller_motion_new() -> *mut GtkEventController;
    pub(super) fn gtk_event_controller_scroll_new(flags: c_int) -> *mut GtkEventController;
    pub(super) fn gtk_event_controller_set_propagation_phase(
        controller: *mut GtkEventController,
        phase: c_int,
    );
    pub(super) fn gtk_gesture_click_new() -> *mut GtkGesture;
    pub(super) fn gtk_gesture_single_set_button(gesture: *mut GtkGestureSingle, button: c_uint);
    pub(super) fn gtk_gl_area_attach_buffers(area: *mut GtkGLArea);
    pub(super) fn gtk_gl_area_get_error(area: *mut GtkGLArea) -> *mut GError;
    pub(super) fn gtk_gl_area_make_current(area: *mut GtkGLArea);
    pub(super) fn gtk_gl_area_new() -> *mut GtkWidget;
    pub(super) fn gtk_gl_area_queue_render(area: *mut GtkGLArea);
    pub(super) fn gtk_gl_area_set_auto_render(area: *mut GtkGLArea, auto_render: c_int);
    pub(super) fn gtk_grid_attach(
        grid: *mut GtkGrid,
        child: *mut GtkWidget,
        column: c_int,
        row: c_int,
        width: c_int,
        height: c_int,
    );
    pub(super) fn gtk_grid_layout_child_get_type() -> GType;
    pub(super) fn gtk_grid_layout_child_set_column(child: *mut GtkGridLayoutChild, column: c_int);
    pub(super) fn gtk_grid_layout_child_set_column_span(
        child: *mut GtkGridLayoutChild,
        span: c_int,
    );
    pub(super) fn gtk_grid_layout_child_set_row(child: *mut GtkGridLayoutChild, row: c_int);
    pub(super) fn gtk_grid_layout_child_set_row_span(child: *mut GtkGridLayoutChild, span: c_int);
    pub(super) fn gtk_grid_new() -> *mut GtkWidget;
    pub(super) fn gtk_grid_set_column_homogeneous(grid: *mut GtkGrid, homogeneous: c_int);
    pub(super) fn gtk_grid_set_row_homogeneous(grid: *mut GtkGrid, homogeneous: c_int);
    pub(super) fn gtk_label_new(str: *const c_char) -> *mut GtkWidget;
    pub(super) fn gtk_label_set_ellipsize(label: *mut GtkLabel, mode: c_int);
    pub(super) fn gtk_label_set_text(label: *mut GtkLabel, str: *const c_char);
    pub(super) fn gtk_label_set_wrap(label: *mut GtkLabel, wrap: c_int);
    pub(super) fn gtk_label_set_xalign(label: *mut GtkLabel, xalign: f32);
    pub(super) fn gtk_layout_manager_get_layout_child(
        manager: *mut GtkLayoutManager,
        child: *mut GtkWidget,
    ) -> *mut GtkLayoutChild;
    pub(super) fn gtk_native_get_surface(self_: *mut GtkNative) -> *mut GdkSurface;
    pub(super) fn gtk_overlay_add_overlay(overlay: *mut GtkOverlay, widget: *mut GtkWidget);
    pub(super) fn gtk_overlay_new() -> *mut GtkWidget;
    pub(super) fn gtk_overlay_set_child(overlay: *mut GtkOverlay, child: *mut GtkWidget);
    pub(super) fn gtk_paned_get_position(paned: *mut GtkPaned) -> c_int;
    pub(super) fn gtk_paned_new(orientation: c_int) -> *mut GtkWidget;
    pub(super) fn gtk_paned_set_end_child(paned: *mut GtkPaned, child: *mut GtkWidget);
    pub(super) fn gtk_paned_set_position(paned: *mut GtkPaned, position: c_int);
    pub(super) fn gtk_paned_set_resize_end_child(paned: *mut GtkPaned, resize: c_int);
    pub(super) fn gtk_paned_set_resize_start_child(paned: *mut GtkPaned, resize: c_int);
    pub(super) fn gtk_paned_set_shrink_end_child(paned: *mut GtkPaned, resize: c_int);
    pub(super) fn gtk_paned_set_shrink_start_child(paned: *mut GtkPaned, resize: c_int);
    pub(super) fn gtk_paned_set_start_child(paned: *mut GtkPaned, child: *mut GtkWidget);
    pub(super) fn gtk_paned_set_wide_handle(paned: *mut GtkPaned, wide: c_int);
    pub(super) fn gtk_popover_popdown(popover: *mut GtkPopover);
    pub(super) fn gtk_popover_popup(popover: *mut GtkPopover);
    pub(super) fn gtk_range_set_value(range: *mut GtkRange, value: c_double);
    pub(super) fn gtk_scale_new_with_range(
        orientation: c_int,
        min: c_double,
        max: c_double,
        step: c_double,
    ) -> *mut GtkWidget;
    pub(super) fn gtk_scale_set_draw_value(scale: *mut GtkScale, draw_value: c_int);
    pub(super) fn gtk_style_context_add_provider_for_display(
        display: *mut GdkDisplay,
        provider: *mut GtkStyleProvider,
        priority: c_uint,
    );
    pub(super) fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    pub(super) fn gtk_widget_set_can_target(widget: *mut GtkWidget, can_target: c_int);
    pub(super) fn gtk_widget_get_height(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_get_layout_manager(widget: *mut GtkWidget) -> *mut GtkLayoutManager;
    pub(super) fn gtk_widget_get_native(widget: *mut GtkWidget) -> *mut GtkNative;
    pub(super) fn gtk_widget_get_realized(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_get_scale_factor(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_get_sensitive(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_get_visible(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_get_width(widget: *mut GtkWidget) -> c_int;
    pub(super) fn gtk_widget_remove_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    pub(super) fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    pub(super) fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    pub(super) fn gtk_widget_set_margin_end(widget: *mut GtkWidget, margin: c_int);
    pub(super) fn gtk_widget_set_sensitive(widget: *mut GtkWidget, sensitive: c_int);
    pub(super) fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    pub(super) fn gtk_widget_set_tooltip_text(widget: *mut GtkWidget, text: *const c_char);
    pub(super) fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
    pub(super) fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
    pub(super) fn gtk_widget_set_visible(widget: *mut GtkWidget, visible: c_int);
    pub(super) fn gtk_widget_unparent(widget: *mut GtkWidget);

}
