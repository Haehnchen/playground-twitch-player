#![allow(clashing_extern_declarations)]

use std::ffi::{c_char, c_double, c_int, c_uint, c_ulonglong, c_void, CStr};
use std::mem;
use std::ptr;

use crate::channel_switcher_overlay::{
    channel_switcher_overlay_free, channel_switcher_overlay_is_visible,
    channel_switcher_overlay_new, channel_switcher_overlay_set_settings,
    channel_switcher_overlay_show_at, ChannelSwitcherOverlay,
};
use crate::player_footer::{
    player_footer_stream_info_free, player_footer_stream_info_get_widget,
    player_footer_stream_info_new, player_footer_stream_info_set, PlayerFooterStreamInfo,
};
use crate::player_icons::{
    player_plus_icon_new, player_refresh_icon_new, player_stream_settings_icon_new,
    player_tile_focus_icon_new, player_trash_icon_new,
};
use crate::player_motion::{player_motion_tracker_ignore_stationary, PlayerMotionTracker};
use crate::player_overlay_controls::player_overlay_button_new;
use crate::player_session::{
    player_session_free, player_session_get_channel, player_session_get_label,
    player_session_get_mpv, player_session_get_muted, player_session_get_volume,
    player_session_is_playing, player_session_is_ready, player_session_load_stream,
    player_session_new, player_session_reenable_video, player_session_set_hwdec_enabled,
    player_session_set_wakeup_callback, player_session_stop, player_session_toggle_stream_info,
    MpvHandle, PlayerSession,
};
use crate::player_stream_quality::{
    player_stream_quality_state_begin_fetch, player_stream_quality_state_cache_is_valid,
    player_stream_quality_state_cancel_fetch, player_stream_quality_state_clear,
    player_stream_quality_state_finish_fetch, player_stream_quality_state_mark_fetched,
    player_stream_quality_state_reset_selection, player_stream_quality_state_select,
    player_stream_quality_state_select_auto, PlayerStreamQualityState,
};
use crate::player_stream_settings::{
    player_stream_settings_popover_new, player_stream_settings_quality_list_populate,
};
use crate::player_style::{player_style_install_footer_css, player_style_install_overlay_css};
use crate::player_volume::{
    player_volume_apply_scroll, player_volume_mute_button_new, player_volume_set_muted,
    player_volume_sync_session_from_range, player_volume_toggle_muted,
    player_volume_update_mute_button,
};
use crate::settings::{app_settings_get_hwdec_enabled, AppSettings, AppSettingsChannel};
use crate::twitch_stream_info::{
    twitch_current_stream_free, twitch_stream_info_fetch_current_stream_async,
    twitch_stream_info_fetch_current_stream_finish,
    twitch_stream_info_fetch_stream_qualities_async,
    twitch_stream_info_fetch_stream_qualities_finish,
    twitch_stream_info_format_current_stream_metadata,
    twitch_stream_info_format_current_stream_title, GAsyncResult, GCancellable, GError, GPtrArray,
    TwitchStreamQuality,
};

macro_rules! cstr {
    ($value:literal) => {
        concat!($value, "\0").as_ptr() as *const c_char
    };
}

const MAX_TILES: usize = 4;
const MPV_MAINLOOP_PRIORITY: c_int = -100;
const STREAM_TITLE_REFRESH_SECONDS: c_uint = 3 * 60;
const STREAM_QUALITY_CACHE_SECONDS: c_uint = 2 * 60;
const GRID_CHANNEL_DROPDOWN_WIDTH: c_int = 119;
const GRID_VOLUME_SCALE_WIDTH: c_int = 82;

const FALSE: c_int = 0;
const TRUE: c_int = 1;
const G_SOURCE_REMOVE: c_int = 0;
const G_SOURCE_CONTINUE: c_int = 1;
const G_IO_ERROR_CANCELLED: c_int = 19;
const G_LOG_LEVEL_DEBUG: c_int = 1 << 7;
const G_LOG_LEVEL_WARNING: c_int = 1 << 4;
const G_LOG_DOMAIN: &[u8] = b"twitch-player-grid\0";

const GTK_ALIGN_FILL: c_int = 0;
const GTK_ALIGN_START: c_int = 1;
const GTK_ALIGN_END: c_int = 2;
const GTK_ALIGN_CENTER: c_int = 3;
const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
const GTK_ORIENTATION_VERTICAL: c_int = 1;
const GTK_PHASE_CAPTURE: c_int = 1;
const GTK_EVENT_CONTROLLER_SCROLL_VERTICAL: c_int = 1;
const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: c_uint = 600;

const GDK_BUTTON_PRIMARY: c_uint = 1;
const GDK_BUTTON_SECONDARY: c_uint = 3;
const GDK_EVENT_PROPAGATE: c_int = FALSE;
const GDK_EVENT_STOP: c_int = TRUE;
const GDK_MOTION_NOTIFY: c_int = 1;
const GDK_BUTTON_PRESS: c_int = 2;
const GDK_BUTTON_RELEASE: c_int = 3;
const GDK_BUTTON1_MASK: c_uint = 1 << 8;

const GL_COLOR_BUFFER_BIT: c_uint = 0x0000_4000;
const GL_FRAMEBUFFER_BINDING: c_uint = 0x8CA6;

const MPV_EVENT_NONE: c_int = 0;
const MPV_EVENT_SHUTDOWN: c_int = 1;
const MPV_EVENT_LOG_MESSAGE: c_int = 2;
const MPV_EVENT_START_FILE: c_int = 6;
const MPV_EVENT_END_FILE: c_int = 7;
const MPV_EVENT_FILE_LOADED: c_int = 8;
const MPV_EVENT_VIDEO_RECONFIG: c_int = 17;
const MPV_END_FILE_REASON_EOF: c_int = 0;
const MPV_END_FILE_REASON_ERROR: c_int = 4;
const MPV_RENDER_PARAM_INVALID: c_int = 0;
const MPV_RENDER_PARAM_API_TYPE: c_int = 1;
const MPV_RENDER_PARAM_OPENGL_INIT_PARAMS: c_int = 2;
const MPV_RENDER_PARAM_OPENGL_FBO: c_int = 3;
const MPV_RENDER_PARAM_FLIP_Y: c_int = 4;
const MPV_RENDER_UPDATE_FRAME: u64 = 1 << 0;

const PANGO_ELLIPSIZE_END: c_int = 3;
const PLAYER_TILE_FOCUS_ICON_EXPAND: c_int = 0;
const PLAYER_TILE_FOCUS_ICON_RESTORE: c_int = 1;
const PLAYER_EMPTY_STREAM_LABEL: *const c_char = cstr!("Select");
const PLAYER_STARTING_STREAM_STATUS: *const c_char = cstr!("Starting stream");
const PLAYER_VOLUME_MIN: c_double = 0.0;
const PLAYER_VOLUME_MAX: c_double = 130.0;

const GRID_CSS: &str = concat!(
    ".grid-root {",
    "  background: #050505;",
    "}",
    ".stream-grid {",
    "  background: #050505;",
    "}",
    ".tile-container {",
    "  background: #050505;",
    "  border: none;",
    "}",
    ".tile-left {",
    "  border-right: 1px solid rgba(255, 255, 255, 0.12);",
    "}",
    ".tile-top {",
    "  border-bottom: 1px solid rgba(255, 255, 255, 0.12);",
    "}",
    ".tile-footer {",
    "  background: rgba(0, 0, 0, 0.62);",
    "  color: white;",
    "  padding: 4px 6px;",
    "}",
    ".tile-footer .stream-info-labels {",
    "  margin-left: 2px;",
    "  margin-right: 2px;",
    "}",
    ".tile-footer button,",
    ".tile-footer menubutton,",
    ".tile-footer menubutton > button,",
    ".tile-footer popover,",
    ".tile-footer scale {",
    "  color: white;",
    "}",
    ".tile-footer button,",
    ".tile-footer menubutton > button {",
    "  background: rgba(30, 30, 30, 0.82);",
    "  color: white;",
    "  border-color: transparent;",
    "  outline-color: transparent;",
    "  box-shadow: none;",
    "  min-height: 0;",
    "}",
    ".tile-footer button:hover,",
    ".tile-footer menubutton > button:hover {",
    "  background: rgba(54, 54, 54, 0.90);",
    "}",
    ".channel-dropdown {",
    "  min-width: 119px;",
    "  min-height: 24px;",
    "}",
    ".channel-selector {",
    "  min-width: 119px;",
    "}",
    ".channel-dropdown,",
    ".channel-dropdown > button {",
    "  padding: 2px 8px;",
    "  min-height: 24px;",
    "}",
    ".channel-button-label {",
    "  color: white;",
    "  font-size: 13px;",
    "}",
    ".stream-title-label {",
    "  color: rgba(255, 255, 255, 0.88);",
    "  font-size: 12px;",
    "}",
    ".stream-metadata-label {",
    "  color: rgba(255, 255, 255, 0.76);",
    "  font-size: 11px;",
    "}",
    ".channel-popover contents {",
    "  background: rgba(28, 28, 28, 0.98);",
    "  padding: 0;",
    "  margin: 0;",
    "  border: none;",
    "  border-radius: 4px;",
    "  box-shadow: none;",
    "}",
    ".channel-popover {",
    "  padding: 0;",
    "  margin: 0;",
    "  border: none;",
    "  border-radius: 4px;",
    "  box-shadow: none;",
    "}",
    ".channel-menu {",
    "  background: rgba(28, 28, 28, 0.98);",
    "  padding: 2px 0;",
    "  margin: 0;",
    "}",
    ".channel-menu-item {",
    "  background: transparent;",
    "  color: white;",
    "  border-color: transparent;",
    "  outline-color: transparent;",
    "  box-shadow: none;",
    "  border-radius: 0;",
    "  margin: 0;",
    "  min-height: 0;",
    "  padding: 6px 10px;",
    "}",
    ".channel-menu-item box {",
    "  padding: 0;",
    "  margin: 0;",
    "}",
    ".channel-menu-item label {",
    "  color: white;",
    "  padding: 0;",
    "  margin: 0;",
    "}",
    ".channel-menu-item:hover {",
    "  background: rgba(74, 74, 74, 0.98);",
    "  color: white;",
    "}",
);

#[repr(C)]
struct GdkDevice {
    _private: [u8; 0],
}

#[repr(C)]
struct GdkDisplay {
    _private: [u8; 0],
}

#[repr(C)]
struct GdkEvent {
    _private: [u8; 0],
}

#[repr(C)]
struct GdkGLContext {
    _private: [u8; 0],
}

#[repr(C)]
struct GdkSurface {
    _private: [u8; 0],
}

#[repr(C)]
struct GdkToplevel {
    _private: [u8; 0],
}

#[repr(C)]
struct GObject {
    _private: [u8; 0],
}

#[repr(C)]
struct GSource {
    _private: [u8; 0],
}

