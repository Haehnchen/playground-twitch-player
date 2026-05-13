#![allow(clashing_extern_declarations)]

use std::ffi::{c_char, c_double, c_int, c_uint, c_ulonglong, c_void, CStr};
use std::mem;
use std::ptr;

use crate::channel_switcher_overlay::{
    channel_switcher_overlay_free, channel_switcher_overlay_is_visible,
    channel_switcher_overlay_new, channel_switcher_overlay_set_settings,
    channel_switcher_overlay_show_at, ChannelSwitcherOverlay,
};
use crate::chat_panel::{
    chat_panel_free, chat_panel_get_widget, chat_panel_new, chat_panel_start, ChatPanel,
};
use crate::player_footer::{
    player_footer_stream_info_free, player_footer_stream_info_get_widget,
    player_footer_stream_info_new, player_footer_stream_info_set, PlayerFooterStreamInfo,
};
use crate::player_icons::{
    player_chat_icon_new, player_plus_icon_new, player_refresh_icon_new,
    player_stream_settings_icon_new,
};
use crate::player_motion::{player_motion_tracker_ignore_stationary, PlayerMotionTracker};
use crate::player_overlay_controls::player_overlay_button_new;
use crate::player_session::{
    player_session_drop_buffers, player_session_dup_url, player_session_get_channel,
    player_session_get_mpv, player_session_get_muted, player_session_get_url,
    player_session_get_volume, player_session_is_playing, player_session_is_ready,
    player_session_load_stream, player_session_reenable_video, player_session_set_hwdec_enabled,
    player_session_set_wakeup_callback, player_session_toggle_stream_info, MpvHandle,
    PlayerSession,
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
use crate::player_volume::{
    player_volume_apply_scroll, player_volume_mute_button_new, player_volume_set_muted,
    player_volume_sync_session_from_range, player_volume_toggle_muted,
};
use crate::settings::{
    app_settings_get_channel, app_settings_get_channel_count, app_settings_get_hwdec_enabled,
    AppSettings, AppSettingsChannel,
};
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

const STREAM_TITLE_REFRESH_SECONDS: c_uint = 3 * 60;
const STREAM_QUALITY_CACHE_SECONDS: c_uint = 2 * 60;
const STREAM_DROPDOWN_WIDTH: c_int = 140;
const DEFAULT_CHAT_WIDTH: c_int = 280;
const MIN_CHAT_WIDTH: c_int = 180;
const MIN_VIDEO_WIDTH: c_int = 320;
const MPV_MAINLOOP_PRIORITY: c_int = -100;

const FALSE: c_int = 0;
const TRUE: c_int = 1;
const G_SOURCE_REMOVE: c_int = 0;
const G_SOURCE_CONTINUE: c_int = 1;
const G_IO_ERROR_CANCELLED: c_int = 19;
const G_LOG_LEVEL_DEBUG: c_int = 1 << 7;
const G_LOG_LEVEL_WARNING: c_int = 1 << 4;
const G_LOG_DOMAIN: &[u8] = b"twitch-player-single\0";

const GTK_ALIGN_FILL: c_int = 0;
const GTK_ALIGN_START: c_int = 1;
const GTK_ALIGN_END: c_int = 2;
const GTK_ALIGN_CENTER: c_int = 3;
const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
const GTK_ORIENTATION_VERTICAL: c_int = 1;
const GTK_PHASE_CAPTURE: c_int = 1;
const GTK_EVENT_CONTROLLER_SCROLL_VERTICAL: c_int = 1;

const GDK_BUTTON_PRIMARY: c_uint = 1;
const GDK_BUTTON_SECONDARY: c_uint = 3;
const GDK_EVENT_PROPAGATE: c_int = FALSE;
const GDK_EVENT_STOP: c_int = TRUE;
const GDK_MOTION_NOTIFY: c_int = 1;
const GDK_BUTTON_PRESS: c_int = 2;
const GDK_BUTTON_RELEASE: c_int = 3;
const GDK_BUTTON1_MASK: c_uint = 1 << 8;
const GDK_CONTROL_MASK: c_uint = 1 << 2;
const GDK_KEY_M_UPPER: c_uint = 0x04d;
const GDK_KEY_M_LOWER: c_uint = 0x06d;

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

const PLAYER_CHAT_ICON_OPEN: c_int = 0;
const PLAYER_CHAT_ICON_CLOSE: c_int = 1;
const PLAYER_EMPTY_STREAM_LABEL: *const c_char = cstr!("Select");
const PLAYER_STARTING_STREAM_STATUS: *const c_char = cstr!("Starting stream");
const PLAYER_VOLUME_MIN: c_double = 0.0;
const PLAYER_VOLUME_MAX: c_double = 130.0;

#[repr(C)]
struct GdkDevice {
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
struct GtkLabel {
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
struct GtkPaned {
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

struct StreamEntry {
    label: *mut c_char,
    channel: *mut c_char,
    url: *mut c_char,
}

pub struct SinglePlayer {
    startup_target: *const c_char,
    window: *mut GtkWidget,
    video_overlay: *mut GtkWidget,
    gl_area: *mut GtkWidget,
    main_area: *mut GtkWidget,
    chat_toggle_button: *mut GtkWidget,
    bottom_panel: *mut GtkWidget,
    stream_combo: *mut GtkWidget,
    stream_refresh_button: *mut GtkWidget,
    empty_button: *mut GtkWidget,
    mute_button: *mut GtkWidget,
    volume_scale: *mut GtkWidget,
    stream_settings_popover: *mut GtkWidget,
    quality_list_box: *mut GtkWidget,
    quality_status_label: *mut GtkWidget,
    status_label: *mut GtkWidget,
    streams: *mut StreamEntry,
    stream_count: c_uint,
    session: *mut PlayerSession,
    channel_switcher: *mut ChannelSwitcherOverlay,
    mpv_gl: *mut MpvRenderContext,
    chat_panel: *mut ChatPanel,
    settings: *mut AppSettings,
    title_cancel: *mut GCancellable,
    stream_info: *mut PlayerFooterStreamInfo,
    stream_quality: PlayerStreamQualityState,
    chat_paned_position: c_int,
    last_render_width: c_int,
    last_render_height: c_int,
    render_queued: c_int,
    event_queued: c_int,
    render_warmup_source: c_uint,
    render_warmup_frames: c_int,
    active_stream: c_uint,
    chat_visible: c_int,
    footer_hide_source: c_uint,
    title_refresh_source: c_uint,
    chat_position_source: c_uint,
    title_generation: c_uint,
    motion_tracker: PlayerMotionTracker,
    move_press_x: c_double,
    move_press_y: c_double,
    move_pressed: c_int,
    closing: c_int,
    fullscreen: c_int,
    fullscreen_callback: SinglePlayerFullscreenCallback,
    fullscreen_user_data: *mut c_void,
    stream_playing: c_int,
    title_fetch_in_progress: c_int,
}

struct StreamTitleCallbackData {
    state: *mut SinglePlayer,
    generation: c_uint,
}

struct StreamQualityCallbackData {
    state: *mut SinglePlayer,
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
pub type SinglePlayerFullscreenCallback = Option<unsafe extern "C" fn(*mut c_void)>;
pub type SinglePlayerSettingsCallback = Option<unsafe extern "C" fn(*mut c_void)>;

unsafe extern "C" {
    static epoxy_eglGetProcAddress: unsafe extern "C" fn(*const c_char) -> *mut c_void;
    static epoxy_glClear: unsafe extern "C" fn(c_uint);
    static epoxy_glClearColor: unsafe extern "C" fn(f32, f32, f32, f32);
    static epoxy_glGetIntegerv: unsafe extern "C" fn(c_uint, *mut c_int);

    fn g_ascii_strcasecmp(str1: *const c_char, str2: *const c_char) -> c_int;
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
    fn g_realloc_n(mem: *mut c_void, n_blocks: usize, n_block_bytes: usize) -> *mut c_void;
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
    fn gtk_button_get_child(button: *mut GtkButton) -> *mut GtkWidget;
    fn gtk_button_new() -> *mut GtkWidget;
    fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
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
    fn gtk_label_get_type() -> GType;
    fn gtk_label_new(str: *const c_char) -> *mut GtkWidget;
    fn gtk_label_set_text(label: *mut GtkLabel, str: *const c_char);
    fn gtk_label_set_xalign(label: *mut GtkLabel, xalign: f32);
    fn gtk_native_get_surface(self_: *mut GtkNative) -> *mut GdkSurface;
    fn gtk_overlay_add_overlay(overlay: *mut GtkOverlay, widget: *mut GtkWidget);
    fn gtk_overlay_new() -> *mut GtkWidget;
    fn gtk_overlay_set_child(overlay: *mut GtkOverlay, child: *mut GtkWidget);
    fn gtk_paned_get_position(paned: *mut GtkPaned) -> c_int;
    fn gtk_paned_get_type() -> GType;
    fn gtk_paned_new(orientation: c_int) -> *mut GtkWidget;
    fn gtk_paned_set_end_child(paned: *mut GtkPaned, child: *mut GtkWidget);
    fn gtk_paned_set_position(paned: *mut GtkPaned, position: c_int);
    fn gtk_paned_set_resize_end_child(paned: *mut GtkPaned, resize: c_int);
    fn gtk_paned_set_resize_start_child(paned: *mut GtkPaned, resize: c_int);
    fn gtk_paned_set_shrink_end_child(paned: *mut GtkPaned, resize: c_int);
    fn gtk_paned_set_shrink_start_child(paned: *mut GtkPaned, resize: c_int);
    fn gtk_paned_set_start_child(paned: *mut GtkPaned, child: *mut GtkWidget);
    fn gtk_paned_set_wide_handle(paned: *mut GtkPaned, wide: c_int);
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
    fn gtk_widget_add_controller(widget: *mut GtkWidget, controller: *mut c_void);
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    fn gtk_widget_get_height(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_get_native(widget: *mut GtkWidget) -> *mut GtkNative;
    fn gtk_widget_get_realized(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_get_scale_factor(widget: *mut GtkWidget) -> c_int;
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
    fn gtk_widget_get_type() -> GType;

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

unsafe fn has_prefix(value: *const c_char, prefix: &[u8]) -> bool {
    if value.is_null() {
        return false;
    }
    CStr::from_ptr(value).to_bytes().starts_with(prefix)
}

unsafe fn clear_object<T>(slot: *mut *mut T) {
    if !(*slot).is_null() {
        let old = *slot;
        *slot = ptr::null_mut();
        g_object_unref(old as *mut c_void);
    }
}

unsafe fn is_instance<T>(instance: *mut T, type_: GType) -> bool {
    !instance.is_null() && g_type_check_instance_is_a(instance as *mut GTypeInstance, type_) != 0
}

unsafe fn log_debug(format: *const c_char, first: *const c_char, second: *const c_char) {
    g_log(
        G_LOG_DOMAIN.as_ptr() as *const c_char,
        G_LOG_LEVEL_DEBUG,
        format,
        first,
        second,
    );
}

unsafe fn log_warning(format: *const c_char, message: *const c_char) {
    g_log(
        G_LOG_DOMAIN.as_ptr() as *const c_char,
        G_LOG_LEVEL_WARNING,
        format,
        message,
    );
}

unsafe fn stream_at(state: *mut SinglePlayer, index: c_uint) -> *mut StreamEntry {
    (*state).streams.add(index as usize)
}

unsafe fn update_empty_button(state: *mut SinglePlayer) {
    if !(*state).empty_button.is_null() {
        gtk_widget_set_visible(
            (*state).empty_button,
            ((*state).stream_playing == 0) as c_int,
        );
    }
    if !(*state).stream_refresh_button.is_null() {
        gtk_widget_set_visible((*state).stream_refresh_button, (*state).stream_playing);
    }
}

unsafe fn end_file_finishes_stream(end: *mut MpvEventEndFile) -> bool {
    end.is_null()
        || (*end).reason == MPV_END_FILE_REASON_EOF
        || (*end).reason == MPV_END_FILE_REASON_ERROR
}

fn clamp_chat_paned_position(position: c_int, width: c_int) -> c_int {
    if width <= 1 {
        return position;
    }

    let min_position = MIN_VIDEO_WIDTH.min(1.max(width - MIN_CHAT_WIDTH));
    let max_position = min_position.max(width - MIN_CHAT_WIDTH);
    position.clamp(min_position, max_position)
}

fn get_default_chat_paned_position(width: c_int) -> c_int {
    if width > 1 {
        width - DEFAULT_CHAT_WIDTH
    } else {
        0
    }
}

unsafe fn get_chat_paned_position_for_width(state: *mut SinglePlayer, width: c_int) -> c_int {
    let position = if (*state).chat_paned_position > 0 {
        (*state).chat_paned_position
    } else {
        get_default_chat_paned_position(width)
    };
    clamp_chat_paned_position(position, width)
}

unsafe extern "C" fn apply_chat_position(user_data: *mut c_void) -> c_int {
    let state = user_data as *mut SinglePlayer;

    if (*state).closing != 0 || (*state).main_area.is_null() {
        (*state).chat_position_source = 0;
        return G_SOURCE_REMOVE;
    }

    let width = gtk_widget_get_width((*state).main_area);
    if width <= 1 {
        return G_SOURCE_CONTINUE;
    }

    (*state).chat_paned_position = get_chat_paned_position_for_width(state, width);
    gtk_paned_set_position(
        (*state).main_area as *mut GtkPaned,
        (*state).chat_paned_position,
    );
    (*state).chat_position_source = 0;

    G_SOURCE_REMOVE
}

unsafe fn set_status(state: *mut SinglePlayer, message: *const c_char) {
    if !(*state).status_label.is_null() {
        gtk_label_set_text((*state).status_label as *mut GtkLabel, message);
    }
}

unsafe fn set_stream_title(
    state: *mut SinglePlayer,
    title: *const c_char,
    metadata: *const c_char,
) {
    player_footer_stream_info_set((*state).stream_info, title, metadata);
}

unsafe fn get_active_stream_channel(state: *mut SinglePlayer) -> *const c_char {
    if (*state).active_stream >= (*state).stream_count {
        return ptr::null();
    }
    (*stream_at(state, (*state).active_stream)).channel
}

unsafe fn get_active_stream_label(state: *mut SinglePlayer) -> *const c_char {
    if (*state).active_stream >= (*state).stream_count {
        return ptr::null();
    }
    (*stream_at(state, (*state).active_stream)).label
}

unsafe fn get_active_stream_url(state: *mut SinglePlayer) -> *const c_char {
    if (*state).active_stream >= (*state).stream_count {
        return ptr::null();
    }
    (*stream_at(state, (*state).active_stream)).url
}

unsafe fn clear_stream_qualities(state: *mut SinglePlayer) {
    player_stream_quality_state_clear(&mut (*state).stream_quality);
}

unsafe fn reset_stream_quality_selection(state: *mut SinglePlayer) {
    player_stream_quality_state_reset_selection(&mut (*state).stream_quality);
}

unsafe fn stream_settings_popover_is_visible(state: *mut SinglePlayer) -> bool {
    !(*state).stream_settings_popover.is_null()
        && gtk_widget_get_visible((*state).stream_settings_popover) != 0
}

unsafe fn stream_qualities_cache_is_valid(state: *mut SinglePlayer) -> bool {
    player_stream_quality_state_cache_is_valid(
        &mut (*state).stream_quality,
        STREAM_QUALITY_CACHE_SECONDS,
    ) != 0
}

unsafe fn reset_stream_title(state: *mut SinglePlayer) {
    (*state).title_generation = (*state).title_generation.wrapping_add(1);
    if !(*state).title_cancel.is_null() {
        g_cancellable_cancel((*state).title_cancel);
        clear_object(&mut (*state).title_cancel);
    }
    (*state).title_fetch_in_progress = FALSE;
    set_stream_title(state, cstr!(""), cstr!(""));
}

unsafe fn start_chat(state: *mut SinglePlayer, channel: *const c_char) {
    if is_nonempty(channel) {
        chat_panel_start((*state).chat_panel, channel);
    }
}

unsafe fn request_stream_title_update(state: *mut SinglePlayer, force: c_int) {
    let channel = get_active_stream_channel(state);

    if (*state).closing != 0 || (*state).stream_playing == 0 || !is_nonempty(channel) {
        return;
    }
    if (*state).title_fetch_in_progress != 0 && force == 0 {
        return;
    }

    if force != 0 {
        (*state).title_generation = (*state).title_generation.wrapping_add(1);
        if !(*state).title_cancel.is_null() {
            g_cancellable_cancel((*state).title_cancel);
            clear_object(&mut (*state).title_cancel);
        }
        (*state).title_fetch_in_progress = FALSE;
    }

    let data = Box::into_raw(Box::new(StreamTitleCallbackData {
        state,
        generation: (*state).title_generation.wrapping_add(1),
    }));
    (*state).title_generation = (*data).generation;

    (*state).title_cancel = g_cancellable_new();
    (*state).title_fetch_in_progress = TRUE;

    twitch_stream_info_fetch_current_stream_async(
        channel,
        (*state).title_cancel,
        Some(on_stream_title_fetched),
        data as *mut c_void,
    );
}

unsafe fn load_stream_url(
    state: *mut SinglePlayer,
    url: *const c_char,
    label: *const c_char,
    channel: *const c_char,
) {
    reset_stream_quality_selection(state);
    set_status(state, PLAYER_STARTING_STREAM_STATUS);
    (*state).stream_playing = TRUE;
    update_empty_button(state);
    update_stream_combo_label(state);
    reset_stream_title(state);
    start_chat(state, channel);
    request_stream_title_update(state, TRUE);
    player_session_load_stream((*state).session, url, label, channel);
}

unsafe fn reload_stream_with_quality(
    state: *mut SinglePlayer,
    quality: *const TwitchStreamQuality,
) {
    if quality.is_null() || !is_nonempty((*quality).url) {
        return;
    }

    let label = get_active_stream_label(state);
    let channel = get_active_stream_channel(state);
    if !is_nonempty(channel) {
        return;
    }

    player_stream_quality_state_select(&mut (*state).stream_quality, quality);

    set_status(state, PLAYER_STARTING_STREAM_STATUS);
    (*state).stream_playing = TRUE;
    update_empty_button(state);
    reset_stream_title(state);
    start_chat(state, channel);
    request_stream_title_update(state, TRUE);
    player_session_load_stream((*state).session, (*quality).url, label, channel);
}

unsafe fn reload_stream_auto(state: *mut SinglePlayer) {
    let url = get_active_stream_url(state);
    let label = get_active_stream_label(state);
    let channel = get_active_stream_channel(state);

    if !is_nonempty(url) {
        return;
    }

    player_stream_quality_state_select_auto(&mut (*state).stream_quality);
    load_stream_url(state, url, label, channel);
}

unsafe extern "C" fn on_stream_title_fetched(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    let data = user_data as *mut StreamTitleCallbackData;
    let state = (*data).state;
    let mut error: *mut GError = ptr::null_mut();
    let stream = twitch_stream_info_fetch_current_stream_finish(result, &mut error);

    if (*data).generation != (*state).title_generation {
        g_clear_error(&mut error);
        twitch_current_stream_free(stream);
        drop(Box::from_raw(data));
        return;
    }

    (*state).title_fetch_in_progress = FALSE;
    clear_object(&mut (*state).title_cancel);

    if (*state).closing != 0 || (*state).stream_playing == 0 {
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
                cstr!("stream title fetch failed: %s"),
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
    set_stream_title(state, title, metadata);
    g_free(title as *mut c_void);
    g_free(metadata as *mut c_void);
    twitch_current_stream_free(stream);
    drop(Box::from_raw(data));
}

unsafe extern "C" fn refresh_stream_title(user_data: *mut c_void) -> c_int {
    let state = user_data as *mut SinglePlayer;

    if (*state).closing != 0 {
        (*state).title_refresh_source = 0;
        return G_SOURCE_REMOVE;
    }

    request_stream_title_update(state, FALSE);
    G_SOURCE_CONTINUE
}

unsafe fn get_mpv(state: *mut SinglePlayer) -> *mut MpvHandle {
    player_session_get_mpv((*state).session)
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
    let state = user_data as *mut SinglePlayer;

    g_atomic_int_set(&mut (*state).render_queued, 0);

    if (*state).closing == 0 && !(*state).gl_area.is_null() {
        gtk_gl_area_queue_render((*state).gl_area as *mut GtkGLArea);
    }

    G_SOURCE_REMOVE
}

unsafe extern "C" fn warmup_video_render(user_data: *mut c_void) -> c_int {
    let state = user_data as *mut SinglePlayer;

    if (*state).closing != 0 || (*state).gl_area.is_null() || (*state).render_warmup_frames <= 0 {
        (*state).render_warmup_source = 0;
        return G_SOURCE_REMOVE;
    }

    (*state).render_warmup_frames -= 1;
    gtk_gl_area_queue_render((*state).gl_area as *mut GtkGLArea);
    G_SOURCE_CONTINUE
}

unsafe fn start_render_warmup(state: *mut SinglePlayer) {
    remove_source_if_active(&mut (*state).render_warmup_source);
    (*state).render_warmup_frames = 90;
    (*state).render_warmup_source =
        g_timeout_add(16, Some(warmup_video_render), state as *mut c_void);
}

unsafe extern "C" fn on_mpv_render_update(ctx: *mut c_void) {
    let state = ctx as *mut SinglePlayer;

    if g_atomic_int_compare_and_exchange(&mut (*state).render_queued, 0, 1) != 0 {
        g_idle_add_full(
            MPV_MAINLOOP_PRIORITY,
            Some(queue_mpv_render),
            state as *mut c_void,
            ptr::null_mut(),
        );
    }
}

unsafe extern "C" fn process_mpv_events(user_data: *mut c_void) -> c_int {
    let state = user_data as *mut SinglePlayer;

    g_atomic_int_set(&mut (*state).event_queued, 0);

    let mpv = get_mpv(state);
    if (*state).closing != 0 || mpv.is_null() {
        return G_SOURCE_REMOVE;
    }

    loop {
        let event = mpv_wait_event(mpv, 0.0);
        if event.is_null() || (*event).event_id == MPV_EVENT_NONE {
            break;
        }

        match (*event).event_id {
            MPV_EVENT_START_FILE => {
                set_status(state, cstr!("Loading stream"));
            }
            MPV_EVENT_FILE_LOADED => {
                (*state).stream_playing = TRUE;
                update_empty_button(state);
                set_status(state, cstr!("Playback running"));
            }
            MPV_EVENT_END_FILE => {
                let end = (*event).data as *mut MpvEventEndFile;
                if end_file_finishes_stream(end) {
                    (*state).stream_playing = FALSE;
                    update_empty_button(state);
                }
                if !end.is_null() && (*end).reason == MPV_END_FILE_REASON_ERROR {
                    set_status(state, cstr!("Stream could not be played"));
                } else {
                    set_status(state, cstr!("Stopped"));
                }
            }
            MPV_EVENT_VIDEO_RECONFIG => {}
            MPV_EVENT_LOG_MESSAGE => {
                let log = (*event).data as *mut MpvEventLogMessage;
                if !log.is_null() && !(*log).prefix.is_null() && !(*log).text.is_null() {
                    log_debug(cstr!("mpv[%s]: %s"), (*log).prefix, (*log).text);
                }
            }
            MPV_EVENT_SHUTDOWN => return G_SOURCE_REMOVE,
            _ => {}
        }
    }

    G_SOURCE_REMOVE
}

unsafe extern "C" fn on_mpv_wakeup(ctx: *mut c_void) {
    let state = ctx as *mut SinglePlayer;

    if g_atomic_int_compare_and_exchange(&mut (*state).event_queued, 0, 1) != 0 {
        g_idle_add_full(
            MPV_MAINLOOP_PRIORITY,
            Some(process_mpv_events),
            state as *mut c_void,
            ptr::null_mut(),
        );
    }
}

unsafe fn set_active_stream_from_channel(
    state: *mut SinglePlayer,
    channel: *const AppSettingsChannel,
) {
    if channel.is_null() {
        return;
    }

    for i in 0..(*state).stream_count {
        let stream = stream_at(state, i);
        if target_matches_stream_values(
            (*channel).url,
            (*channel).channel,
            (*stream).label,
            (*stream).channel,
            (*stream).url,
        ) {
            (*state).active_stream = i;
            return;
        }
    }

    let label = if is_nonempty((*channel).label) {
        (*channel).label
    } else if is_nonempty((*channel).channel) {
        (*channel).channel
    } else {
        (*channel).url
    };
    let index = (*state).stream_count;

    (*state).streams = g_realloc_n(
        (*state).streams as *mut c_void,
        ((*state).stream_count + 1) as usize,
        mem::size_of::<StreamEntry>(),
    ) as *mut StreamEntry;
    let stream = stream_at(state, index);
    (*stream).label = g_strdup(label);
    (*stream).channel = g_strdup((*channel).channel);
    (*stream).url = g_strdup((*channel).url);
    (*state).stream_count += 1;
    (*state).active_stream = index;
}

unsafe extern "C" fn activate_context_channel(
    channel: *const AppSettingsChannel,
    user_data: *mut c_void,
) {
    let state = user_data as *mut SinglePlayer;

    if channel.is_null() || !is_nonempty((*channel).url) {
        return;
    }

    set_active_stream_from_channel(state, channel);
    load_stream_url(state, (*channel).url, (*channel).label, (*channel).channel);
    show_footer(state);
}

unsafe fn play_selected_stream(state: *mut SinglePlayer) {
    if player_session_is_ready((*state).session) == 0 {
        log_warning(
            cstr!("%s"),
            cstr!("play requested, but mpv is not available"),
        );
        return;
    }

    let active = (*state).active_stream;

    if active >= (*state).stream_count {
        set_status(state, cstr!("No stream selected"));
        return;
    }

    let stream = stream_at(state, active);
    load_stream_url(state, (*stream).url, (*stream).label, (*stream).channel);
}

unsafe extern "C" fn on_volume_changed(range: *mut GtkRange, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;

    player_volume_sync_session_from_range((*state).session, range);
    if player_session_get_muted((*state).session) != 0 {
        player_volume_set_muted((*state).session, (*state).mute_button, FALSE);
    }
}

unsafe fn schedule_footer_hide(state: *mut SinglePlayer) {
    remove_source_if_active(&mut (*state).footer_hide_source);
    (*state).footer_hide_source = g_timeout_add(1800, Some(hide_footer), state as *mut c_void);
}

unsafe extern "C" fn hide_footer(user_data: *mut c_void) -> c_int {
    let state = user_data as *mut SinglePlayer;

    (*state).footer_hide_source = 0;

    if channel_switcher_overlay_is_visible((*state).channel_switcher) != 0
        || stream_settings_popover_is_visible(state)
    {
        schedule_footer_hide(state);
        return G_SOURCE_REMOVE;
    }

    if (*state).closing == 0 {
        if !(*state).bottom_panel.is_null() {
            gtk_widget_set_visible((*state).bottom_panel, FALSE);
        }
        if !(*state).chat_toggle_button.is_null() {
            gtk_widget_set_visible((*state).chat_toggle_button, FALSE);
        }
    }

    G_SOURCE_REMOVE
}

unsafe fn show_footer(state: *mut SinglePlayer) {
    if (*state).closing != 0 {
        return;
    }

    if !(*state).bottom_panel.is_null() {
        gtk_widget_set_visible((*state).bottom_panel, TRUE);
    }
    if !(*state).chat_toggle_button.is_null() {
        gtk_widget_set_visible((*state).chat_toggle_button, TRUE);
    }
    schedule_footer_hide(state);
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
    state: *mut SinglePlayer,
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

unsafe extern "C" fn on_video_motion(
    _controller: *mut GtkEventControllerMotion,
    x: c_double,
    y: c_double,
    user_data: *mut c_void,
) {
    let state = user_data as *mut SinglePlayer;

    if player_motion_tracker_ignore_stationary(
        &mut (*state).motion_tracker,
        state as *mut c_void,
        x,
        y,
    ) != 0
    {
        return;
    }

    show_footer(state);
}

unsafe fn request_fullscreen_toggle(state: *mut SinglePlayer) {
    if let Some(callback) = (*state).fullscreen_callback {
        callback((*state).fullscreen_user_data);
    }
    show_footer(state);
}

unsafe extern "C" fn on_video_pressed(
    _gesture: *mut GtkGestureClick,
    n_press: c_int,
    _x: c_double,
    _y: c_double,
    user_data: *mut c_void,
) {
    if n_press == 2 {
        request_fullscreen_toggle(user_data as *mut SinglePlayer);
    }
}

unsafe extern "C" fn on_video_legacy_event(
    _controller: *mut GtkEventControllerLegacy,
    event: *mut GdkEvent,
    user_data: *mut c_void,
) -> c_int {
    let state = user_data as *mut SinglePlayer;
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

unsafe extern "C" fn on_video_scroll(
    _controller: *mut GtkEventControllerScroll,
    dx: c_double,
    dy: c_double,
    user_data: *mut c_void,
) -> c_int {
    let state = user_data as *mut SinglePlayer;
    if channel_switcher_overlay_is_visible((*state).channel_switcher) != 0 {
        return GDK_EVENT_PROPAGATE;
    }

    if player_volume_apply_scroll((*state).volume_scale, dx, dy) == 0 {
        return GDK_EVENT_PROPAGATE;
    }

    show_footer(state);
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

    let state = user_data as *mut SinglePlayer;
    channel_switcher_overlay_show_at((*state).channel_switcher, x, y);
    show_footer(state);
}

unsafe extern "C" fn on_empty_button_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;
    channel_switcher_overlay_show_at((*state).channel_switcher, 0.0, 0.0);
    show_footer(state);
}

unsafe fn set_chat_visible(state: *mut SinglePlayer, visible: c_int) {
    if !is_instance((*state).main_area, gtk_paned_get_type()) || (*state).chat_panel.is_null() {
        return;
    }

    if visible != 0 {
        (*state).chat_visible = TRUE;
        gtk_paned_set_end_child(
            (*state).main_area as *mut GtkPaned,
            chat_panel_get_widget((*state).chat_panel),
        );
        let width = gtk_widget_get_width((*state).main_area);
        if width > 1 {
            (*state).chat_paned_position = get_chat_paned_position_for_width(state, width);
            gtk_paned_set_position(
                (*state).main_area as *mut GtkPaned,
                (*state).chat_paned_position,
            );
        } else if (*state).chat_position_source == 0 {
            (*state).chat_position_source =
                g_timeout_add(50, Some(apply_chat_position), state as *mut c_void);
        }
    } else {
        remove_source_if_active(&mut (*state).chat_position_source);
        let position = gtk_paned_get_position((*state).main_area as *mut GtkPaned);
        if position > 0 {
            (*state).chat_paned_position = position;
        }
        (*state).chat_visible = FALSE;
        gtk_paned_set_end_child((*state).main_area as *mut GtkPaned, ptr::null_mut());
    }

    gtk_widget_set_tooltip_text(
        (*state).chat_toggle_button,
        if visible != 0 {
            cstr!("Close chat")
        } else {
            cstr!("Open chat")
        },
    );
    gtk_button_set_child(
        (*state).chat_toggle_button as *mut GtkButton,
        player_chat_icon_new(if visible != 0 {
            PLAYER_CHAT_ICON_CLOSE
        } else {
            PLAYER_CHAT_ICON_OPEN
        }),
    );
}

unsafe extern "C" fn on_chat_toggle_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;
    set_chat_visible(state, ((*state).chat_visible == 0) as c_int);
    show_footer(state);
}

unsafe extern "C" fn on_mute_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;
    player_volume_toggle_muted((*state).session, (*state).mute_button);
    show_footer(state);
}

unsafe extern "C" fn on_quality_auto_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;

    reload_stream_auto(state);
    if !(*state).stream_settings_popover.is_null() {
        gtk_popover_popdown((*state).stream_settings_popover as *mut GtkPopover);
    }
    show_footer(state);
}

unsafe extern "C" fn on_quality_button_clicked(button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;
    let quality = g_object_get_data(button as *mut GObject, cstr!("stream-quality"))
        as *const TwitchStreamQuality;

    reload_stream_with_quality(state, quality);
    if !(*state).stream_settings_popover.is_null() {
        gtk_popover_popdown((*state).stream_settings_popover as *mut GtkPopover);
    }
    show_footer(state);
}

unsafe extern "C" fn on_stream_info_toggle_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    let state = user_data as *mut SinglePlayer;

    player_session_toggle_stream_info((*state).session);
    if !(*state).stream_settings_popover.is_null() {
        gtk_popover_popdown((*state).stream_settings_popover as *mut GtkPopover);
    }
    show_footer(state);
}

unsafe extern "C" fn on_stream_qualities_fetched(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    let data = user_data as *mut StreamQualityCallbackData;
    let state = (*data).state;
    let mut error: *mut GError = ptr::null_mut();
    let qualities = twitch_stream_info_fetch_stream_qualities_finish(result, &mut error);

    if (*data).generation != (*state).stream_quality.generation {
        if !qualities.is_null() {
            g_ptr_array_unref(qualities);
        }
        g_clear_error(&mut error);
        drop(Box::from_raw(data));
        return;
    }

    player_stream_quality_state_finish_fetch(&mut (*state).stream_quality, qualities);

    if !error.is_null() {
        if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0 {
            gtk_label_set_text(
                (*state).quality_status_label as *mut GtkLabel,
                cstr!("Qualities unavailable"),
            );
            g_log(
                G_LOG_DOMAIN.as_ptr() as *const c_char,
                G_LOG_LEVEL_DEBUG,
                cstr!("stream quality fetch failed: %s"),
                (*error).message,
            );
        }
        g_clear_error(&mut error);
        drop(Box::from_raw(data));
        return;
    }

    player_stream_quality_state_mark_fetched(&mut (*state).stream_quality);
    populate_quality_buttons(state);
    drop(Box::from_raw(data));
}

unsafe fn request_stream_qualities_update(state: *mut SinglePlayer, force: c_int) {
    let channel = get_active_stream_channel(state);

    if (*state).closing != 0 || !is_nonempty(channel) {
        return;
    }
    if (*state).stream_quality.fetch_in_progress != 0 && force == 0 {
        return;
    }
    if force == 0 && stream_qualities_cache_is_valid(state) {
        populate_quality_buttons(state);
        return;
    }

    if force != 0 {
        player_stream_quality_state_cancel_fetch(&mut (*state).stream_quality);
    }

    gtk_label_set_text(
        (*state).quality_status_label as *mut GtkLabel,
        cstr!("Loading..."),
    );

    let data = Box::into_raw(Box::new(StreamQualityCallbackData {
        state,
        generation: player_stream_quality_state_begin_fetch(&mut (*state).stream_quality),
    }));

    twitch_stream_info_fetch_stream_qualities_async(
        channel,
        (*state).stream_quality.cancel as *mut GCancellable,
        Some(on_stream_qualities_fetched),
        data as *mut c_void,
    );
}

unsafe fn populate_quality_buttons(state: *mut SinglePlayer) {
    player_stream_settings_quality_list_populate(
        (*state).quality_list_box,
        (*state).quality_status_label,
        (*state).stream_quality.qualities,
        (*state).stream_quality.selected_url,
        (*state).stream_quality.selected_label,
        on_quality_button_clicked as *const c_void,
        state as *mut c_void,
        on_quality_auto_clicked as *const c_void,
        state as *mut c_void,
    );
}

unsafe extern "C" fn on_stream_settings_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;

    if (*state).stream_settings_popover.is_null() {
        return;
    }
    if player_session_is_playing((*state).session) == 0
        || get_active_stream_channel(state).is_null()
    {
        show_footer(state);
        return;
    }

    request_stream_qualities_update(state, FALSE);
    gtk_popover_popup((*state).stream_settings_popover as *mut GtkPopover);
    show_footer(state);
}

unsafe extern "C" fn on_stream_refresh_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;

    if player_session_is_playing((*state).session) == 0 {
        return;
    }

    player_session_drop_buffers((*state).session);
    start_render_warmup(state);
    if !(*state).gl_area.is_null() {
        gtk_gl_area_queue_render((*state).gl_area as *mut GtkGLArea);
    }
    show_footer(state);
}

unsafe extern "C" fn on_stream_button_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;
    channel_switcher_overlay_show_at((*state).channel_switcher, 0.0, 0.0);
    show_footer(state);
}

unsafe fn update_stream_combo_label(state: *mut SinglePlayer) {
    if (*state).stream_combo.is_null() {
        return;
    }

    let child = gtk_button_get_child((*state).stream_combo as *mut GtkButton);
    if !is_instance(child, gtk_label_get_type()) {
        return;
    }

    if (*state).stream_playing == 0
        || (*state).stream_count == 0
        || (*state).active_stream >= (*state).stream_count
    {
        gtk_label_set_text(child as *mut GtkLabel, PLAYER_EMPTY_STREAM_LABEL);
        return;
    }

    gtk_label_set_text(
        child as *mut GtkLabel,
        (*stream_at(state, (*state).active_stream)).label,
    );
}

unsafe extern "C" fn on_gl_render(
    area: *mut GtkGLArea,
    _context: *mut GdkGLContext,
    user_data: *mut c_void,
) -> c_int {
    let state = user_data as *mut SinglePlayer;

    if (*state).mpv_gl.is_null() {
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

    let update_flags = mpv_render_context_update((*state).mpv_gl) as u64;
    let size_changed = width != (*state).last_render_width || height != (*state).last_render_height;
    let warming_up = (*state).render_warmup_frames > 0;

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

    let status = mpv_render_context_render((*state).mpv_gl, params.as_mut_ptr());
    if status < 0 {
        log_warning(cstr!("mpv render: %s"), mpv_error_string(status));
    } else {
        (*state).last_render_width = width;
        (*state).last_render_height = height;
    }

    TRUE
}

unsafe extern "C" fn on_gl_realize(area: *mut GtkGLArea, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;

    let mpv = get_mpv(state);
    if mpv.is_null() {
        g_log(
            G_LOG_DOMAIN.as_ptr() as *const c_char,
            G_LOG_LEVEL_DEBUG,
            cstr!("%s"),
            cstr!("GL realize skipped: mpv is not available"),
        );
        return;
    }

    gtk_gl_area_make_current(area);

    let gl_error = gtk_gl_area_get_error(area);
    if !gl_error.is_null() {
        log_warning(cstr!("GTK GL area error: %s"), (*gl_error).message);
        set_status(state, cstr!("OpenGL could not be initialized"));
        return;
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

    let status = mpv_render_context_create(&mut (*state).mpv_gl, mpv, params.as_mut_ptr());
    if status < 0 {
        log_warning(cstr!("mpv render context: %s"), mpv_error_string(status));
        set_status(state, cstr!("mpv rendering could not be started"));
        return;
    }

    mpv_render_context_set_update_callback(
        (*state).mpv_gl,
        Some(on_mpv_render_update),
        state as *mut c_void,
    );
    // The session may already be playing; force mpv to bind video to this new GLArea.
    player_session_reenable_video((*state).session);
    start_render_warmup(state);
    gtk_gl_area_queue_render(area);
}

unsafe fn clear_mpv_render_context(state: *mut SinglePlayer) {
    if !(*state).gl_area.is_null() && gtk_widget_get_realized((*state).gl_area) != 0 {
        gtk_gl_area_make_current((*state).gl_area as *mut GtkGLArea);
    }

    if !(*state).mpv_gl.is_null() {
        mpv_render_context_set_update_callback((*state).mpv_gl, None, ptr::null_mut());
        mpv_render_context_free((*state).mpv_gl);
        (*state).mpv_gl = ptr::null_mut();
    }
    remove_source_if_active(&mut (*state).render_warmup_source);
    (*state).last_render_width = 0;
    (*state).last_render_height = 0;
    (*state).render_warmup_frames = 0;
}

unsafe extern "C" fn on_gl_unrealize(area: *mut GtkGLArea, user_data: *mut c_void) {
    let state = user_data as *mut SinglePlayer;

    gtk_gl_area_make_current(area);
    clear_mpv_render_context(state);
}

unsafe fn create_controls(state: *mut SinglePlayer) -> *mut GtkWidget {
    let box_ = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 4);
    gtk_widget_add_css_class(box_, cstr!("player-footer"));
    gtk_widget_add_css_class(box_, cstr!("video-footer"));

    let stream_selector = gtk_overlay_new();
    gtk_widget_add_css_class(stream_selector, cstr!("stream-selector"));
    gtk_widget_set_halign(stream_selector, GTK_ALIGN_START);
    gtk_widget_set_size_request(stream_selector, STREAM_DROPDOWN_WIDTH, -1);
    gtk_widget_set_hexpand(stream_selector, FALSE);

    (*state).stream_combo = gtk_button_new();
    gtk_widget_add_css_class((*state).stream_combo, cstr!("stream-dropdown"));
    let stream_label = gtk_label_new(cstr!(""));
    gtk_widget_add_css_class(stream_label, cstr!("stream-button-label"));
    gtk_widget_set_halign(stream_label, GTK_ALIGN_START);
    gtk_widget_set_margin_end(stream_label, 22);
    gtk_label_set_xalign(stream_label as *mut GtkLabel, 0.0);
    gtk_button_set_child((*state).stream_combo as *mut GtkButton, stream_label);
    gtk_widget_set_halign((*state).stream_combo, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand((*state).stream_combo, TRUE);
    g_signal_connect_data(
        (*state).stream_combo as *mut c_void,
        cstr!("clicked"),
        on_stream_button_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    update_stream_combo_label(state);

    gtk_overlay_set_child(stream_selector as *mut GtkOverlay, (*state).stream_combo);

    (*state).stream_refresh_button =
        player_overlay_button_new(player_refresh_icon_new(), cstr!("Resync video"));
    gtk_widget_add_css_class(
        (*state).stream_refresh_button,
        cstr!("stream-refresh-button"),
    );
    gtk_widget_add_css_class(
        (*state).stream_refresh_button,
        cstr!("player-refresh-button"),
    );
    gtk_widget_set_halign((*state).stream_refresh_button, GTK_ALIGN_END);
    gtk_widget_set_valign((*state).stream_refresh_button, GTK_ALIGN_CENTER);
    gtk_widget_set_margin_end((*state).stream_refresh_button, 3);
    gtk_overlay_add_overlay(
        stream_selector as *mut GtkOverlay,
        (*state).stream_refresh_button,
    );
    g_signal_connect_data(
        (*state).stream_refresh_button as *mut c_void,
        cstr!("clicked"),
        on_stream_refresh_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    update_empty_button(state);

    (*state).stream_info = player_footer_stream_info_new();

    (*state).volume_scale = gtk_scale_new_with_range(
        GTK_ORIENTATION_HORIZONTAL,
        PLAYER_VOLUME_MIN,
        PLAYER_VOLUME_MAX,
        1.0,
    );
    gtk_widget_add_css_class((*state).volume_scale, cstr!("volume-scale"));
    gtk_range_set_value(
        (*state).volume_scale as *mut GtkRange,
        player_session_get_volume((*state).session),
    );
    gtk_widget_set_size_request((*state).volume_scale, 140, -1);
    gtk_scale_set_draw_value((*state).volume_scale as *mut GtkScale, FALSE);
    (*state).mute_button = player_volume_mute_button_new((*state).session);

    gtk_box_append(box_ as *mut GtkBox, stream_selector);
    gtk_box_append(
        box_ as *mut GtkBox,
        player_footer_stream_info_get_widget((*state).stream_info),
    );
    gtk_box_append(box_ as *mut GtkBox, (*state).mute_button);
    gtk_box_append(box_ as *mut GtkBox, (*state).volume_scale);

    (*state).chat_toggle_button = player_overlay_button_new(
        player_chat_icon_new(PLAYER_CHAT_ICON_OPEN),
        cstr!("Open chat"),
    );
    gtk_widget_add_css_class((*state).chat_toggle_button, cstr!("chat-toggle"));
    gtk_box_append(box_ as *mut GtkBox, (*state).chat_toggle_button);

    let stream_settings_button =
        player_overlay_button_new(player_stream_settings_icon_new(), cstr!("Stream settings"));
    gtk_widget_add_css_class(stream_settings_button, cstr!("stream-settings-button"));
    gtk_box_append(box_ as *mut GtkBox, stream_settings_button);

    let mut info_button: *mut GtkWidget = ptr::null_mut();
    (*state).stream_settings_popover = player_stream_settings_popover_new(
        stream_settings_button,
        &mut (*state).quality_list_box,
        &mut (*state).quality_status_label,
        &mut info_button,
    );

    g_signal_connect_data(
        stream_settings_button as *mut c_void,
        cstr!("clicked"),
        on_stream_settings_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        info_button as *mut c_void,
        cstr!("clicked"),
        on_stream_info_toggle_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        (*state).mute_button as *mut c_void,
        cstr!("clicked"),
        on_mute_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        (*state).chat_toggle_button as *mut c_void,
        cstr!("clicked"),
        on_chat_toggle_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        (*state).volume_scale as *mut c_void,
        cstr!("value-changed"),
        on_volume_changed as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );

    box_
}

unsafe fn extract_twitch_channel(target: *const c_char) -> *mut c_char {
    if !is_nonempty(target) {
        return ptr::null_mut();
    }

    let bytes = CStr::from_ptr(target).to_bytes();
    let prefix = b"twitch.tv/";
    let Some(prefix_index) = bytes
        .windows(prefix.len())
        .position(|window| window == prefix)
    else {
        return ptr::null_mut();
    };
    let mut start = prefix_index + prefix.len();

    while start < bytes.len() && bytes[start] == b'/' {
        start += 1;
    }

    let mut end = start;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    if end == start {
        return ptr::null_mut();
    }

    let mut channel = bytes[start..end].to_vec();
    channel.push(0);
    g_ascii_strdown(channel.as_ptr() as *const c_char, -1)
}

unsafe fn target_matches_stream_values(
    target: *const c_char,
    target_channel: *const c_char,
    label: *const c_char,
    channel: *const c_char,
    url: *const c_char,
) -> bool {
    if !is_nonempty(target) {
        return false;
    }

    g_ascii_strcasecmp(target, label) == 0
        || g_ascii_strcasecmp(target, channel) == 0
        || g_ascii_strcasecmp(target, url) == 0
        || (!target_channel.is_null() && g_ascii_strcasecmp(target_channel, channel) == 0)
}

unsafe fn init_streams(state: *mut SinglePlayer, target: *const c_char) {
    let settings_count = app_settings_get_channel_count((*state).settings);
    let base_count = settings_count;
    let mut extra_count = 0;
    let mut active_stream = 0;
    let mut startup_known = false;
    let startup_channel = extract_twitch_channel(target);
    let mut extra_label: *mut c_char = ptr::null_mut();
    let mut extra_channel: *mut c_char = ptr::null_mut();
    let mut extra_url: *mut c_char = ptr::null_mut();

    if is_nonempty(target) {
        for i in 0..base_count {
            let channel = app_settings_get_channel((*state).settings, i);
            if target_matches_stream_values(
                target,
                startup_channel,
                (*channel).label,
                (*channel).channel,
                (*channel).url,
            ) {
                active_stream = i;
                startup_known = true;
                break;
            }
        }
    }

    if !startup_known && is_nonempty(target) {
        extra_count = 1;

        if !startup_channel.is_null() {
            extra_label = g_strdup(startup_channel);
            extra_channel = g_strdup(startup_channel);
        } else if has_prefix(target, b"http://") || has_prefix(target, b"https://") {
            extra_label = g_strdup(target);
        } else {
            extra_label = g_strdup(target);
            extra_channel = g_ascii_strdown(target, -1);
        }

        if has_prefix(target, b"http://") || has_prefix(target, b"https://") {
            extra_url = g_strdup(target);
        } else {
            extra_url = g_strdup_printf(cstr!("https://www.twitch.tv/%s"), target);
        }

        active_stream = base_count;
    }

    (*state).stream_count = base_count + extra_count;
    (*state).streams = if (*state).stream_count > 0 {
        g_malloc0((*state).stream_count as usize * mem::size_of::<StreamEntry>())
            as *mut StreamEntry
    } else {
        ptr::null_mut()
    };

    for i in 0..base_count {
        let channel = app_settings_get_channel((*state).settings, i);
        let stream = stream_at(state, i);
        (*stream).label = g_strdup((*channel).label);
        (*stream).channel = g_strdup((*channel).channel);
        (*stream).url = g_strdup((*channel).url);
    }

    if extra_count == 1 {
        let stream = stream_at(state, base_count);
        (*stream).label = extra_label;
        (*stream).channel = extra_channel;
        (*stream).url = extra_url;
    }

    (*state).active_stream = active_stream;
    g_free(startup_channel as *mut c_void);
}

unsafe fn free_streams(state: *mut SinglePlayer) {
    if (*state).streams.is_null() {
        return;
    }

    for i in 0..(*state).stream_count {
        let stream = stream_at(state, i);
        g_free((*stream).label as *mut c_void);
        g_free((*stream).channel as *mut c_void);
        g_free((*stream).url as *mut c_void);
    }

    g_free((*state).streams as *mut c_void);
    (*state).streams = ptr::null_mut();
    (*state).stream_count = 0;
}

unsafe fn maybe_start_initial_stream(state: *mut SinglePlayer) {
    if is_nonempty((*state).startup_target) {
        play_selected_stream(state);
    }
}

unsafe fn single_player_destroy(state: *mut SinglePlayer) {
    if state.is_null() {
        return;
    }

    (*state).closing = TRUE;

    remove_source_if_active(&mut (*state).footer_hide_source);
    remove_source_if_active(&mut (*state).title_refresh_source);

    if !(*state).title_cancel.is_null() {
        g_cancellable_cancel((*state).title_cancel);
        clear_object(&mut (*state).title_cancel);
    }

    remove_source_if_active(&mut (*state).chat_position_source);
    remove_source_if_active(&mut (*state).render_warmup_source);
    clear_stream_qualities(state);
    clear_mpv_render_context(state);

    player_session_set_wakeup_callback((*state).session, None, ptr::null_mut());
    (*state).session = ptr::null_mut();

    if is_instance((*state).main_area, gtk_paned_get_type()) {
        let position = gtk_paned_get_position((*state).main_area as *mut GtkPaned);
        if position > 0 {
            (*state).chat_paned_position = position;
        }
        gtk_paned_set_end_child((*state).main_area as *mut GtkPaned, ptr::null_mut());
    }

    chat_panel_free((*state).chat_panel);
    (*state).chat_panel = ptr::null_mut();
    (*state).gl_area = ptr::null_mut();
    (*state).video_overlay = ptr::null_mut();
    (*state).main_area = ptr::null_mut();
    (*state).chat_toggle_button = ptr::null_mut();
    (*state).bottom_panel = ptr::null_mut();
    (*state).stream_combo = ptr::null_mut();
    (*state).stream_refresh_button = ptr::null_mut();
    if !(*state).stream_info.is_null() {
        player_footer_stream_info_free((*state).stream_info);
        (*state).stream_info = ptr::null_mut();
    }
    (*state).empty_button = ptr::null_mut();
    (*state).mute_button = ptr::null_mut();
    (*state).volume_scale = ptr::null_mut();
    if !(*state).stream_settings_popover.is_null() {
        gtk_widget_unparent((*state).stream_settings_popover);
    }
    (*state).stream_settings_popover = ptr::null_mut();
    (*state).quality_list_box = ptr::null_mut();
    (*state).quality_status_label = ptr::null_mut();
    (*state).status_label = ptr::null_mut();
    channel_switcher_overlay_free((*state).channel_switcher);
    (*state).channel_switcher = ptr::null_mut();
    free_streams(state);
    (*state).settings = ptr::null_mut();
}

unsafe fn add_weak_pointer<T>(object: *mut T, slot: *mut *mut T) {
    g_object_add_weak_pointer(object as *mut GObject, slot as *mut *mut c_void);
}

pub unsafe fn single_player_new<W>(
    window: *mut W,
    settings: *mut AppSettings,
    session: *mut PlayerSession,
    startup_target: *const c_char,
    auto_start: c_int,
    chat_paned_position: c_int,
    fullscreen_callback: SinglePlayerFullscreenCallback,
    fullscreen_user_data: *mut c_void,
    settings_callback: SinglePlayerSettingsCallback,
    settings_user_data: *mut c_void,
) -> *mut SinglePlayer {
    let window = window as *mut GtkWindow;
    let state = Box::into_raw(Box::new(SinglePlayer {
        startup_target,
        window: window as *mut GtkWidget,
        video_overlay: ptr::null_mut(),
        gl_area: ptr::null_mut(),
        main_area: ptr::null_mut(),
        chat_toggle_button: ptr::null_mut(),
        bottom_panel: ptr::null_mut(),
        stream_combo: ptr::null_mut(),
        stream_refresh_button: ptr::null_mut(),
        empty_button: ptr::null_mut(),
        mute_button: ptr::null_mut(),
        volume_scale: ptr::null_mut(),
        stream_settings_popover: ptr::null_mut(),
        quality_list_box: ptr::null_mut(),
        quality_status_label: ptr::null_mut(),
        status_label: ptr::null_mut(),
        streams: ptr::null_mut(),
        stream_count: 0,
        session,
        channel_switcher: ptr::null_mut(),
        mpv_gl: ptr::null_mut(),
        chat_panel: ptr::null_mut(),
        settings,
        title_cancel: ptr::null_mut(),
        stream_info: ptr::null_mut(),
        stream_quality: PlayerStreamQualityState::new(),
        chat_paned_position,
        last_render_width: 0,
        last_render_height: 0,
        render_queued: 0,
        event_queued: 0,
        render_warmup_source: 0,
        render_warmup_frames: 0,
        active_stream: 0,
        chat_visible: FALSE,
        footer_hide_source: 0,
        title_refresh_source: 0,
        chat_position_source: 0,
        title_generation: 0,
        motion_tracker: PlayerMotionTracker::new(),
        move_press_x: 0.0,
        move_press_y: 0.0,
        move_pressed: FALSE,
        closing: FALSE,
        fullscreen: FALSE,
        fullscreen_callback,
        fullscreen_user_data,
        stream_playing: FALSE,
        title_fetch_in_progress: FALSE,
    }));

    let session_target = player_session_get_url((*state).session);
    init_streams(
        state,
        if is_nonempty(session_target) {
            session_target
        } else {
            (*state).startup_target
        },
    );
    (*state).stream_playing = player_session_is_playing((*state).session);

    (*state).main_area = gtk_paned_new(GTK_ORIENTATION_HORIZONTAL);
    add_weak_pointer((*state).main_area, &mut (*state).main_area);
    gtk_widget_add_css_class((*state).main_area, cstr!("main-area"));
    gtk_widget_set_hexpand((*state).main_area, TRUE);
    gtk_widget_set_vexpand((*state).main_area, TRUE);
    gtk_paned_set_wide_handle((*state).main_area as *mut GtkPaned, FALSE);
    gtk_paned_set_resize_start_child((*state).main_area as *mut GtkPaned, TRUE);
    gtk_paned_set_shrink_start_child((*state).main_area as *mut GtkPaned, FALSE);
    gtk_paned_set_resize_end_child((*state).main_area as *mut GtkPaned, FALSE);
    gtk_paned_set_shrink_end_child((*state).main_area as *mut GtkPaned, FALSE);

    (*state).gl_area = gtk_gl_area_new();
    add_weak_pointer((*state).gl_area, &mut (*state).gl_area);
    configure_gl_area((*state).gl_area as *mut GtkGLArea);
    gtk_widget_set_hexpand((*state).gl_area, TRUE);
    gtk_widget_set_vexpand((*state).gl_area, TRUE);

    (*state).video_overlay = gtk_overlay_new();
    add_weak_pointer((*state).video_overlay, &mut (*state).video_overlay);
    gtk_widget_set_hexpand((*state).video_overlay, TRUE);
    gtk_widget_set_vexpand((*state).video_overlay, TRUE);
    gtk_overlay_set_child((*state).video_overlay as *mut GtkOverlay, (*state).gl_area);
    gtk_paned_set_start_child((*state).main_area as *mut GtkPaned, (*state).video_overlay);

    (*state).chat_panel = chat_panel_new(DEFAULT_CHAT_WIDTH);

    let video_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(video_click as *mut GtkGestureSingle, GDK_BUTTON_PRIMARY);
    g_signal_connect_data(
        video_click as *mut c_void,
        cstr!("pressed"),
        on_video_pressed as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*state).gl_area, video_click as *mut c_void);

    let context_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(context_click as *mut GtkGestureSingle, GDK_BUTTON_SECONDARY);
    g_signal_connect_data(
        context_click as *mut c_void,
        cstr!("pressed"),
        on_context_pressed as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*state).video_overlay, context_click as *mut c_void);

    let video_legacy = gtk_event_controller_legacy_new();
    g_signal_connect_data(
        video_legacy as *mut c_void,
        cstr!("event"),
        on_video_legacy_event as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*state).gl_area, video_legacy as *mut c_void);

    let video_scroll = gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_VERTICAL);
    gtk_event_controller_set_propagation_phase(video_scroll, GTK_PHASE_CAPTURE);
    g_signal_connect_data(
        video_scroll as *mut c_void,
        cstr!("scroll"),
        on_video_scroll as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*state).video_overlay, video_scroll as *mut c_void);

    (*state).empty_button = gtk_button_new();
    add_weak_pointer((*state).empty_button, &mut (*state).empty_button);
    let empty_icon_frame = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class(empty_icon_frame, cstr!("empty-stream-button-visible"));
    gtk_widget_set_halign(empty_icon_frame, GTK_ALIGN_CENTER);
    gtk_widget_set_valign(empty_icon_frame, GTK_ALIGN_CENTER);
    gtk_box_append(empty_icon_frame as *mut GtkBox, player_plus_icon_new());
    gtk_button_set_child((*state).empty_button as *mut GtkButton, empty_icon_frame);
    gtk_widget_add_css_class((*state).empty_button, cstr!("empty-stream-button"));
    gtk_widget_set_tooltip_text((*state).empty_button, cstr!("Select channel"));
    gtk_widget_set_halign((*state).empty_button, GTK_ALIGN_CENTER);
    gtk_widget_set_valign((*state).empty_button, GTK_ALIGN_CENTER);
    g_signal_connect_data(
        (*state).empty_button as *mut c_void,
        cstr!("clicked"),
        on_empty_button_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_overlay_add_overlay(
        (*state).video_overlay as *mut GtkOverlay,
        (*state).empty_button,
    );
    update_empty_button(state);

    (*state).bottom_panel = create_controls(state);
    gtk_widget_set_halign((*state).bottom_panel, GTK_ALIGN_FILL);
    gtk_widget_set_valign((*state).bottom_panel, GTK_ALIGN_END);
    gtk_overlay_add_overlay(
        (*state).video_overlay as *mut GtkOverlay,
        (*state).bottom_panel,
    );
    (*state).channel_switcher = channel_switcher_overlay_new(
        (*state).video_overlay as *mut GtkOverlay,
        (*state).settings,
        Some(activate_context_channel),
        state as *mut c_void,
        settings_callback,
        settings_user_data,
    );
    (*state).title_refresh_source = g_timeout_add_seconds(
        STREAM_TITLE_REFRESH_SECONDS,
        Some(refresh_stream_title),
        state as *mut c_void,
    );

    let video_motion = gtk_event_controller_motion_new();
    gtk_event_controller_set_propagation_phase(video_motion, GTK_PHASE_CAPTURE);
    g_signal_connect_data(
        video_motion as *mut c_void,
        cstr!("motion"),
        on_video_motion as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*state).video_overlay, video_motion as *mut c_void);

    if player_session_is_ready((*state).session) == 0 {
        set_status(state, cstr!("mpv could not be initialized"));
        gtk_widget_set_sensitive((*state).stream_combo, FALSE);
    } else {
        player_session_set_hwdec_enabled(
            (*state).session,
            app_settings_get_hwdec_enabled((*state).settings),
        );
        player_session_set_wakeup_callback(
            (*state).session,
            Some(on_mpv_wakeup),
            state as *mut c_void,
        );
    }

    g_signal_connect_data(
        (*state).gl_area as *mut c_void,
        cstr!("realize"),
        on_gl_realize as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        (*state).gl_area as *mut c_void,
        cstr!("unrealize"),
        on_gl_unrealize as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        (*state).gl_area as *mut c_void,
        cstr!("render"),
        on_gl_render as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );

    schedule_footer_hide(state);

    if player_session_is_playing((*state).session) != 0 {
        update_stream_combo_label(state);
        set_status(state, cstr!("Playback running"));
        start_chat(state, get_active_stream_channel(state));
        request_stream_title_update(state, TRUE);
    } else if player_session_is_ready((*state).session) != 0 && auto_start != 0 {
        maybe_start_initial_stream(state);
    }

    state
}

pub unsafe fn single_player_get_widget<W>(player: *mut SinglePlayer) -> *mut W {
    if !player.is_null() && is_instance((*player).main_area, gtk_widget_get_type()) {
        (*player).main_area as *mut W
    } else {
        ptr::null_mut()
    }
}

pub unsafe fn single_player_dup_current_target(player: *mut SinglePlayer) -> *mut c_char {
    if player.is_null() || player_session_is_playing((*player).session) == 0 {
        return ptr::null_mut();
    }

    let channel = player_session_get_channel((*player).session);
    if is_nonempty(channel) {
        return g_strdup(channel);
    }

    player_session_dup_url((*player).session)
}

pub unsafe fn single_player_get_chat_paned_position(player: *mut SinglePlayer) -> c_int {
    if player.is_null() {
        return 0;
    }

    if is_instance((*player).main_area, gtk_paned_get_type()) {
        let position = gtk_paned_get_position((*player).main_area as *mut GtkPaned);
        if position > 0 {
            (*player).chat_paned_position = position;
        }
    }

    (*player).chat_paned_position
}

pub unsafe fn single_player_set_fullscreen(player: *mut SinglePlayer, fullscreen: c_int) {
    if !player.is_null() {
        (*player).fullscreen = fullscreen;
    }
}

pub unsafe fn single_player_show_overlay(player: *mut SinglePlayer) {
    if !player.is_null() {
        show_footer(player);
    }
}

pub unsafe fn single_player_handle_key(
    player: *mut SinglePlayer,
    keyval: c_uint,
    modifiers: c_uint,
) -> c_int {
    if player.is_null() || (modifiers & GDK_CONTROL_MASK) != 0 {
        return GDK_EVENT_PROPAGATE;
    }

    if channel_switcher_overlay_is_visible((*player).channel_switcher) != 0 {
        return GDK_EVENT_PROPAGATE;
    }

    if keyval != GDK_KEY_M_LOWER && keyval != GDK_KEY_M_UPPER {
        return GDK_EVENT_PROPAGATE;
    }

    player_volume_toggle_muted((*player).session, (*player).mute_button);
    show_footer(player);
    GDK_EVENT_STOP
}

pub unsafe fn single_player_set_settings(player: *mut SinglePlayer, settings: *mut AppSettings) {
    if player.is_null() {
        return;
    }

    let current_channel = if (*player).active_stream < (*player).stream_count {
        g_strdup((*stream_at(player, (*player).active_stream)).channel)
    } else {
        ptr::null_mut()
    };

    (*player).settings = settings;
    player_session_set_hwdec_enabled((*player).session, app_settings_get_hwdec_enabled(settings));
    channel_switcher_overlay_set_settings((*player).channel_switcher, settings);
    free_streams(player);
    init_streams(player, current_channel);
    g_free(current_channel as *mut c_void);
    update_stream_combo_label(player);
    show_footer(player);
}

pub unsafe fn single_player_free(player: *mut SinglePlayer) {
    single_player_destroy(player);
}
