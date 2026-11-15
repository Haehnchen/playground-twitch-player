use super::{FALSE, GClosureNotify, GDestroyNotify, GSourceFunc, GType, TRUE};
use crate::player::session::MpvHandle;
use crate::twitch::stream_info::{GCancellable, GError, GPtrArray};
use std::ffi::{c_char, c_double, c_int, c_uint, c_ulonglong, c_void};
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
pub(super) const MPV_EVENT_NONE: c_int = 0;
pub(super) const MPV_EVENT_SHUTDOWN: c_int = 1;
pub(super) const MPV_EVENT_LOG_MESSAGE: c_int = 2;
pub(super) const MPV_EVENT_START_FILE: c_int = 6;
pub(super) const MPV_EVENT_END_FILE: c_int = 7;
pub(super) const MPV_EVENT_FILE_LOADED: c_int = 8;
pub(super) const MPV_EVENT_VIDEO_RECONFIG: c_int = 17;
pub(super) const MPV_END_FILE_REASON_EOF: c_int = 0;
pub(super) const MPV_END_FILE_REASON_ERROR: c_int = 4;
pub(super) const MPV_RENDER_PARAM_INVALID: c_int = 0;
pub(super) const MPV_RENDER_PARAM_API_TYPE: c_int = 1;
pub(super) const MPV_RENDER_PARAM_OPENGL_INIT_PARAMS: c_int = 2;
pub(super) const MPV_RENDER_PARAM_OPENGL_FBO: c_int = 3;
pub(super) const MPV_RENDER_PARAM_FLIP_Y: c_int = 4;
pub(super) const MPV_RENDER_UPDATE_FRAME: u64 = 1 << 0;
pub(super) const PANGO_ELLIPSIZE_END: c_int = 3;

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
pub(super) struct GdkGLContext {
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
pub(super) struct GtkBox {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkButton {
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
pub(super) struct GtkEventControllerLegacy {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkEventControllerMotion {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkEventControllerScroll {
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
pub(super) struct GtkGLArea {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkGrid {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkGridLayoutChild {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkLabel {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkLayoutChild {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkLayoutManager {
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
pub(super) struct GtkPaned {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkPopover {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkRange {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkScale {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct GtkStyleProvider {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWindow {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct MpvRenderContext {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct MpvEvent {
    pub(super) event_id: c_int,
    pub(super) error: c_int,
    pub(super) reply_userdata: u64,
    pub(super) data: *mut c_void,
}

#[repr(C)]
pub(super) struct MpvEventLogMessage {
    pub(super) prefix: *const c_char,
    pub(super) level: *const c_char,
    pub(super) text: *const c_char,
    pub(super) log_level: c_int,
}

#[repr(C)]
pub(super) struct MpvEventEndFile {
    pub(super) reason: c_int,
    pub(super) error: c_int,
    pub(super) playlist_entry_id: i64,
    pub(super) playlist_insert_id: i64,
    pub(super) playlist_insert_num_entries: c_int,
}

#[repr(C)]
pub(super) struct MpvOpenGLInitParams {
    pub(super) get_proc_address:
        Option<unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void>,
    pub(super) get_proc_address_ctx: *mut c_void,
}

#[repr(C)]
pub(super) struct MpvOpenGLFbo {
    pub(super) fbo: c_int,
    pub(super) w: c_int,
    pub(super) h: c_int,
    pub(super) internal_format: c_int,
}

#[repr(C)]
pub(super) struct MpvRenderParam {
    pub(super) type_: c_int,
    pub(super) data: *mut c_void,
}

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
    pub(super) fn g_malloc0(n_bytes: usize) -> *mut c_void;
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
    pub(super) fn g_ptr_array_unref(array: *mut GPtrArray);
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
    pub(super) fn gtk_widget_add_controller(widget: *mut GtkWidget, controller: *mut c_void);
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

    pub(super) fn mpv_error_string(error: c_int) -> *const c_char;
    pub(super) fn mpv_render_context_create(
        res: *mut *mut MpvRenderContext,
        mpv: *mut MpvHandle,
        params: *mut MpvRenderParam,
    ) -> c_int;
    pub(super) fn mpv_render_context_free(ctx: *mut MpvRenderContext);
    pub(super) fn mpv_render_context_render(
        ctx: *mut MpvRenderContext,
        params: *mut MpvRenderParam,
    ) -> c_int;
    pub(super) fn mpv_render_context_set_update_callback(
        ctx: *mut MpvRenderContext,
        callback: Option<unsafe extern "C" fn(*mut c_void)>,
        callback_ctx: *mut c_void,
    );
    pub(super) fn mpv_render_context_update(ctx: *mut MpvRenderContext) -> c_ulonglong;
    pub(super) fn mpv_wait_event(ctx: *mut MpvHandle, timeout: c_double) -> *mut MpvEvent;

}