#[repr(C)]
struct GTypeInstance {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkBox {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkButton {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkCssProvider {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkEventController {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkEventControllerLegacy {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkEventControllerMotion {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkEventControllerScroll {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkGesture {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkGestureClick {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkGestureSingle {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkGLArea {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkGrid {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkGridLayoutChild {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkLabel {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkLayoutChild {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkLayoutManager {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkNative {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkOverlay {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkPopover {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkRange {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkScale {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkStyleProvider {
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
struct MpvRenderContext {
    _private: [u8; 0],
}

struct StreamTile {
    app: *mut GridAppState,
    index: c_uint,
    label: *mut c_char,
    channel: *mut c_char,
    container: *mut GtkWidget,
    overlay: *mut GtkWidget,
    gl_area: *mut GtkWidget,
    footer: *mut GtkWidget,
    channel_combo: *mut GtkWidget,
    channel_label: *mut GtkWidget,
    channel_refresh_button: *mut GtkWidget,
    close_button: *mut GtkWidget,
    empty_label: *mut GtkWidget,
    focus_button: *mut GtkWidget,
    stream_info_button: *mut GtkWidget,
    mute_button: *mut GtkWidget,
    volume_scale: *mut GtkWidget,
    stream_settings_popover: *mut GtkWidget,
    quality_list_box: *mut GtkWidget,
    quality_status_label: *mut GtkWidget,
    channel_switcher: *mut ChannelSwitcherOverlay,
    session: *mut PlayerSession,
    title_cancel: *mut GCancellable,
    stream_info: *mut PlayerFooterStreamInfo,
    stream_quality: PlayerStreamQualityState,
    mpv_gl: *mut MpvRenderContext,
    last_render_width: c_int,
    last_render_height: c_int,
    render_queued: c_int,
    event_queued: c_int,
    render_warmup_source: c_uint,
    title_generation: c_uint,
    render_warmup_frames: c_int,
    owns_session: c_int,
    title_fetch_in_progress: c_int,
}

pub struct GridAppState {
    targets: [*mut c_char; MAX_TILES],
    target_count: c_uint,
    window: *mut GtkWidget,
    root_overlay: *mut GtkWidget,
    grid: *mut GtkWidget,
    grid_items: [*mut GtkWidget; MAX_TILES],
    top_controls: *mut GtkWidget,
    tiles: [StreamTile; MAX_TILES],
    primary_session: *mut PlayerSession,
    settings: *mut AppSettings,
    visible_footer_tile: *mut StreamTile,
    footer_hide_source: c_uint,
    title_refresh_source: c_uint,
    video_fullscreen_focus_source: c_uint,
    focused_tile: c_uint,
    video_fullscreen_pending_tile: c_uint,
    motion_tracker: PlayerMotionTracker,
    fullscreen_callback: GridPlayerFullscreenCallback,
    fullscreen_user_data: *mut c_void,
    settings_callback: GridPlayerSettingsCallback,
    settings_user_data: *mut c_void,
    move_press_x: c_double,
    move_press_y: c_double,
    move_pressed: c_int,
    closing: c_int,
    fullscreen: c_int,
    tile_focused: c_int,
    video_fullscreen_active: c_int,
    video_fullscreen_restore_app_fullscreen: c_int,
    video_fullscreen_restore_tile_focused: c_int,
    video_fullscreen_restore_focused_tile: c_uint,
    started: c_int,
}

struct StreamTitleCallbackData {
    tile: *mut StreamTile,
    generation: c_uint,
}

struct StreamQualityCallbackData {
    tile: *mut StreamTile,
    generation: c_uint,
}

#[repr(C)]
struct MpvEvent {
    event_id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

#[repr(C)]
struct MpvEventLogMessage {
    prefix: *const c_char,
    level: *const c_char,
    text: *const c_char,
    log_level: c_int,
}

#[repr(C)]
struct MpvEventEndFile {
    reason: c_int,
    error: c_int,
    playlist_entry_id: i64,
    playlist_insert_id: i64,
    playlist_insert_num_entries: c_int,
}

#[repr(C)]
struct MpvOpenGLInitParams {
    get_proc_address: Option<unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void>,
    get_proc_address_ctx: *mut c_void,
}

#[repr(C)]
struct MpvOpenGLFbo {
    fbo: c_int,
    w: c_int,
    h: c_int,
    internal_format: c_int,
}

#[repr(C)]
struct MpvRenderParam {
    type_: c_int,
    data: *mut c_void,
}

type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;
type GType = usize;
pub type GridPlayerFullscreenCallback = Option<unsafe extern "C" fn(*mut c_void)>;
pub type GridPlayerSettingsCallback = Option<unsafe extern "C" fn(*mut c_void)>;

unsafe extern "C" {
    static epoxy_eglGetProcAddress: unsafe extern "C" fn(*const c_char) -> *mut c_void;
    static epoxy_glClear: unsafe extern "C" fn(c_uint);
    static epoxy_glClearColor: unsafe extern "C" fn(f32, f32, f32, f32);
    static epoxy_glGetIntegerv: unsafe extern "C" fn(c_uint, *mut c_int);

    fn g_ascii_strdown(str: *const c_char, len: isize) -> *mut c_char;
    fn g_atomic_int_compare_and_exchange(atomic: *mut c_int, oldval: c_int, newval: c_int)
        -> c_int;
    fn g_atomic_int_set(atomic: *mut c_int, newval: c_int);
    fn g_cancellable_cancel(cancellable: *mut GCancellable);
    fn g_cancellable_new() -> *mut GCancellable;
    fn g_clear_error(error: *mut *mut GError);
    fn g_error_matches(error: *const GError, domain: c_uint, code: c_int) -> c_int;
    fn g_free(mem: *mut c_void);
    fn g_io_error_quark() -> c_uint;
    fn g_idle_add_full(
        priority: c_int,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: *mut c_void,
    ) -> c_uint;
    fn g_log(log_domain: *const c_char, log_level: c_int, format: *const c_char, ...);
    fn g_main_context_find_source_by_id(context: *mut c_void, source_id: c_uint) -> *mut GSource;
    fn g_malloc0(n_bytes: usize) -> *mut c_void;
    fn g_object_add_weak_pointer(object: *mut GObject, weak_pointer_location: *mut *mut c_void);
    fn g_object_get_data(object: *mut GObject, key: *const c_char) -> *mut c_void;
    fn g_object_unref(object: *mut c_void);
    fn g_ptr_array_unref(array: *mut GPtrArray);
    fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *const c_void,
        data: *mut c_void,
        destroy_data: *mut c_void,
        connect_flags: c_int,
    ) -> usize;
    fn g_source_destroy(source: *mut GSource);
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn g_strdup_printf(format: *const c_char, ...) -> *mut c_char;
    fn g_timeout_add(interval: c_uint, function: Option<GSourceFunc>, data: *mut c_void) -> c_uint;
    fn g_timeout_add_seconds(
        interval: c_uint,
        function: Option<GSourceFunc>,
        data: *mut c_void,
    ) -> c_uint;
    fn g_type_check_instance_is_a(instance: *mut GTypeInstance, iface_type: GType) -> c_int;

    fn gdk_button_event_get_button(event: *mut GdkEvent) -> c_uint;
    fn gdk_display_get_default() -> *mut GdkDisplay;
    fn gdk_event_get_device(event: *mut GdkEvent) -> *mut GdkDevice;
    fn gdk_event_get_event_type(event: *mut GdkEvent) -> c_int;
    fn gdk_event_get_modifier_state(event: *mut GdkEvent) -> c_uint;
    fn gdk_event_get_position(event: *mut GdkEvent, x: *mut c_double, y: *mut c_double) -> c_int;
    fn gdk_event_get_time(event: *mut GdkEvent) -> c_uint;
    fn gdk_toplevel_begin_move(
        toplevel: *mut GdkToplevel,
        device: *mut GdkDevice,
        button: c_uint,
        x: c_double,
        y: c_double,
        timestamp: c_uint,
    );
    fn gdk_toplevel_get_type() -> GType;

    fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    fn gtk_button_new() -> *mut GtkWidget;
    fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    fn gtk_css_provider_load_from_string(css_provider: *mut GtkCssProvider, string: *const c_char);
    fn gtk_css_provider_new() -> *mut GtkCssProvider;
    fn gtk_event_controller_legacy_new() -> *mut GtkEventController;
    fn gtk_event_controller_motion_new() -> *mut GtkEventController;
    fn gtk_event_controller_scroll_new(flags: c_int) -> *mut GtkEventController;
    fn gtk_event_controller_set_propagation_phase(
        controller: *mut GtkEventController,
        phase: c_int,
    );
    fn gtk_gesture_click_new() -> *mut GtkGesture;
    fn gtk_gesture_single_set_button(gesture: *mut GtkGestureSingle, button: c_uint);
    fn gtk_gl_area_attach_buffers(area: *mut GtkGLArea);
    fn gtk_gl_area_get_error(area: *mut GtkGLArea) -> *mut GError;
    fn gtk_gl_area_make_current(area: *mut GtkGLArea);
    fn gtk_gl_area_new() -> *mut GtkWidget;
    fn gtk_gl_area_queue_render(area: *mut GtkGLArea);
    fn gtk_gl_area_set_auto_render(area: *mut GtkGLArea, auto_render: c_int);
    fn gtk_grid_attach(
        grid: *mut GtkGrid,
        child: *mut GtkWidget,
        column: c_int,
        row: c_int,
        width: c_int,
        height: c_int,
    );
    fn gtk_grid_layout_child_get_type() -> GType;
    fn gtk_grid_layout_child_set_column(child: *mut GtkGridLayoutChild, column: c_int);
    fn gtk_grid_layout_child_set_column_span(child: *mut GtkGridLayoutChild, span: c_int);
    fn gtk_grid_layout_child_set_row(child: *mut GtkGridLayoutChild, row: c_int);
    fn gtk_grid_layout_child_set_row_span(child: *mut GtkGridLayoutChild, span: c_int);
    fn gtk_grid_new() -> *mut GtkWidget;
    fn gtk_grid_set_column_homogeneous(grid: *mut GtkGrid, homogeneous: c_int);
    fn gtk_grid_set_row_homogeneous(grid: *mut GtkGrid, homogeneous: c_int);
    fn gtk_label_new(str: *const c_char) -> *mut GtkWidget;
    fn gtk_label_set_ellipsize(label: *mut GtkLabel, mode: c_int);
    fn gtk_label_set_text(label: *mut GtkLabel, str: *const c_char);
    fn gtk_label_set_xalign(label: *mut GtkLabel, xalign: f32);
    fn gtk_layout_manager_get_layout_child(
        manager: *mut GtkLayoutManager,
        child: *mut GtkWidget,
    ) -> *mut GtkLayoutChild;
    fn gtk_native_get_surface(self_: *mut GtkNative) -> *mut GdkSurface;
    fn gtk_overlay_add_overlay(overlay: *mut GtkOverlay, widget: *mut GtkWidget);
    fn gtk_overlay_new() -> *mut GtkWidget;
    fn gtk_overlay_set_child(overlay: *mut GtkOverlay, child: *mut GtkWidget);
    fn gtk_popover_popdown(popover: *mut GtkPopover);
    fn gtk_popover_popup(popover: *mut GtkPopover);
    fn gtk_range_set_value(range: *mut GtkRange, value: c_double);
    fn gtk_scale_new_with_range(
        orientation: c_int,
        min: c_double,
        max: c_double,
        step: c_double,
    ) -> *mut GtkWidget;
    fn gtk_scale_set_draw_value(scale: *mut GtkScale, draw_value: c_int);
    fn gtk_style_context_add_provider_for_display(
        display: *mut GdkDisplay,
        provider: *mut GtkStyleProvider,
        priority: c_uint,
    );
    fn gtk_widget_add_controller(widget: *mut GtkWidget, controller: *mut c_void);
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    fn gtk_widget_get_height(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_get_layout_manager(widget: *mut GtkWidget) -> *mut GtkLayoutManager;
    fn gtk_widget_get_native(widget: *mut GtkWidget) -> *mut GtkNative;
    fn gtk_widget_get_realized(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_get_scale_factor(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_get_sensitive(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_get_visible(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_get_width(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_margin_end(widget: *mut GtkWidget, margin: c_int);
    fn gtk_widget_set_sensitive(widget: *mut GtkWidget, sensitive: c_int);
    fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    fn gtk_widget_set_tooltip_text(widget: *mut GtkWidget, text: *const c_char);
    fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_visible(widget: *mut GtkWidget, visible: c_int);
    fn gtk_widget_unparent(widget: *mut GtkWidget);

    fn mpv_error_string(error: c_int) -> *const c_char;
    fn mpv_render_context_create(
        res: *mut *mut MpvRenderContext,
        mpv: *mut MpvHandle,
        params: *mut MpvRenderParam,
    ) -> c_int;
    fn mpv_render_context_free(ctx: *mut MpvRenderContext);
    fn mpv_render_context_render(ctx: *mut MpvRenderContext, params: *mut MpvRenderParam) -> c_int;
    fn mpv_render_context_set_update_callback(
        ctx: *mut MpvRenderContext,
        callback: Option<unsafe extern "C" fn(*mut c_void)>,
        callback_ctx: *mut c_void,
    );
    fn mpv_render_context_update(ctx: *mut MpvRenderContext) -> c_ulonglong;
    fn mpv_wait_event(ctx: *mut MpvHandle, timeout: c_double) -> *mut MpvEvent;

}

unsafe fn is_nonempty(value: *const c_char) -> bool {
    !value.is_null() && *value != 0
}

unsafe fn clear_pointer<T>(slot: *mut *mut T) {
    if !(*slot).is_null() {
        let old = *slot;
        *slot = ptr::null_mut();
        g_free(old as *mut c_void);
    }
}

unsafe fn clear_object<T>(slot: *mut *mut T) {
    if !(*slot).is_null() {
        let old = *slot;
        *slot = ptr::null_mut();
        g_object_unref(old as *mut c_void);
    }
}

unsafe fn add_weak_pointer<T>(object: *mut T, slot: *mut *mut T) {
    g_object_add_weak_pointer(object as *mut GObject, slot as *mut *mut c_void);
}

unsafe fn is_instance<T>(instance: *mut T, type_: GType) -> bool {
    !instance.is_null() && g_type_check_instance_is_a(instance as *mut GTypeInstance, type_) != 0
}

unsafe fn log_warning(format: *const c_char, message: *const c_char) {
    g_log(
        G_LOG_DOMAIN.as_ptr() as *const c_char,
        G_LOG_LEVEL_WARNING,
        format,
        message,
    );
}

fn grid_player_fullscreen_should_restore(
    video_fullscreen_active: c_int,
    app_fullscreen: c_int,
    tile_focused: c_int,
    focused_tile: c_uint,
    tile_index: c_uint,
) -> bool {
    video_fullscreen_active != 0
        || (app_fullscreen != 0 && tile_focused != 0 && focused_tile == tile_index)
}

fn grid_player_fullscreen_should_exit_app(
    app_fullscreen: c_int,
    video_fullscreen_active: c_int,
    restore_app_fullscreen: c_int,
) -> bool {
    app_fullscreen != 0 && (video_fullscreen_active == 0 || restore_app_fullscreen == 0)
}

pub fn grid_player_test_fullscreen_should_restore(
    video_fullscreen_active: c_int,
    app_fullscreen: c_int,
    tile_focused: c_int,
    focused_tile: c_uint,
    tile_index: c_uint,
) -> c_int {
    grid_player_fullscreen_should_restore(
        video_fullscreen_active,
        app_fullscreen,
        tile_focused,
        focused_tile,
        tile_index,
    ) as c_int
}

pub fn grid_player_test_fullscreen_should_exit_app(
    app_fullscreen: c_int,
    video_fullscreen_active: c_int,
    restore_app_fullscreen: c_int,
) -> c_int {
    grid_player_fullscreen_should_exit_app(
        app_fullscreen,
        video_fullscreen_active,
        restore_app_fullscreen,
    ) as c_int
}

unsafe fn tile_mpv(tile: *mut StreamTile) -> *mut MpvHandle {
    player_session_get_mpv((*tile).session)
}

unsafe fn remove_source_if_active(source_id: *mut c_uint) {
    if *source_id == 0 {
        return;
    }

    let source = g_main_context_find_source_by_id(ptr::null_mut(), *source_id);
    if !source.is_null() {
        g_source_destroy(source);
    }
    *source_id = 0;
}

unsafe extern "C" fn get_proc_address(_ctx: *mut c_void, name: *const c_char) -> *mut c_void {
    (epoxy_eglGetProcAddress)(name)
}

unsafe fn configure_gl_area(area: *mut GtkGLArea) {
    gtk_gl_area_set_auto_render(area, FALSE);
}

unsafe extern "C" fn queue_mpv_render(user_data: *mut c_void) -> c_int {
    let tile = user_data as *mut StreamTile;

    g_atomic_int_set(&mut (*tile).render_queued, 0);

    if (*(*tile).app).closing == 0 && !(*tile).gl_area.is_null() {
        gtk_gl_area_queue_render((*tile).gl_area as *mut GtkGLArea);
    }

    G_SOURCE_REMOVE
}

unsafe extern "C" fn warmup_tile_render(user_data: *mut c_void) -> c_int {
    let tile = user_data as *mut StreamTile;

    if (*(*tile).app).closing != 0 || (*tile).gl_area.is_null() || (*tile).render_warmup_frames <= 0
    {
        (*tile).render_warmup_source = 0;
        return G_SOURCE_REMOVE;
    }

    (*tile).render_warmup_frames -= 1;
    gtk_gl_area_queue_render((*tile).gl_area as *mut GtkGLArea);
    G_SOURCE_CONTINUE
}

unsafe fn start_render_warmup(tile: *mut StreamTile) {
    remove_source_if_active(&mut (*tile).render_warmup_source);
    (*tile).render_warmup_frames = 90;
    (*tile).render_warmup_source = g_timeout_add(16, Some(warmup_tile_render), tile as *mut c_void);
}

unsafe extern "C" fn on_mpv_render_update(ctx: *mut c_void) {
    let tile = ctx as *mut StreamTile;

    if g_atomic_int_compare_and_exchange(&mut (*tile).render_queued, 0, 1) != 0 {
        g_idle_add_full(
            MPV_MAINLOOP_PRIORITY,
            Some(queue_mpv_render),
            tile as *mut c_void,
            ptr::null_mut(),
        );
    }
}

unsafe extern "C" fn process_mpv_events(user_data: *mut c_void) -> c_int {
    let tile = user_data as *mut StreamTile;

    g_atomic_int_set(&mut (*tile).event_queued, 0);

    let mpv = tile_mpv(tile);
    if (*(*tile).app).closing != 0 || mpv.is_null() {
        return G_SOURCE_REMOVE;
    }

    loop {
        let event = mpv_wait_event(mpv, 0.0);
        if event.is_null() || (*event).event_id == MPV_EVENT_NONE {
            break;
        }

        match (*event).event_id {
            MPV_EVENT_START_FILE => set_tile_status(tile, cstr!("Loading")),
            MPV_EVENT_FILE_LOADED => set_tile_status(tile, cstr!("Playback running")),
            MPV_EVENT_END_FILE => {
                let end = (*event).data as *mut MpvEventEndFile;
                if !end.is_null() && (*end).reason == MPV_END_FILE_REASON_ERROR {
                    set_tile_status(tile, cstr!("Stream could not be played"));
                } else if end.is_null() || (*end).reason == MPV_END_FILE_REASON_EOF {
                    set_tile_status(tile, cstr!("Stopped"));
                }
            }
            MPV_EVENT_VIDEO_RECONFIG => {}
            MPV_EVENT_LOG_MESSAGE => {
                let log = (*event).data as *mut MpvEventLogMessage;
                if !log.is_null() && !(*log).prefix.is_null() && !(*log).text.is_null() {
                    g_log(
                        G_LOG_DOMAIN.as_ptr() as *const c_char,
                        G_LOG_LEVEL_DEBUG,
                        cstr!("mpv[%s][%u]: %s"),
                        (*log).prefix,
                        (*tile).index,
                        (*log).text,
                    );
                }
            }
            MPV_EVENT_SHUTDOWN => return G_SOURCE_REMOVE,
            _ => {}
        }
    }

    G_SOURCE_REMOVE
}

unsafe extern "C" fn on_mpv_wakeup(ctx: *mut c_void) {
    let tile = ctx as *mut StreamTile;

    if g_atomic_int_compare_and_exchange(&mut (*tile).event_queued, 0, 1) != 0 {
        g_idle_add_full(
            MPV_MAINLOOP_PRIORITY,
            Some(process_mpv_events),
            tile as *mut c_void,
            ptr::null_mut(),
        );
    }
}

unsafe fn dup_twitch_channel_name(value: *const c_char) -> *mut c_char {
    if !is_nonempty(value) {
        return ptr::null_mut();
    }

    let bytes = CStr::from_ptr(value).to_bytes();
    if bytes.is_empty()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        return ptr::null_mut();
    }

    g_ascii_strdown(value, bytes.len() as isize)
}

unsafe fn target_to_label(target: *const c_char, channel: *const c_char) -> *mut c_char {
    if is_nonempty(channel) {
        return g_strdup(channel);
    }

    if is_nonempty(target) {
        g_strdup(target)
    } else {
        ptr::null_mut()
    }
}

unsafe fn set_tile_status(_tile: *mut StreamTile, _message: *const c_char) {}

unsafe fn set_tile_stream_title(
    tile: *mut StreamTile,
    title: *const c_char,
    metadata: *const c_char,
) {
    player_footer_stream_info_set((*tile).stream_info, title, metadata);
}

unsafe fn reset_tile_stream_title(tile: *mut StreamTile) {
    (*tile).title_generation = (*tile).title_generation.wrapping_add(1);
    if !(*tile).title_cancel.is_null() {
        g_cancellable_cancel((*tile).title_cancel);
        clear_object(&mut (*tile).title_cancel);
    }
    (*tile).title_fetch_in_progress = FALSE;
    set_tile_stream_title(tile, cstr!(""), cstr!(""));
}

unsafe fn clear_tile_stream_qualities(tile: *mut StreamTile) {
    player_stream_quality_state_clear(&mut (*tile).stream_quality);
}

unsafe fn reset_tile_quality_selection(tile: *mut StreamTile) {
    player_stream_quality_state_reset_selection(&mut (*tile).stream_quality);
}

unsafe fn tile_settings_popover_is_visible(tile: *mut StreamTile) -> bool {
    !(*tile).stream_settings_popover.is_null()
        && gtk_widget_get_visible((*tile).stream_settings_popover) != 0
}

unsafe fn tile_qualities_cache_is_valid(tile: *mut StreamTile) -> bool {
    player_stream_quality_state_cache_is_valid(
        &mut (*tile).stream_quality,
        STREAM_QUALITY_CACHE_SECONDS,
    ) != 0
}

unsafe fn reload_tile_stream_with_quality(
    tile: *mut StreamTile,
    quality: *const TwitchStreamQuality,
) {
    if quality.is_null() || !is_nonempty((*quality).url) || !is_nonempty((*tile).channel) {
        return;
    }

    player_stream_quality_state_select(&mut (*tile).stream_quality, quality);

    set_tile_status(tile, PLAYER_STARTING_STREAM_STATUS);
    player_session_load_stream(
        (*tile).session,
        (*quality).url,
        (*tile).label,
        (*tile).channel,
    );
    update_tile_empty_state(tile);
    request_tile_title_update(tile, TRUE);
}

unsafe fn reload_tile_stream_auto(tile: *mut StreamTile) {
    player_stream_quality_state_select_auto(&mut (*tile).stream_quality);
    load_tile_stream(tile);
}

unsafe extern "C" fn on_tile_title_fetched(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    let data = user_data as *mut StreamTitleCallbackData;
    let tile = (*data).tile;
    let mut error: *mut GError = ptr::null_mut();
    let stream = twitch_stream_info_fetch_current_stream_finish(result, &mut error);

    if (*data).generation != (*tile).title_generation {
        g_clear_error(&mut error);
        twitch_current_stream_free(stream);
        drop(Box::from_raw(data));
        return;
    }

    (*tile).title_fetch_in_progress = FALSE;
    clear_object(&mut (*tile).title_cancel);

    if (*(*tile).app).closing != 0 || player_session_is_playing((*tile).session) == 0 {
        g_clear_error(&mut error);
        twitch_current_stream_free(stream);
        drop(Box::from_raw(data));
        return;
    }

    if !error.is_null() {
        if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0 {
            g_log(
                G_LOG_DOMAIN.as_ptr() as *const c_char,
                G_LOG_LEVEL_DEBUG,
                cstr!("grid stream title fetch failed: %s"),
                (*error).message,
            );
        }
        g_clear_error(&mut error);
        twitch_current_stream_free(stream);
        drop(Box::from_raw(data));
        return;
    }

    let title = twitch_stream_info_format_current_stream_title(stream);
    let metadata = twitch_stream_info_format_current_stream_metadata(stream);
    set_tile_stream_title(tile, title, metadata);
    g_free(title as *mut c_void);
    g_free(metadata as *mut c_void);
    twitch_current_stream_free(stream);
    drop(Box::from_raw(data));
}

unsafe fn request_tile_title_update(tile: *mut StreamTile, force: c_int) {
    if (*(*tile).app).closing != 0
        || player_session_is_playing((*tile).session) == 0
        || !is_nonempty((*tile).channel)
    {
        return;
    }
    if (*tile).title_fetch_in_progress != 0 && force == 0 {
        return;
    }

    if force != 0 {
        reset_tile_stream_title(tile);
    }

    let data = Box::into_raw(Box::new(StreamTitleCallbackData {
        tile,
        generation: (*tile).title_generation.wrapping_add(1),
    }));
    (*tile).title_generation = (*data).generation;

    (*tile).title_cancel = g_cancellable_new();
    (*tile).title_fetch_in_progress = TRUE;

    twitch_stream_info_fetch_current_stream_async(
        (*tile).channel,
        (*tile).title_cancel,
        Some(on_tile_title_fetched),
        data as *mut c_void,
    );
}

unsafe extern "C" fn refresh_grid_stream_titles(user_data: *mut c_void) -> c_int {
    let state = user_data as *mut GridAppState;

    if (*state).closing != 0 {
        (*state).title_refresh_source = 0;
        return G_SOURCE_REMOVE;
    }

    for i in 0..MAX_TILES {
        request_tile_title_update(&mut (*state).tiles[i], FALSE);
    }

    G_SOURCE_CONTINUE
}

unsafe fn update_tile_channel_label(tile: *mut StreamTile) {
    if (*tile).channel_label.is_null() {
        return;
    }

    let label = if is_nonempty((*tile).label) {
        (*tile).label
    } else {
        PLAYER_EMPTY_STREAM_LABEL as *mut c_char
    };
    gtk_label_set_text((*tile).channel_label as *mut GtkLabel, label);
    gtk_widget_set_tooltip_text(
        (*tile).channel_label,
        if is_nonempty((*tile).label) {
            (*tile).label
        } else {
            ptr::null_mut()
        },
    );
}

unsafe fn sync_tile_from_session(tile: *mut StreamTile) {
    if player_session_is_playing((*tile).session) == 0 {
        return;
    }

    let label = player_session_get_label((*tile).session);
    let channel = player_session_get_channel((*tile).session);

    g_free((*tile).label as *mut c_void);
    g_free((*tile).channel as *mut c_void);
    (*tile).channel = if is_nonempty(channel) {
        g_strdup(channel)
    } else {
        ptr::null_mut()
    };
    (*tile).label = g_strdup(if is_nonempty(label) {
        label
    } else {
        (*tile).channel
    });
}

unsafe fn update_tile_empty_state(tile: *mut StreamTile) {
    let has_stream = is_nonempty((*tile).channel) as c_int;

    if !(*tile).empty_label.is_null() {
        gtk_widget_set_visible((*tile).empty_label, (has_stream == 0) as c_int);
    }
    if !(*tile).close_button.is_null() {
        gtk_widget_set_sensitive((*tile).close_button, TRUE);
    }
    if !(*tile).stream_info_button.is_null() {
        gtk_widget_set_sensitive((*tile).stream_info_button, TRUE);
    }
    if !(*tile).mute_button.is_null() {
        gtk_widget_set_sensitive((*tile).mute_button, TRUE);
        player_volume_update_mute_button((*tile).mute_button, (*tile).session);
    }
    if !(*tile).volume_scale.is_null() {
        gtk_widget_set_sensitive(
            (*tile).volume_scale,
            (has_stream != 0 && player_session_is_ready((*tile).session) != 0) as c_int,
        );
    }
    if !(*tile).channel_refresh_button.is_null() {
        gtk_widget_set_visible((*tile).channel_refresh_button, has_stream);
    }

    update_tile_channel_label(tile);
}

unsafe fn load_tile_stream(tile: *mut StreamTile) {
    if player_session_is_ready((*tile).session) == 0 || !is_nonempty((*tile).channel) {
        return;
    }

    reset_tile_quality_selection(tile);
    let url = g_strdup_printf(cstr!("https://www.twitch.tv/%s"), (*tile).channel);

    set_tile_status(tile, PLAYER_STARTING_STREAM_STATUS);
    player_session_load_stream((*tile).session, url, (*tile).label, (*tile).channel);
    g_free(url as *mut c_void);
    update_tile_empty_state(tile);
    request_tile_title_update(tile, TRUE);
}

unsafe fn clear_tile_render_context(tile: *mut StreamTile) {
    if !(*tile).gl_area.is_null() && gtk_widget_get_realized((*tile).gl_area) != 0 {
        gtk_gl_area_make_current((*tile).gl_area as *mut GtkGLArea);
    }

    if !(*tile).mpv_gl.is_null() {
        mpv_render_context_set_update_callback((*tile).mpv_gl, None, ptr::null_mut());
        mpv_render_context_free((*tile).mpv_gl);
        (*tile).mpv_gl = ptr::null_mut();
    }
    remove_source_if_active(&mut (*tile).render_warmup_source);
    (*tile).last_render_width = 0;
    (*tile).last_render_height = 0;
    (*tile).render_warmup_frames = 0;
}

unsafe fn reset_owned_tile_session(tile: *mut StreamTile) {
    clear_tile_render_context(tile);
    player_session_set_wakeup_callback((*tile).session, None, ptr::null_mut());
    if (*tile).owns_session != 0 {
        player_session_free((*tile).session);
        (*tile).session = player_session_new();
        player_session_set_hwdec_enabled(
            (*tile).session,
            app_settings_get_hwdec_enabled((*(*tile).app).settings),
        );
    } else {
        player_session_stop((*tile).session);
    }
}

unsafe fn stop_tile_stream(tile: *mut StreamTile) {
    reset_owned_tile_session(tile);
    clear_pointer(&mut (*tile).label);
    clear_pointer(&mut (*tile).channel);
    reset_tile_quality_selection(tile);
    reset_tile_stream_title(tile);
    update_tile_empty_state(tile);

    if !(*tile).gl_area.is_null() {
        gtk_gl_area_queue_render((*tile).gl_area as *mut GtkGLArea);
    }
}

unsafe fn ensure_tile_session(tile: *mut StreamTile) -> c_int {
    if (*tile).session.is_null() {
        (*tile).session = player_session_new();
        (*tile).owns_session = TRUE;
    }

    if player_session_is_ready((*tile).session) == 0 {
        update_tile_empty_state(tile);
        return FALSE;
    }

    player_session_set_hwdec_enabled(
        (*tile).session,
        app_settings_get_hwdec_enabled((*(*tile).app).settings),
    );
    player_session_set_wakeup_callback((*tile).session, Some(on_mpv_wakeup), tile as *mut c_void);
    if !(*tile).gl_area.is_null()
        && gtk_widget_get_realized((*tile).gl_area) != 0
        && create_mpv_render_context(tile) == 0
    {
        update_tile_empty_state(tile);
        return FALSE;
    }

    update_tile_empty_state(tile);
    TRUE
}

unsafe fn set_tile_channel(tile: *mut StreamTile, channel: *const AppSettingsChannel) {
    if channel.is_null() || !is_nonempty((*channel).channel) {
        return;
    }

    g_free((*tile).label as *mut c_void);
    g_free((*tile).channel as *mut c_void);
    (*tile).label = g_strdup((*channel).label);
    (*tile).channel = g_strdup((*channel).channel);
    reset_tile_quality_selection(tile);
    reset_tile_stream_title(tile);

    if ensure_tile_session(tile) == 0 {
        return;
    }

    load_tile_stream(tile);
}

unsafe extern "C" fn activate_tile_context_channel(
    channel: *const AppSettingsChannel,
    user_data: *mut c_void,
) {
    let tile = user_data as *mut StreamTile;

    set_tile_channel(tile, channel);
    show_tile_overlay(tile);
}

unsafe extern "C" fn on_volume_changed(range: *mut GtkRange, user_data: *mut c_void) {
    let tile = user_data as *mut StreamTile;

    player_volume_sync_session_from_range((*tile).session, range);
    if player_session_get_muted((*tile).session) != 0 {
        player_volume_set_muted((*tile).session, (*tile).mute_button, FALSE);
    }
}

unsafe extern "C" fn on_tile_close_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let tile = user_data as *mut StreamTile;

    stop_tile_stream(tile);
    show_tile_overlay(tile);
}

unsafe extern "C" fn on_empty_tile_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let tile = user_data as *mut StreamTile;

    channel_switcher_overlay_show_at((*tile).channel_switcher, 0.0, 0.0);
    show_tile_overlay(tile);
}

unsafe extern "C" fn on_mute_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let tile = user_data as *mut StreamTile;

    if !is_nonempty((*tile).channel) {
        return;
    }

    player_volume_toggle_muted((*tile).session, (*tile).mute_button);
    show_tile_overlay(tile);
}

unsafe extern "C" fn on_tile_quality_auto_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let tile = user_data as *mut StreamTile;

    reload_tile_stream_auto(tile);
    if !(*tile).stream_settings_popover.is_null() {
        gtk_popover_popdown((*tile).stream_settings_popover as *mut GtkPopover);
    }
    show_tile_overlay(tile);
}

unsafe extern "C" fn on_tile_quality_button_clicked(
    button: *mut GtkButton,
    user_data: *mut c_void,
) {
    let tile = user_data as *mut StreamTile;
    let quality = g_object_get_data(button as *mut GObject, cstr!("stream-quality"))
        as *const TwitchStreamQuality;

    reload_tile_stream_with_quality(tile, quality);
    if !(*tile).stream_settings_popover.is_null() {
        gtk_popover_popdown((*tile).stream_settings_popover as *mut GtkPopover);
    }
    show_tile_overlay(tile);
}

unsafe extern "C" fn on_tile_stream_info_toggle_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    let tile = user_data as *mut StreamTile;

    player_session_toggle_stream_info((*tile).session);
    if !(*tile).stream_settings_popover.is_null() {
        gtk_popover_popdown((*tile).stream_settings_popover as *mut GtkPopover);
    }
    show_tile_overlay(tile);
}

unsafe extern "C" fn on_tile_stream_qualities_fetched(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    let data = user_data as *mut StreamQualityCallbackData;
    let tile = (*data).tile;
    let mut error: *mut GError = ptr::null_mut();
    let qualities = twitch_stream_info_fetch_stream_qualities_finish(result, &mut error);

    if (*data).generation != (*tile).stream_quality.generation {
        if !qualities.is_null() {
            g_ptr_array_unref(qualities);
        }
        g_clear_error(&mut error);
        drop(Box::from_raw(data));
        return;
    }

    player_stream_quality_state_finish_fetch(&mut (*tile).stream_quality, qualities);

    if !error.is_null() {
        if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0 {
            gtk_label_set_text(
                (*tile).quality_status_label as *mut GtkLabel,
                cstr!("Qualities unavailable"),
            );
            g_log(
                G_LOG_DOMAIN.as_ptr() as *const c_char,
                G_LOG_LEVEL_DEBUG,
                cstr!("grid stream quality fetch failed: %s"),
                (*error).message,
            );
        }
        g_clear_error(&mut error);
        drop(Box::from_raw(data));
        return;
    }

    player_stream_quality_state_mark_fetched(&mut (*tile).stream_quality);
    populate_tile_quality_buttons(tile);
    drop(Box::from_raw(data));
}

unsafe fn request_tile_qualities_update(tile: *mut StreamTile, force: c_int) {
    if (*(*tile).app).closing != 0 || !is_nonempty((*tile).channel) {
        return;
    }
    if (*tile).stream_quality.fetch_in_progress != 0 && force == 0 {
        return;
    }
    if force == 0 && tile_qualities_cache_is_valid(tile) {
        populate_tile_quality_buttons(tile);
        return;
    }

    if force != 0 {
        player_stream_quality_state_cancel_fetch(&mut (*tile).stream_quality);
    }

    gtk_label_set_text(
        (*tile).quality_status_label as *mut GtkLabel,
        cstr!("Loading..."),
    );

    let data = Box::into_raw(Box::new(StreamQualityCallbackData {
        tile,
        generation: player_stream_quality_state_begin_fetch(&mut (*tile).stream_quality),
    }));

    twitch_stream_info_fetch_stream_qualities_async(
        (*tile).channel,
        (*tile).stream_quality.cancel as *mut GCancellable,
        Some(on_tile_stream_qualities_fetched),
        data as *mut c_void,
    );
}

unsafe fn populate_tile_quality_buttons(tile: *mut StreamTile) {
    player_stream_settings_quality_list_populate(
        (*tile).quality_list_box,
        (*tile).quality_status_label,
        (*tile).stream_quality.qualities,
        (*tile).stream_quality.selected_url,
        (*tile).stream_quality.selected_label,
        on_tile_quality_button_clicked as *const c_void,
        tile as *mut c_void,
        on_tile_quality_auto_clicked as *const c_void,
        tile as *mut c_void,
    );
}

unsafe extern "C" fn on_tile_stream_settings_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    let tile = user_data as *mut StreamTile;

    if (*tile).stream_settings_popover.is_null() {
        return;
    }
    if player_session_is_playing((*tile).session) == 0 || !is_nonempty((*tile).channel) {
        show_tile_overlay(tile);
        return;
    }

    request_tile_qualities_update(tile, FALSE);
    gtk_popover_popup((*tile).stream_settings_popover as *mut GtkPopover);
    show_tile_overlay(tile);
}

unsafe extern "C" fn on_channel_refresh_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let tile = user_data as *mut StreamTile;

    if player_session_is_playing((*tile).session) == 0 {
        return;
    }

    player_session_reenable_video((*tile).session);
    start_render_warmup(tile);
    if !(*tile).gl_area.is_null() {
        gtk_gl_area_queue_render((*tile).gl_area as *mut GtkGLArea);
    }
    show_tile_overlay(tile);
}

unsafe extern "C" fn on_channel_button_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let tile = user_data as *mut StreamTile;

    channel_switcher_overlay_show_at((*tile).channel_switcher, 0.0, 0.0);
    show_tile_overlay(tile);
}

unsafe fn is_channel_menu_open(tile: *mut StreamTile) -> bool {
    channel_switcher_overlay_is_visible((*tile).channel_switcher) != 0
}

unsafe extern "C" fn hide_footers(user_data: *mut c_void) -> c_int {
    let state = user_data as *mut GridAppState;

    (*state).footer_hide_source = 0;

    for i in 0..MAX_TILES {
        let tile = &mut (*state).tiles[i] as *mut StreamTile;
        if is_channel_menu_open(tile) || tile_settings_popover_is_visible(tile) {
            schedule_footer_hide(state);
            return G_SOURCE_REMOVE;
        }
    }

    (*state).visible_footer_tile = ptr::null_mut();

    if (*state).closing == 0 {
        if !(*state).top_controls.is_null() {
            gtk_widget_set_visible((*state).top_controls, FALSE);
        }
        for i in 0..MAX_TILES {
            if !(*state).tiles[i].footer.is_null() {
                gtk_widget_set_visible((*state).tiles[i].footer, FALSE);
            }
        }
    }

    G_SOURCE_REMOVE
}

unsafe fn schedule_footer_hide(state: *mut GridAppState) {
    remove_source_if_active(&mut (*state).footer_hide_source);
    (*state).footer_hide_source = g_timeout_add(1800, Some(hide_footers), state as *mut c_void);
}

unsafe fn show_tile_overlay(tile: *mut StreamTile) {
    let state = (*tile).app;

    if (*state).closing != 0 {
        return;
    }

    if !(*state).top_controls.is_null() {
        gtk_widget_set_visible((*state).top_controls, TRUE);
    }
    for i in 0..MAX_TILES {
        if !(*state).tiles[i].footer.is_null() {
            let visible = (&mut (*state).tiles[i] as *mut StreamTile == tile) as c_int;
            gtk_widget_set_visible((*state).tiles[i].footer, visible);
        }
    }
    (*state).visible_footer_tile = tile;

    schedule_footer_hide(state);
}

unsafe fn get_grid_layout_child(
    state: *mut GridAppState,
    child: *mut GtkWidget,
) -> *mut GtkGridLayoutChild {
    let layout = gtk_widget_get_layout_manager((*state).grid);
    let layout_child = gtk_layout_manager_get_layout_child(layout, child);

    if !is_instance(layout_child, gtk_grid_layout_child_get_type()) {
        return ptr::null_mut();
    }

    layout_child as *mut GtkGridLayoutChild
}

unsafe fn set_grid_item_layout(
    state: *mut GridAppState,
    widget: *mut GtkWidget,
    column: c_int,
    row: c_int,
    column_span: c_int,
    row_span: c_int,
) {
    let child = get_grid_layout_child(state, widget);

    if child.is_null() {
        return;
    }

    gtk_grid_layout_child_set_column(child, column);
    gtk_grid_layout_child_set_row(child, row);
    gtk_grid_layout_child_set_column_span(child, column_span);
    gtk_grid_layout_child_set_row_span(child, row_span);
}

unsafe fn restore_grid_layout(state: *mut GridAppState) {
    for i in 0..MAX_TILES {
        if (*state).grid_items[i].is_null() {
            continue;
        }

        set_grid_item_layout(
            state,
            (*state).grid_items[i],
            (i % 2) as c_int,
            (i / 2) as c_int,
            1,
            1,
        );
        gtk_widget_set_visible((*state).grid_items[i], TRUE);
    }

    (*state).tile_focused = FALSE;
}

unsafe fn is_tile_focused(tile: *mut StreamTile) -> bool {
    let state = (*tile).app;

    (*state).tile_focused != 0 && (*state).focused_tile == (*tile).index
}

unsafe fn update_tile_focus_buttons(state: *mut GridAppState) {
    for i in 0..MAX_TILES {
        let tile = &mut (*state).tiles[i] as *mut StreamTile;
        if (*tile).focus_button.is_null() {
            continue;
        }

        let focused = is_tile_focused(tile);
        gtk_button_set_child(
            (*tile).focus_button as *mut GtkButton,
            player_tile_focus_icon_new(if focused {
                PLAYER_TILE_FOCUS_ICON_RESTORE
            } else {
                PLAYER_TILE_FOCUS_ICON_EXPAND
            }),
        );
        gtk_widget_set_tooltip_text(
            (*tile).focus_button,
            if focused {
                cstr!("Restore grid")
            } else {
                cstr!("Focus tile")
            },
        );
    }
}

unsafe fn focus_tile(tile: *mut StreamTile) {
    let state = (*tile).app;

    for i in 0..MAX_TILES {
        if !(*state).grid_items[i].is_null() {
            gtk_widget_set_visible(
                (*state).grid_items[i],
                (i as c_uint == (*tile).index) as c_int,
            );
        }
    }

    set_grid_item_layout(state, (*tile).container, 0, 0, 2, 2);
    (*state).focused_tile = (*tile).index;
    (*state).tile_focused = TRUE;
}

unsafe fn toggle_tile_focus(tile: *mut StreamTile) {
    let state = (*tile).app;

    if (*state).tile_focused != 0 && (*state).focused_tile == (*tile).index {
        restore_grid_layout(state);
    } else {
        focus_tile(tile);
    }

    update_tile_focus_buttons(state);
    show_tile_overlay(tile);
}

unsafe extern "C" fn apply_pending_video_fullscreen_focus(user_data: *mut c_void) -> c_int {
    let state = user_data as *mut GridAppState;

    (*state).video_fullscreen_focus_source = 0;

    if (*state).closing != 0 || (*state).video_fullscreen_pending_tile >= MAX_TILES as c_uint {
        return G_SOURCE_REMOVE;
    }

    let tile =
        &mut (*state).tiles[(*state).video_fullscreen_pending_tile as usize] as *mut StreamTile;
    if !is_tile_focused(tile) {
        focus_tile(tile);
        update_tile_focus_buttons(state);
    }
    show_tile_overlay(tile);

    G_SOURCE_REMOVE
}

unsafe fn schedule_video_fullscreen_focus(tile: *mut StreamTile) {
    let state = (*tile).app;

    remove_source_if_active(&mut (*state).video_fullscreen_focus_source);
    (*state).video_fullscreen_pending_tile = (*tile).index;
    (*state).video_fullscreen_focus_source = g_timeout_add(
        50,
        Some(apply_pending_video_fullscreen_focus),
        state as *mut c_void,
    );
}

unsafe fn restore_video_fullscreen_layout(state: *mut GridAppState, tile: *mut StreamTile) {
    let restore_tile_focused = (*state).video_fullscreen_active != 0
        && (*state).video_fullscreen_restore_tile_focused != 0;
    let restore_focused_tile = (*state).video_fullscreen_restore_focused_tile;

    remove_source_if_active(&mut (*state).video_fullscreen_focus_source);
    (*state).video_fullscreen_active = FALSE;

    if restore_tile_focused
        && restore_focused_tile < MAX_TILES as c_uint
        && !(*state).grid_items[restore_focused_tile as usize].is_null()
    {
        focus_tile(&mut (*state).tiles[restore_focused_tile as usize]);
    } else {
        restore_grid_layout(state);
    }

    update_tile_focus_buttons(state);
    if !tile.is_null() {
        show_tile_overlay(tile);
    }
}

unsafe fn request_tile_fullscreen_toggle(tile: *mut StreamTile) {
    let state = (*tile).app;
    let video_fullscreen_active = (*state).video_fullscreen_active;

    if grid_player_fullscreen_should_restore(
        video_fullscreen_active,
        (*state).fullscreen,
        (*state).tile_focused,
        (*state).focused_tile,
        (*tile).index,
    ) {
        let exit_app_fullscreen = grid_player_fullscreen_should_exit_app(
            (*state).fullscreen,
            video_fullscreen_active,
            (*state).video_fullscreen_restore_app_fullscreen,
        );

        restore_video_fullscreen_layout(state, tile);
        if exit_app_fullscreen {
            if let Some(callback) = (*state).fullscreen_callback {
                callback((*state).fullscreen_user_data);
            }
        }
        return;
    }

    (*state).video_fullscreen_restore_app_fullscreen = (*state).fullscreen;
    (*state).video_fullscreen_restore_tile_focused = (*state).tile_focused;
    (*state).video_fullscreen_restore_focused_tile = (*state).focused_tile;
    (*state).video_fullscreen_active = TRUE;

    if (*state).fullscreen == 0 {
        if let Some(callback) = (*state).fullscreen_callback {
            callback((*state).fullscreen_user_data);
        }
    }

    schedule_video_fullscreen_focus(tile);
}

unsafe extern "C" fn on_tile_focus_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    toggle_tile_focus(user_data as *mut StreamTile);
}

unsafe fn get_toplevel_event_data_from_event(
    window: *mut GtkWidget,
    event: *mut GdkEvent,
    toplevel_out: *mut *mut GdkToplevel,
    device_out: *mut *mut GdkDevice,
    x_out: *mut c_double,
    y_out: *mut c_double,
    timestamp_out: *mut c_uint,
) -> c_int {
    let native = gtk_widget_get_native(window);
    let surface = if native.is_null() {
        ptr::null_mut()
    } else {
        gtk_native_get_surface(native)
    };

    if surface.is_null() || !is_instance(surface, gdk_toplevel_get_type()) || event.is_null() {
        return FALSE;
    }

    *device_out = gdk_event_get_device(event);
    *timestamp_out = gdk_event_get_time(event);

    if (*device_out).is_null() || gdk_event_get_position(event, x_out, y_out) == 0 {
        return FALSE;
    }

    *toplevel_out = surface as *mut GdkToplevel;
    TRUE
}

unsafe fn begin_window_move_from_event(
    state: *mut GridAppState,
    event: *mut GdkEvent,
    button: c_uint,
) {
    let mut toplevel: *mut GdkToplevel = ptr::null_mut();
    let mut device: *mut GdkDevice = ptr::null_mut();
    let mut x = 0.0;
    let mut y = 0.0;
    let mut timestamp = 0;

    if get_toplevel_event_data_from_event(
        (*state).window,
        event,
        &mut toplevel,
        &mut device,
        &mut x,
        &mut y,
        &mut timestamp,
    ) != 0
    {
        gdk_toplevel_begin_move(toplevel, device, button, x, y, timestamp);
    }
}

unsafe extern "C" fn on_tile_motion(
    _controller: *mut GtkEventControllerMotion,
    x: c_double,
    y: c_double,
    user_data: *mut c_void,
) {
    let tile = user_data as *mut StreamTile;
    let state = (*tile).app;

    if player_motion_tracker_ignore_stationary(
        &mut (*state).motion_tracker,
        tile as *mut c_void,
        x,
        y,
    ) != 0
    {
        return;
    }

    show_tile_overlay(tile);
}

unsafe extern "C" fn on_video_pressed(
    _gesture: *mut GtkGestureClick,
    n_press: c_int,
    _x: c_double,
    _y: c_double,
    user_data: *mut c_void,
) {
    if n_press == 2 {
        request_tile_fullscreen_toggle(user_data as *mut StreamTile);
    }
}

unsafe extern "C" fn on_video_legacy_event(
    _controller: *mut GtkEventControllerLegacy,
    event: *mut GdkEvent,
    user_data: *mut c_void,
) -> c_int {
    let tile = user_data as *mut StreamTile;
    let state = (*tile).app;
    let type_ = gdk_event_get_event_type(event);

    if (*state).fullscreen != 0 {
        return GDK_EVENT_PROPAGATE;
    }

    if type_ == GDK_BUTTON_PRESS && gdk_button_event_get_button(event) == GDK_BUTTON_PRIMARY {
        (*state).move_pressed = gdk_event_get_position(
            event,
            &mut (*state).move_press_x,
            &mut (*state).move_press_y,
        );
        return GDK_EVENT_PROPAGATE;
    }

    if type_ == GDK_BUTTON_RELEASE && gdk_button_event_get_button(event) == GDK_BUTTON_PRIMARY {
        (*state).move_pressed = FALSE;
        return GDK_EVENT_PROPAGATE;
    }

    if type_ != GDK_MOTION_NOTIFY || (*state).move_pressed == 0 {
        return GDK_EVENT_PROPAGATE;
    }

    if (gdk_event_get_modifier_state(event) & GDK_BUTTON1_MASK) == 0 {
        (*state).move_pressed = FALSE;
        return GDK_EVENT_PROPAGATE;
    }

    let mut x = 0.0;
    let mut y = 0.0;
    if gdk_event_get_position(event, &mut x, &mut y) == 0 {
        return GDK_EVENT_PROPAGATE;
    }

    if (x - (*state).move_press_x).abs() < 4.0 && (y - (*state).move_press_y).abs() < 4.0 {
        return GDK_EVENT_PROPAGATE;
    }

    (*state).move_pressed = FALSE;
    begin_window_move_from_event(state, event, GDK_BUTTON_PRIMARY);
    GDK_EVENT_STOP
}

unsafe extern "C" fn on_tile_scroll(
    _controller: *mut GtkEventControllerScroll,
    dx: c_double,
    dy: c_double,
    user_data: *mut c_void,
) -> c_int {
    let tile = user_data as *mut StreamTile;
    if channel_switcher_overlay_is_visible((*tile).channel_switcher) != 0 {
        return GDK_EVENT_PROPAGATE;
    }

    if (*tile).volume_scale.is_null()
        || gtk_widget_get_sensitive((*tile).volume_scale) == 0
        || player_volume_apply_scroll((*tile).volume_scale, dx, dy) == 0
    {
        return GDK_EVENT_PROPAGATE;
    }

    show_tile_overlay(tile);
    GDK_EVENT_STOP
}

unsafe extern "C" fn on_context_pressed(
    _gesture: *mut GtkGestureClick,
    n_press: c_int,
    x: c_double,
    y: c_double,
    user_data: *mut c_void,
) {
    if n_press != 1 {
        return;
    }

    let tile = user_data as *mut StreamTile;
    channel_switcher_overlay_show_at((*tile).channel_switcher, x, y);
    show_tile_overlay(tile);
}

unsafe extern "C" fn on_gl_render(
    area: *mut GtkGLArea,
    _context: *mut GdkGLContext,
    user_data: *mut c_void,
) -> c_int {
    let tile = user_data as *mut StreamTile;

    if (*tile).mpv_gl.is_null() {
        gtk_gl_area_attach_buffers(area);
        (epoxy_glClearColor)(0.02, 0.02, 0.02, 1.0);
        (epoxy_glClear)(GL_COLOR_BUFFER_BIT);
        return TRUE;
    }

    let scale = gtk_widget_get_scale_factor(area as *mut GtkWidget);
    let width = gtk_widget_get_width(area as *mut GtkWidget) * scale;
    let height = gtk_widget_get_height(area as *mut GtkWidget) * scale;

    if width <= 0 || height <= 0 {
        return TRUE;
    }

    let update_flags = mpv_render_context_update((*tile).mpv_gl) as u64;
    let size_changed = width != (*tile).last_render_width || height != (*tile).last_render_height;
    let warming_up = (*tile).render_warmup_frames > 0;

    if (update_flags & MPV_RENDER_UPDATE_FRAME) == 0 && !size_changed && !warming_up {
        return TRUE;
    }

    gtk_gl_area_attach_buffers(area);

    let mut current_fbo: c_int = 0;
    (epoxy_glGetIntegerv)(GL_FRAMEBUFFER_BINDING, &mut current_fbo);

    let mut fbo = MpvOpenGLFbo {
        fbo: current_fbo,
        w: width,
        h: height,
        internal_format: 0,
    };
    let mut flip_y: c_int = 1;
    let mut params = [
        MpvRenderParam {
            type_: MPV_RENDER_PARAM_OPENGL_FBO,
            data: &mut fbo as *mut MpvOpenGLFbo as *mut c_void,
        },
        MpvRenderParam {
            type_: MPV_RENDER_PARAM_FLIP_Y,
            data: &mut flip_y as *mut c_int as *mut c_void,
        },
        MpvRenderParam {
            type_: MPV_RENDER_PARAM_INVALID,
            data: ptr::null_mut(),
        },
    ];

    let status = mpv_render_context_render((*tile).mpv_gl, params.as_mut_ptr());
    if status < 0 {
        log_warning(cstr!("mpv render: %s"), mpv_error_string(status));
    } else {
        (*tile).last_render_width = width;
        (*tile).last_render_height = height;
    }

    TRUE
}

unsafe fn create_mpv_render_context(tile: *mut StreamTile) -> c_int {
    let mpv = tile_mpv(tile);
    if mpv.is_null() || (*tile).gl_area.is_null() {
        return FALSE;
    }

    gtk_gl_area_make_current((*tile).gl_area as *mut GtkGLArea);

    let gl_error = gtk_gl_area_get_error((*tile).gl_area as *mut GtkGLArea);
    if !gl_error.is_null() {
        log_warning(cstr!("GTK GL area error: %s"), (*gl_error).message);
        return FALSE;
    }

    if !(*tile).mpv_gl.is_null() {
        mpv_render_context_set_update_callback((*tile).mpv_gl, None, ptr::null_mut());
        mpv_render_context_free((*tile).mpv_gl);
        (*tile).mpv_gl = ptr::null_mut();
    }

    let mut gl_init_params = MpvOpenGLInitParams {
        get_proc_address: Some(get_proc_address),
        get_proc_address_ctx: ptr::null_mut(),
    };
    let mut params = [
        MpvRenderParam {
            type_: MPV_RENDER_PARAM_API_TYPE,
            data: cstr!("opengl") as *mut c_void,
        },
        MpvRenderParam {
            type_: MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
            data: &mut gl_init_params as *mut MpvOpenGLInitParams as *mut c_void,
        },
        MpvRenderParam {
            type_: MPV_RENDER_PARAM_INVALID,
            data: ptr::null_mut(),
        },
    ];

    let status = mpv_render_context_create(&mut (*tile).mpv_gl, mpv, params.as_mut_ptr());
    if status < 0 {
        log_warning(cstr!("mpv render context: %s"), mpv_error_string(status));
        return FALSE;
    }

    mpv_render_context_set_update_callback(
        (*tile).mpv_gl,
        Some(on_mpv_render_update),
        tile as *mut c_void,
    );
    player_session_reenable_video((*tile).session);
    start_render_warmup(tile);
    gtk_gl_area_queue_render((*tile).gl_area as *mut GtkGLArea);
    TRUE
}

unsafe extern "C" fn on_gl_realize(_area: *mut GtkGLArea, user_data: *mut c_void) {
    let tile = user_data as *mut StreamTile;

    if !tile_mpv(tile).is_null() && create_mpv_render_context(tile) == 0 {
        set_tile_status(tile, cstr!("Render error"));
    }
}

unsafe extern "C" fn on_gl_unrealize(area: *mut GtkGLArea, user_data: *mut c_void) {
    let tile = user_data as *mut StreamTile;

    gtk_gl_area_make_current(area);
    clear_tile_render_context(tile);
}

unsafe fn create_tile_footer(tile: *mut StreamTile) -> *mut GtkWidget {
    let box_ = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 4);
    gtk_widget_add_css_class(box_, cstr!("player-footer"));
    gtk_widget_add_css_class(box_, cstr!("tile-footer"));

    let channel_selector = gtk_overlay_new();
    gtk_widget_add_css_class(channel_selector, cstr!("channel-selector"));
    gtk_widget_set_size_request(channel_selector, GRID_CHANNEL_DROPDOWN_WIDTH, -1);
    gtk_widget_set_hexpand(channel_selector, FALSE);

    (*tile).channel_combo = gtk_button_new();
    gtk_widget_add_css_class((*tile).channel_combo, cstr!("channel-dropdown"));
    (*tile).channel_label = gtk_label_new(cstr!(""));
    gtk_widget_add_css_class((*tile).channel_label, cstr!("channel-button-label"));
    gtk_widget_set_halign((*tile).channel_label, GTK_ALIGN_START);
    gtk_widget_set_margin_end((*tile).channel_label, 20);
    gtk_label_set_xalign((*tile).channel_label as *mut GtkLabel, 0.0);
    gtk_label_set_ellipsize((*tile).channel_label as *mut GtkLabel, PANGO_ELLIPSIZE_END);
    gtk_button_set_child(
        (*tile).channel_combo as *mut GtkButton,
        (*tile).channel_label,
    );
    gtk_widget_set_halign((*tile).channel_combo, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand((*tile).channel_combo, TRUE);
    g_signal_connect_data(
        (*tile).channel_combo as *mut c_void,
        cstr!("clicked"),
        on_channel_button_clicked as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );

    gtk_overlay_set_child(channel_selector as *mut GtkOverlay, (*tile).channel_combo);

    (*tile).channel_refresh_button =
        player_overlay_button_new(player_refresh_icon_new(), cstr!("Refresh video"));
    gtk_widget_add_css_class(
        (*tile).channel_refresh_button,
        cstr!("channel-refresh-button"),
    );
    gtk_widget_add_css_class(
        (*tile).channel_refresh_button,
        cstr!("player-refresh-button"),
    );
    gtk_widget_set_halign((*tile).channel_refresh_button, GTK_ALIGN_END);
    gtk_widget_set_valign((*tile).channel_refresh_button, GTK_ALIGN_CENTER);
    gtk_widget_set_margin_end((*tile).channel_refresh_button, 3);
    gtk_overlay_add_overlay(
        channel_selector as *mut GtkOverlay,
        (*tile).channel_refresh_button,
    );
    g_signal_connect_data(
        (*tile).channel_refresh_button as *mut c_void,
        cstr!("clicked"),
        on_channel_refresh_clicked as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );

    (*tile).close_button = player_overlay_button_new(player_trash_icon_new(), cstr!("Clear slot"));
    gtk_widget_add_css_class((*tile).close_button, cstr!("tile-close-button"));
    g_signal_connect_data(
        (*tile).close_button as *mut c_void,
        cstr!("clicked"),
        on_tile_close_clicked as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );

    (*tile).stream_info = player_footer_stream_info_new();

    (*tile).volume_scale = gtk_scale_new_with_range(
        GTK_ORIENTATION_HORIZONTAL,
        PLAYER_VOLUME_MIN,
        PLAYER_VOLUME_MAX,
        1.0,
    );
    gtk_widget_add_css_class((*tile).volume_scale, cstr!("volume-scale"));
    gtk_range_set_value(
        (*tile).volume_scale as *mut GtkRange,
        player_session_get_volume((*tile).session),
    );
    gtk_scale_set_draw_value((*tile).volume_scale as *mut GtkScale, FALSE);
    gtk_widget_set_size_request((*tile).volume_scale, GRID_VOLUME_SCALE_WIDTH, -1);
    g_signal_connect_data(
        (*tile).volume_scale as *mut c_void,
        cstr!("value-changed"),
        on_volume_changed as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );

    (*tile).mute_button = player_volume_mute_button_new((*tile).session);
    g_signal_connect_data(
        (*tile).mute_button as *mut c_void,
        cstr!("clicked"),
        on_mute_clicked as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );

    (*tile).focus_button = player_overlay_button_new(
        player_tile_focus_icon_new(PLAYER_TILE_FOCUS_ICON_EXPAND),
        cstr!("Focus tile"),
    );
    g_signal_connect_data(
        (*tile).focus_button as *mut c_void,
        cstr!("clicked"),
        on_tile_focus_clicked as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );

    (*tile).stream_info_button =
        player_overlay_button_new(player_stream_settings_icon_new(), cstr!("Stream settings"));
    gtk_widget_add_css_class((*tile).stream_info_button, cstr!("stream-settings-button"));
    g_signal_connect_data(
        (*tile).stream_info_button as *mut c_void,
        cstr!("clicked"),
        on_tile_stream_settings_clicked as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );

    let mut info_button: *mut GtkWidget = ptr::null_mut();
    (*tile).stream_settings_popover = player_stream_settings_popover_new(
        (*tile).stream_info_button,
        &mut (*tile).quality_list_box,
        &mut (*tile).quality_status_label,
        &mut info_button,
    );
    g_signal_connect_data(
        info_button as *mut c_void,
        cstr!("clicked"),
        on_tile_stream_info_toggle_clicked as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );

    gtk_box_append(box_ as *mut GtkBox, channel_selector);
    gtk_box_append(box_ as *mut GtkBox, (*tile).close_button);
    gtk_box_append(
        box_ as *mut GtkBox,
        player_footer_stream_info_get_widget((*tile).stream_info),
    );
    gtk_box_append(box_ as *mut GtkBox, (*tile).mute_button);
    gtk_box_append(box_ as *mut GtkBox, (*tile).volume_scale);
    gtk_box_append(box_ as *mut GtkBox, (*tile).focus_button);
    gtk_box_append(box_ as *mut GtkBox, (*tile).stream_info_button);
    update_tile_empty_state(tile);

    box_
}

unsafe fn create_stream_tile(
    state: *mut GridAppState,
    index: c_uint,
    target: *const c_char,
) -> *mut GtkWidget {
    let tile = &mut (*state).tiles[index as usize] as *mut StreamTile;
    (*tile).app = state;
    (*tile).index = index;
    (*tile).channel = dup_twitch_channel_name(target);
    (*tile).label = target_to_label(target, (*tile).channel);
    if index == 0 && !(*state).primary_session.is_null() {
        (*tile).session = (*state).primary_session;
    } else if is_nonempty((*tile).channel) {
        (*tile).session = player_session_new();
        player_session_set_hwdec_enabled(
            (*tile).session,
            app_settings_get_hwdec_enabled((*state).settings),
        );
    }
    (*tile).owns_session =
        (!(*tile).session.is_null() && (*tile).session != (*state).primary_session) as c_int;
    sync_tile_from_session(tile);

    (*tile).container = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    add_weak_pointer((*tile).container, &mut (*tile).container);
    gtk_widget_add_css_class((*tile).container, cstr!("tile-container"));
    if index % 2 == 0 {
        gtk_widget_add_css_class((*tile).container, cstr!("tile-left"));
    }
    if index / 2 == 0 {
        gtk_widget_add_css_class((*tile).container, cstr!("tile-top"));
    }
    gtk_widget_set_hexpand((*tile).container, TRUE);
    gtk_widget_set_vexpand((*tile).container, TRUE);

    (*tile).overlay = gtk_overlay_new();
    add_weak_pointer((*tile).overlay, &mut (*tile).overlay);
    gtk_widget_set_hexpand((*tile).overlay, TRUE);
    gtk_widget_set_vexpand((*tile).overlay, TRUE);
    gtk_box_append((*tile).container as *mut GtkBox, (*tile).overlay);

    (*tile).gl_area = gtk_gl_area_new();
    add_weak_pointer((*tile).gl_area, &mut (*tile).gl_area);
    configure_gl_area((*tile).gl_area as *mut GtkGLArea);
    gtk_widget_set_hexpand((*tile).gl_area, TRUE);
    gtk_widget_set_vexpand((*tile).gl_area, TRUE);
    gtk_overlay_set_child((*tile).overlay as *mut GtkOverlay, (*tile).gl_area);

    (*tile).empty_label = gtk_button_new();
    let empty_icon_frame = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class(empty_icon_frame, cstr!("empty-stream-button-visible"));
    gtk_widget_set_halign(empty_icon_frame, GTK_ALIGN_CENTER);
    gtk_widget_set_valign(empty_icon_frame, GTK_ALIGN_CENTER);
    gtk_box_append(empty_icon_frame as *mut GtkBox, player_plus_icon_new());
    gtk_button_set_child((*tile).empty_label as *mut GtkButton, empty_icon_frame);
    gtk_widget_add_css_class((*tile).empty_label, cstr!("empty-stream-button"));
    gtk_widget_set_tooltip_text((*tile).empty_label, cstr!("Select channel"));
    gtk_widget_set_halign((*tile).empty_label, GTK_ALIGN_CENTER);
    gtk_widget_set_valign((*tile).empty_label, GTK_ALIGN_CENTER);
    g_signal_connect_data(
        (*tile).empty_label as *mut c_void,
        cstr!("clicked"),
        on_empty_tile_clicked as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_overlay_add_overlay((*tile).overlay as *mut GtkOverlay, (*tile).empty_label);

    (*tile).footer = create_tile_footer(tile);
    gtk_widget_set_halign((*tile).footer, GTK_ALIGN_FILL);
    gtk_widget_set_valign((*tile).footer, GTK_ALIGN_END);
    gtk_widget_set_visible((*tile).footer, FALSE);
    gtk_overlay_add_overlay((*tile).overlay as *mut GtkOverlay, (*tile).footer);
    (*tile).channel_switcher = channel_switcher_overlay_new(
        (*state).root_overlay as *mut GtkOverlay,
        (*state).settings,
        Some(activate_tile_context_channel),
        tile as *mut c_void,
        (*state).settings_callback,
        (*state).settings_user_data,
    );

    let video_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(video_click as *mut GtkGestureSingle, GDK_BUTTON_PRIMARY);
    g_signal_connect_data(
        video_click as *mut c_void,
        cstr!("pressed"),
        on_video_pressed as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*tile).gl_area, video_click as *mut c_void);

    let context_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(context_click as *mut GtkGestureSingle, GDK_BUTTON_SECONDARY);
    g_signal_connect_data(
        context_click as *mut c_void,
        cstr!("pressed"),
        on_context_pressed as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*tile).overlay, context_click as *mut c_void);

    let video_legacy = gtk_event_controller_legacy_new();
    g_signal_connect_data(
        video_legacy as *mut c_void,
        cstr!("event"),
        on_video_legacy_event as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*tile).gl_area, video_legacy as *mut c_void);

    let video_motion = gtk_event_controller_motion_new();
    gtk_event_controller_set_propagation_phase(video_motion, GTK_PHASE_CAPTURE);
    g_signal_connect_data(
        video_motion as *mut c_void,
        cstr!("motion"),
        on_tile_motion as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*tile).overlay, video_motion as *mut c_void);

    let tile_scroll = gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_VERTICAL);
    gtk_event_controller_set_propagation_phase(tile_scroll, GTK_PHASE_CAPTURE);
    g_signal_connect_data(
        tile_scroll as *mut c_void,
        cstr!("scroll"),
        on_tile_scroll as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*tile).overlay, tile_scroll as *mut c_void);

    g_signal_connect_data(
        (*tile).gl_area as *mut c_void,
        cstr!("realize"),
        on_gl_realize as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        (*tile).gl_area as *mut c_void,
        cstr!("unrealize"),
        on_gl_unrealize as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        (*tile).gl_area as *mut c_void,
        cstr!("render"),
        on_gl_render as *const c_void,
        tile as *mut c_void,
        ptr::null_mut(),
        0,
    );

    update_tile_empty_state(tile);

    (*tile).container
}

unsafe fn install_css() {
    player_style_install_overlay_css();

    let provider = gtk_css_provider_new();
    let mut css = Vec::with_capacity(GRID_CSS.len() + 1);
    css.extend_from_slice(GRID_CSS.as_bytes());
    css.push(0);
    gtk_css_provider_load_from_string(provider, css.as_ptr() as *const c_char);
    gtk_style_context_add_provider_for_display(
        gdk_display_get_default(),
        provider as *mut GtkStyleProvider,
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    g_object_unref(provider as *mut c_void);
    player_style_install_footer_css();
}

unsafe fn get_target_count(state: *mut GridAppState) -> c_uint {
    (*state).target_count.min(MAX_TILES as c_uint)
}

unsafe fn get_target_at(state: *mut GridAppState, index: c_uint) -> *const c_char {
    if index < (*state).target_count {
        (*state).targets[index as usize]
    } else {
        ptr::null()
    }
}

pub unsafe fn grid_player_free(player: *mut GridAppState) {
    let state = player;

    if state.is_null() {
        return;
    }

    (*state).closing = TRUE;

    remove_source_if_active(&mut (*state).footer_hide_source);
    remove_source_if_active(&mut (*state).title_refresh_source);
    remove_source_if_active(&mut (*state).video_fullscreen_focus_source);

    for i in 0..MAX_TILES {
        let tile = &mut (*state).tiles[i] as *mut StreamTile;

        clear_tile_render_context(tile);
        reset_tile_stream_title(tile);
        clear_tile_stream_qualities(tile);
        player_session_set_wakeup_callback((*tile).session, None, ptr::null_mut());
        if (*tile).owns_session != 0 {
            player_session_free((*tile).session);
        }
        (*tile).session = ptr::null_mut();

        clear_pointer(&mut (*tile).label);
        clear_pointer(&mut (*tile).channel);
        (*tile).container = ptr::null_mut();
        (*tile).overlay = ptr::null_mut();
        (*tile).gl_area = ptr::null_mut();
        (*tile).footer = ptr::null_mut();
        (*tile).channel_combo = ptr::null_mut();
        (*tile).channel_label = ptr::null_mut();
        (*tile).channel_refresh_button = ptr::null_mut();
        if !(*tile).stream_info.is_null() {
            player_footer_stream_info_free((*tile).stream_info);
            (*tile).stream_info = ptr::null_mut();
        }
        (*tile).close_button = ptr::null_mut();
        (*tile).empty_label = ptr::null_mut();
        (*tile).stream_info_button = ptr::null_mut();
        (*tile).mute_button = ptr::null_mut();
        (*tile).volume_scale = ptr::null_mut();
        if !(*tile).stream_settings_popover.is_null() {
            gtk_widget_unparent((*tile).stream_settings_popover);
        }
        (*tile).stream_settings_popover = ptr::null_mut();
        (*tile).quality_list_box = ptr::null_mut();
        (*tile).quality_status_label = ptr::null_mut();
        channel_switcher_overlay_free((*tile).channel_switcher);
        (*tile).channel_switcher = ptr::null_mut();
    }

    for i in 0..MAX_TILES {
        clear_pointer(&mut (*state).targets[i]);
    }

    (*state).root_overlay = ptr::null_mut();
    (*state).grid = ptr::null_mut();
    (*state).primary_session = ptr::null_mut();
    (*state).settings = ptr::null_mut();
    /* mpv may already have queued idle callbacks that still carry tile pointers. */
}

pub unsafe fn grid_player_new<W>(
    window: *mut W,
    settings: *mut AppSettings,
    primary_session: *mut PlayerSession,
    targets: *const *const c_char,
    target_count: c_uint,
    fullscreen_callback: GridPlayerFullscreenCallback,
    fullscreen_user_data: *mut c_void,
    settings_callback: GridPlayerSettingsCallback,
    settings_user_data: *mut c_void,
) -> *mut GridAppState {
    let window = window as *mut GtkWindow;
    install_css();

    let state = g_malloc0(mem::size_of::<GridAppState>()) as *mut GridAppState;
    (*state).window = window as *mut GtkWidget;
    (*state).primary_session = primary_session;
    (*state).target_count = if !targets.is_null() {
        target_count.min(MAX_TILES as c_uint)
    } else {
        0
    };
    for i in 0..(*state).target_count {
        (*state).targets[i as usize] = g_strdup(*targets.add(i as usize));
    }
    (*state).settings = settings;
    (*state).fullscreen_callback = fullscreen_callback;
    (*state).fullscreen_user_data = fullscreen_user_data;
    (*state).settings_callback = settings_callback;
    (*state).settings_user_data = settings_user_data;

    (*state).root_overlay = gtk_overlay_new();
    add_weak_pointer((*state).root_overlay, &mut (*state).root_overlay);
    gtk_widget_add_css_class((*state).root_overlay, cstr!("grid-root"));
    gtk_widget_set_hexpand((*state).root_overlay, TRUE);
    gtk_widget_set_vexpand((*state).root_overlay, TRUE);

    (*state).grid = gtk_grid_new();
    add_weak_pointer((*state).grid, &mut (*state).grid);
    gtk_widget_add_css_class((*state).grid, cstr!("stream-grid"));
    gtk_widget_set_hexpand((*state).grid, TRUE);
    gtk_widget_set_vexpand((*state).grid, TRUE);
    gtk_grid_set_row_homogeneous((*state).grid as *mut GtkGrid, TRUE);
    gtk_grid_set_column_homogeneous((*state).grid as *mut GtkGrid, TRUE);
    gtk_overlay_set_child((*state).root_overlay as *mut GtkOverlay, (*state).grid);

    let initial_target_count = get_target_count(state);
    for i in 0..MAX_TILES as c_uint {
        let tile_widget = create_stream_tile(
            state,
            i,
            if i < initial_target_count {
                get_target_at(state, i)
            } else {
                ptr::null()
            },
        );

        gtk_grid_attach(
            (*state).grid as *mut GtkGrid,
            tile_widget,
            (i % 2) as c_int,
            (i / 2) as c_int,
            1,
            1,
        );
        (*state).grid_items[i as usize] = tile_widget;
    }

    schedule_footer_hide(state);
    (*state).title_refresh_source = g_timeout_add_seconds(
        STREAM_TITLE_REFRESH_SECONDS,
        Some(refresh_grid_stream_titles),
        state as *mut c_void,
    );

    state
}

pub unsafe fn grid_player_get_widget<W>(player: *mut GridAppState) -> *mut W {
    if !player.is_null() {
        (*player).root_overlay as *mut W
    } else {
        ptr::null_mut()
    }
}

pub unsafe fn grid_player_dup_first_target(player: *mut GridAppState) -> *mut c_char {
    if player.is_null() {
        return ptr::null_mut();
    }

    for i in 0..MAX_TILES {
        let channel = (*player).tiles[i].channel;
        if is_nonempty(channel) {
            return g_strdup(channel);
        }
    }

    ptr::null_mut()
}

pub unsafe fn grid_player_take_first_session(player: *mut GridAppState) -> *mut PlayerSession {
    if player.is_null() {
        return ptr::null_mut();
    }

    for i in 0..MAX_TILES {
        let tile = &mut (*player).tiles[i] as *mut StreamTile;
        if player_session_is_playing((*tile).session) == 0 {
            continue;
        }

        let session = (*tile).session;
        clear_tile_render_context(tile);
        player_session_set_wakeup_callback(session, None, ptr::null_mut());
        (*tile).session = ptr::null_mut();
        (*tile).owns_session = FALSE;
        return session;
    }

    ptr::null_mut()
}

pub unsafe fn grid_player_start(player: *mut GridAppState) {
    if player.is_null() || (*player).started != 0 {
        return;
    }

    (*player).started = TRUE;
    for i in 0..MAX_TILES {
        let tile = &mut (*player).tiles[i] as *mut StreamTile;
        if !is_nonempty((*tile).channel) && player_session_is_playing((*tile).session) == 0 {
            continue;
        }

        if ensure_tile_session(tile) != 0 {
            if player_session_is_playing((*tile).session) != 0 {
                sync_tile_from_session(tile);
                update_tile_empty_state(tile);
                set_tile_status(tile, cstr!("Playback running"));
                request_tile_title_update(tile, TRUE);
                continue;
            }
            load_tile_stream(tile);
        }
    }
}

pub unsafe fn grid_player_set_fullscreen(player: *mut GridAppState, fullscreen: c_int) {
    if !player.is_null() {
        (*player).fullscreen = fullscreen;
        if fullscreen == 0 {
            if (*player).video_fullscreen_active != 0 {
                restore_video_fullscreen_layout(player, ptr::null_mut());
            } else {
                remove_source_if_active(&mut (*player).video_fullscreen_focus_source);
            }
        }
    }
}

pub unsafe fn grid_player_set_settings(player: *mut GridAppState, settings: *mut AppSettings) {
    if player.is_null() {
        return;
    }

    (*player).settings = settings;
    for i in 0..MAX_TILES {
        player_session_set_hwdec_enabled(
            (*player).tiles[i].session,
            app_settings_get_hwdec_enabled(settings),
        );
        channel_switcher_overlay_set_settings((*player).tiles[i].channel_switcher, settings);
    }
}
