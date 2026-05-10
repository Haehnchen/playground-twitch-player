#![allow(clashing_extern_declarations)]

use std::env;
use std::ffi::{c_char, c_double, c_int, c_uint, c_void, CStr, CString};
use std::os::unix::ffi::OsStringExt;
use std::ptr;

use crate::grid_player::{
    grid_player_dup_first_target, grid_player_free, grid_player_get_widget, grid_player_new,
    grid_player_set_fullscreen, grid_player_set_settings, grid_player_start,
    grid_player_take_first_session, GridAppState as GridPlayer,
};
use crate::player_icons::{
    player_layout_icon_new, player_settings_icon_new, player_window_icon_new,
};
use crate::player_motion::{player_motion_tracker_ignore_stationary, PlayerMotionTracker};
use crate::player_overlay_controls::player_overlay_button_new;
use crate::player_session::{
    player_session_free, player_session_get_channel, player_session_is_playing, player_session_new,
    player_session_set_hwdec_enabled, PlayerSession,
};
use crate::player_style::{player_style_install_footer_css, player_style_install_overlay_css};
use crate::settings::{
    app_settings_free, app_settings_get_hwdec_enabled, app_settings_load, AppSettings, GError,
};
use crate::settings_window::settings_window_show;
use crate::single_player::{
    single_player_dup_current_target, single_player_free, single_player_get_chat_paned_position,
    single_player_get_widget, single_player_handle_key, single_player_new,
    single_player_set_fullscreen, single_player_set_settings, single_player_show_overlay,
    SinglePlayer,
};

macro_rules! cstr {
    ($value:literal) => {
        concat!($value, "\0").as_ptr() as *const c_char
    };
}

const OVERLAY_HIDE_DELAY_MS: c_uint = 1800;
const MAXIMIZE_RESTORE_ATTEMPTS: c_uint = 12;
const DEFAULT_WINDOW_WIDTH: c_int = 1100;
const DEFAULT_WINDOW_HEIGHT: c_int = (DEFAULT_WINDOW_WIDTH * 9 + 8) / 16;
const GRID_PLAYER_MAX_TILES: usize = 4;

const FALSE: c_int = 0;
const TRUE: c_int = 1;
const G_SOURCE_REMOVE: c_int = 0;
const G_SOURCE_CONTINUE: c_int = 1;
const G_APPLICATION_NON_UNIQUE: c_int = 1 << 5;
const G_LOG_LEVEL_DEBUG: c_int = 1 << 7;
const G_LOG_DOMAIN: &[u8] = b"twitch-player\0";
const APP_ICON_SVG: &[u8] =
    include_bytes!("../data/icons/hicolor/scalable/apps/local.twitch-player.svg");

const GTK_ALIGN_FILL: c_int = 0;
const GTK_ALIGN_START: c_int = 1;
const GTK_ALIGN_END: c_int = 2;
const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
const GTK_ORIENTATION_VERTICAL: c_int = 1;
const GTK_PHASE_CAPTURE: c_int = 1;
const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: c_uint = 600;

const GDK_BUTTON_PRIMARY: c_int = 1;
const GDK_EVENT_PROPAGATE: c_int = FALSE;
const GDK_SURFACE_EDGE_NORTH_WEST: c_int = 0;
const GDK_SURFACE_EDGE_NORTH: c_int = 1;
const GDK_SURFACE_EDGE_NORTH_EAST: c_int = 2;
const GDK_SURFACE_EDGE_WEST: c_int = 3;
const GDK_SURFACE_EDGE_EAST: c_int = 4;
const GDK_SURFACE_EDGE_SOUTH_WEST: c_int = 5;
const GDK_SURFACE_EDGE_SOUTH: c_int = 6;
const GDK_SURFACE_EDGE_SOUTH_EAST: c_int = 7;

const CONTENT_MODE_SINGLE: c_int = 0;
const CONTENT_MODE_GRID: c_int = 1;
const SETTINGS_WINDOW_PAGE_GENERAL: c_int = 0;
const SETTINGS_WINDOW_PAGE_CHANNELS: c_int = 1;
const PLAYER_WINDOW_ICON_MINIMIZE: c_int = 0;
const PLAYER_WINDOW_ICON_FULLSCREEN: c_int = 1;
const PLAYER_WINDOW_ICON_CLOSE: c_int = 2;
const PLAYER_LAYOUT_ICON_SINGLE: c_int = 0;
const PLAYER_LAYOUT_ICON_GRID: c_int = 1;

struct AppState {
    window: *mut GtkWidget,
    root_overlay: *mut GtkWidget,
    top_left_controls: *mut GtkWidget,
    top_controls: *mut GtkWidget,
    settings_button: *mut GtkWidget,
    layout_button: *mut GtkWidget,
    settings: *mut AppSettings,
    primary_session: *mut PlayerSession,
    single_player: *mut SinglePlayer,
    grid_player: *mut GridPlayer,
    startup_target: *const c_char,
    grid_targets: *const *const c_char,
    grid_target_count: c_uint,
    single_target: *mut c_char,
    has_single_target_handoff: c_int,
    single_chat_paned_position: c_int,
    content_mode: c_int,
    overlay_hide_source: c_uint,
    maximize_restore_source: c_uint,
    maximize_restore_attempts: c_uint,
    motion_tracker: PlayerMotionTracker,
    closing: c_int,
    fullscreen: c_int,
    window_maximized: c_int,
    was_maximized_before_fullscreen: c_int,
    restore_maximized_after_fullscreen: c_int,
    restore_window_width: c_int,
    restore_window_height: c_int,
}

struct StartupConfig {
    startup_target: *const c_char,
    grid_targets: *const *const c_char,
    grid_target_count: c_uint,
    start_in_grid: c_int,
}

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
struct GdkEventSequence {
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
struct GParamSpec {
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
struct GtkApplication {
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
struct GtkEventControllerKey {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkEventControllerMotion {
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
struct GtkIconTheme {
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
struct GtkStyleProvider {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkWidget {
    _private: [u8; 0],
}

#[repr(C)]
struct GtkWindow {
    _private: [u8; 0],
}

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);
type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;
type GType = usize;

unsafe extern "C" {
    fn g_application_run(application: *mut c_void, argc: c_int, argv: *mut *mut c_char) -> c_int;
    fn g_ascii_strcasecmp(str1: *const c_char, str2: *const c_char) -> c_int;
    fn g_ascii_strdown(str: *const c_char, len: isize) -> *mut c_char;
    fn g_build_filename(first_element: *const c_char, ...) -> *mut c_char;
    fn g_canonicalize_filename(filename: *const c_char, relative_to: *const c_char) -> *mut c_char;
    fn g_clear_error(error: *mut *mut GError);
    fn g_file_read_link(filename: *const c_char, error: *mut *mut GError) -> *mut c_char;
    fn g_file_set_contents(
        filename: *const c_char,
        contents: *const c_char,
        length: isize,
        error: *mut *mut GError,
    ) -> c_int;
    fn g_free(mem: *mut c_void);
    fn g_get_current_dir() -> *mut c_char;
    fn g_get_user_data_dir() -> *const c_char;
    fn g_log(log_domain: *const c_char, log_level: c_int, format: *const c_char, ...);
    fn g_main_context_find_source_by_id(context: *mut c_void, source_id: c_uint) -> *mut GSource;
    fn g_mkdir_with_parents(pathname: *const c_char, mode: c_int) -> c_int;
    fn g_object_add_weak_pointer(object: *mut GObject, weak_pointer_location: *mut *mut c_void);
    fn g_object_get_data(object: *mut GObject, key: *const c_char) -> *mut c_void;
    fn g_object_set_data(object: *mut GObject, key: *const c_char, data: *mut c_void);
    fn g_object_set_data_full(
        object: *mut GObject,
        key: *const c_char,
        data: *mut c_void,
        destroy: Option<GDestroyNotify>,
    );
    fn g_object_unref(object: *mut c_void);
    fn g_path_is_absolute(file_name: *const c_char) -> c_int;
    fn g_set_application_name(application_name: *const c_char);
    fn g_set_prgname(prgname: *const c_char);
    fn g_setenv(variable: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *const c_void,
        data: *mut c_void,
        destroy_data: *mut c_void,
        connect_flags: c_int,
    ) -> usize;
    fn g_source_destroy(source: *mut GSource);
    fn g_strcmp0(str1: *const c_char, str2: *const c_char) -> c_int;
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn g_strdup_printf(format: *const c_char, ...) -> *mut c_char;
    fn g_timeout_add(interval: c_uint, function: Option<GSourceFunc>, data: *mut c_void) -> c_uint;
    fn g_type_check_instance_is_a(instance: *mut GTypeInstance, iface_type: GType) -> c_int;

    fn gdk_display_get_default() -> *mut GdkDisplay;
    fn gdk_event_get_device(event: *mut GdkEvent) -> *mut GdkDevice;
    fn gdk_event_get_position(event: *mut GdkEvent, x: *mut c_double, y: *mut c_double) -> c_int;
    fn gdk_event_get_time(event: *mut GdkEvent) -> c_uint;
    fn gdk_toplevel_begin_resize(
        toplevel: *mut GdkToplevel,
        edge: c_int,
        device: *mut GdkDevice,
        button: c_int,
        x: c_double,
        y: c_double,
        timestamp: c_uint,
    );
    fn gdk_toplevel_get_type() -> GType;

    fn gtk_application_new(application_id: *const c_char, flags: c_int) -> *mut GtkApplication;
    fn gtk_application_window_new(application: *mut GtkApplication) -> *mut GtkWidget;
    fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    fn gtk_css_provider_load_from_string(css_provider: *mut GtkCssProvider, string: *const c_char);
    fn gtk_css_provider_new() -> *mut GtkCssProvider;
    fn gtk_event_controller_key_new() -> *mut GtkEventController;
    fn gtk_event_controller_motion_new() -> *mut GtkEventController;
    fn gtk_event_controller_set_propagation_phase(
        controller: *mut GtkEventController,
        phase: c_int,
    );
    fn gtk_gesture_click_new() -> *mut GtkGesture;
    fn gtk_gesture_get_last_event(
        gesture: *mut GtkGesture,
        sequence: *mut GdkEventSequence,
    ) -> *mut GdkEvent;
    fn gtk_gesture_get_last_updated_sequence(gesture: *mut GtkGesture) -> *mut GdkEventSequence;
    fn gtk_gesture_single_set_button(gesture: *mut GtkGestureSingle, button: c_uint);
    fn gtk_icon_theme_add_search_path(icon_theme: *mut GtkIconTheme, path: *const c_char);
    fn gtk_icon_theme_get_for_display(display: *mut GdkDisplay) -> *mut GtkIconTheme;
    fn gtk_native_get_surface(self_: *mut GtkNative) -> *mut GdkSurface;
    fn gtk_overlay_add_overlay(overlay: *mut GtkOverlay, widget: *mut GtkWidget);
    fn gtk_overlay_get_type() -> GType;
    fn gtk_overlay_new() -> *mut GtkWidget;
    fn gtk_overlay_set_child(overlay: *mut GtkOverlay, child: *mut GtkWidget);
    fn gtk_style_context_add_provider_for_display(
        display: *mut GdkDisplay,
        provider: *mut GtkStyleProvider,
        priority: c_uint,
    );
    fn gtk_widget_add_controller(widget: *mut GtkWidget, controller: *mut c_void);
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    fn gtk_widget_get_height(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_get_native(widget: *mut GtkWidget) -> *mut GtkNative;
    fn gtk_widget_get_width(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_set_cursor_from_name(widget: *mut GtkWidget, name: *const c_char);
    fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    fn gtk_widget_set_tooltip_text(widget: *mut GtkWidget, text: *const c_char);
    fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_visible(widget: *mut GtkWidget, visible: c_int);
    fn gtk_window_close(window: *mut GtkWindow);
    fn gtk_window_fullscreen(window: *mut GtkWindow);
    fn gtk_window_get_type() -> GType;
    fn gtk_window_is_fullscreen(window: *mut GtkWindow) -> c_int;
    fn gtk_window_is_maximized(window: *mut GtkWindow) -> c_int;
    fn gtk_window_maximize(window: *mut GtkWindow);
    fn gtk_window_minimize(window: *mut GtkWindow);
    fn gtk_window_present(window: *mut GtkWindow);
    fn gtk_window_set_child(window: *mut GtkWindow, child: *mut GtkWidget);
    fn gtk_window_set_decorated(window: *mut GtkWindow, setting: c_int);
    fn gtk_window_set_default_icon_name(name: *const c_char);
    fn gtk_window_set_default_size(window: *mut GtkWindow, width: c_int, height: c_int);
    fn gtk_window_set_icon_name(window: *mut GtkWindow, name: *const c_char);
    fn gtk_window_set_title(window: *mut GtkWindow, title: *const c_char);
    fn gtk_window_unfullscreen(window: *mut GtkWindow);

}

unsafe fn is_nonempty(value: *const c_char) -> bool {
    !value.is_null() && *value != 0
}

unsafe fn dup_bytes(bytes: &[u8]) -> *mut c_char {
    let mut value = Vec::with_capacity(bytes.len() + 1);
    value.extend_from_slice(bytes);
    value.push(0);
    g_strdup(value.as_ptr() as *const c_char)
}

unsafe fn c_app_id() -> *const c_char {
    cstr!("local.twitchplayer")
}

unsafe fn dup_twitch_channel_name(value: *const c_char) -> *mut c_char {
    if !is_nonempty(value) {
        return ptr::null_mut();
    }

    let bytes = CStr::from_ptr(value).to_bytes();
    let prefix = b"twitch.tv/";
    let prefix_index = bytes
        .windows(prefix.len())
        .position(|window| window == prefix);
    let from_twitch_url = prefix_index.is_some();
    let mut start = prefix_index.map(|index| index + prefix.len()).unwrap_or(0);

    while start < bytes.len() && bytes[start] == b'/' {
        start += 1;
    }

    let mut end = start;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    if end == start || (!from_twitch_url && end != bytes.len()) {
        return ptr::null_mut();
    }

    let mut channel = bytes[start..end].to_vec();
    channel.push(0);
    g_ascii_strdown(channel.as_ptr() as *const c_char, (end - start) as isize)
}

unsafe fn configure_rendering_defaults() {
    g_setenv(cstr!("GSK_RENDERER"), cstr!("gl"), FALSE);
}

unsafe fn remove_source_if_active(source_id: *mut c_uint) {
    if source_id.is_null() || *source_id == 0 {
        return;
    }

    let source = g_main_context_find_source_by_id(ptr::null_mut(), *source_id);
    if !source.is_null() {
        g_source_destroy(source);
    }
    *source_id = 0;
}

unsafe extern "C" fn hide_window_overlay(user_data: *mut c_void) -> c_int {
    let state = user_data as *mut AppState;
    (*state).overlay_hide_source = 0;

    if (*state).closing == 0 {
        gtk_widget_set_visible((*state).top_left_controls, FALSE);
        gtk_widget_set_visible((*state).top_controls, FALSE);
    }

    G_SOURCE_REMOVE
}

unsafe fn schedule_window_overlay_hide(state: *mut AppState) {
    remove_source_if_active(&mut (*state).overlay_hide_source);
    (*state).overlay_hide_source = g_timeout_add(
        OVERLAY_HIDE_DELAY_MS,
        Some(hide_window_overlay),
        state as *mut c_void,
    );
}

unsafe fn show_window_overlay(state: *mut AppState) {
    if (*state).closing != 0 {
        return;
    }

    gtk_widget_set_visible((*state).top_left_controls, TRUE);
    gtk_widget_set_visible((*state).top_controls, TRUE);
    if !(*state).single_player.is_null() {
        single_player_show_overlay((*state).single_player);
    }
    schedule_window_overlay_hide(state);
}

unsafe fn is_instance<T>(instance: *mut T, type_: GType) -> bool {
    !instance.is_null() && g_type_check_instance_is_a(instance as *mut GTypeInstance, type_) != 0
}

unsafe fn get_toplevel_event_data(
    window: *mut GtkWidget,
    gesture: *mut GtkGesture,
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
    let sequence = gtk_gesture_get_last_updated_sequence(gesture);
    let event = gtk_gesture_get_last_event(gesture, sequence);

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

unsafe fn begin_window_resize(state: *mut AppState, gesture: *mut GtkGesture, edge: c_int) {
    if (*state).fullscreen != 0 {
        return;
    }

    let mut toplevel: *mut GdkToplevel = ptr::null_mut();
    let mut device: *mut GdkDevice = ptr::null_mut();
    let mut x = 0.0;
    let mut y = 0.0;
    let mut timestamp = 0;

    if get_toplevel_event_data(
        (*state).window,
        gesture,
        &mut toplevel,
        &mut device,
        &mut x,
        &mut y,
        &mut timestamp,
    ) != 0
    {
        gdk_toplevel_begin_resize(toplevel, edge, device, GDK_BUTTON_PRIMARY, x, y, timestamp);
    }
}

unsafe extern "C" fn on_resize_pressed(
    gesture: *mut GtkGestureClick,
    n_press: c_int,
    _x: c_double,
    _y: c_double,
    user_data: *mut c_void,
) {
    if n_press != 1 {
        return;
    }

    let edge = g_object_get_data(gesture as *mut GObject, cstr!("resize-edge")) as isize as c_int;
    begin_window_resize(user_data as *mut AppState, gesture as *mut GtkGesture, edge);
}

unsafe fn create_resize_handle(
    state: *mut AppState,
    edge: c_int,
    halign: c_int,
    valign: c_int,
    width: c_int,
    height: c_int,
    cursor: *const c_char,
) -> *mut GtkWidget {
    let handle = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class(handle, cstr!("resize-handle"));
    gtk_widget_set_halign(handle, halign);
    gtk_widget_set_valign(handle, valign);
    gtk_widget_set_size_request(handle, width, height);
    gtk_widget_set_cursor_from_name(handle, cursor);

    if halign == GTK_ALIGN_FILL {
        gtk_widget_set_hexpand(handle, TRUE);
    }
    if valign == GTK_ALIGN_FILL {
        gtk_widget_set_vexpand(handle, TRUE);
    }

    let click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(click as *mut GtkGestureSingle, GDK_BUTTON_PRIMARY as c_uint);
    g_object_set_data(
        click as *mut GObject,
        cstr!("resize-edge"),
        edge as isize as *mut c_void,
    );
    g_signal_connect_data(
        click as *mut c_void,
        cstr!("pressed"),
        on_resize_pressed as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller(handle, click as *mut c_void);

    handle
}

unsafe fn add_resize_handles(overlay: *mut GtkOverlay, state: *mut AppState) {
    gtk_overlay_add_overlay(
        overlay,
        create_resize_handle(
            state,
            GDK_SURFACE_EDGE_NORTH,
            GTK_ALIGN_FILL,
            GTK_ALIGN_START,
            -1,
            6,
            cstr!("n-resize"),
        ),
    );
    gtk_overlay_add_overlay(
        overlay,
        create_resize_handle(
            state,
            GDK_SURFACE_EDGE_SOUTH,
            GTK_ALIGN_FILL,
            GTK_ALIGN_END,
            -1,
            6,
            cstr!("s-resize"),
        ),
    );
    gtk_overlay_add_overlay(
        overlay,
        create_resize_handle(
            state,
            GDK_SURFACE_EDGE_WEST,
            GTK_ALIGN_START,
            GTK_ALIGN_FILL,
            6,
            -1,
            cstr!("w-resize"),
        ),
    );
    gtk_overlay_add_overlay(
        overlay,
        create_resize_handle(
            state,
            GDK_SURFACE_EDGE_EAST,
            GTK_ALIGN_END,
            GTK_ALIGN_FILL,
            6,
            -1,
            cstr!("e-resize"),
        ),
    );
    gtk_overlay_add_overlay(
        overlay,
        create_resize_handle(
            state,
            GDK_SURFACE_EDGE_NORTH_WEST,
            GTK_ALIGN_START,
            GTK_ALIGN_START,
            12,
            12,
            cstr!("nw-resize"),
        ),
    );
    gtk_overlay_add_overlay(
        overlay,
        create_resize_handle(
            state,
            GDK_SURFACE_EDGE_NORTH_EAST,
            GTK_ALIGN_END,
            GTK_ALIGN_START,
            12,
            12,
            cstr!("ne-resize"),
        ),
    );
    gtk_overlay_add_overlay(
        overlay,
        create_resize_handle(
            state,
            GDK_SURFACE_EDGE_SOUTH_WEST,
            GTK_ALIGN_START,
            GTK_ALIGN_END,
            12,
            12,
            cstr!("sw-resize"),
        ),
    );
    gtk_overlay_add_overlay(
        overlay,
        create_resize_handle(
            state,
            GDK_SURFACE_EDGE_SOUTH_EAST,
            GTK_ALIGN_END,
            GTK_ALIGN_END,
            12,
            12,
            cstr!("se-resize"),
        ),
    );
}

unsafe extern "C" fn restore_maximized_after_fullscreen(user_data: *mut c_void) -> c_int {
    let state = user_data as *mut AppState;
    let window = if is_instance((*state).window, gtk_window_get_type()) {
        (*state).window as *mut GtkWindow
    } else {
        ptr::null_mut()
    };

    if (*state).closing != 0 || window.is_null() || (*state).restore_maximized_after_fullscreen == 0
    {
        (*state).maximize_restore_source = 0;
        return G_SOURCE_REMOVE;
    }

    if gtk_window_is_fullscreen(window) != 0 {
        return G_SOURCE_CONTINUE;
    }

    if (*state).restore_window_width > 0 && (*state).restore_window_height > 0 {
        gtk_window_set_default_size(
            window,
            (*state).restore_window_width,
            (*state).restore_window_height,
        );
    }

    gtk_window_maximize(window);
    if gtk_window_is_maximized(window) != 0 {
        (*state).window_maximized = TRUE;
        (*state).restore_maximized_after_fullscreen = FALSE;
        (*state).maximize_restore_source = 0;
        return G_SOURCE_REMOVE;
    }

    (*state).maximize_restore_attempts += 1;
    if (*state).maximize_restore_attempts >= MAXIMIZE_RESTORE_ATTEMPTS {
        (*state).restore_maximized_after_fullscreen = FALSE;
        (*state).maximize_restore_source = 0;
        return G_SOURCE_REMOVE;
    }

    G_SOURCE_CONTINUE
}

unsafe fn schedule_maximized_restore_after_fullscreen(state: *mut AppState) {
    remove_source_if_active(&mut (*state).maximize_restore_source);

    if (*state).restore_maximized_after_fullscreen == 0 {
        return;
    }

    (*state).maximize_restore_attempts = 0;
    (*state).maximize_restore_source = g_timeout_add(
        50,
        Some(restore_maximized_after_fullscreen),
        state as *mut c_void,
    );
}

unsafe fn set_fullscreen(state: *mut AppState, fullscreen: c_int) {
    if (*state).fullscreen == fullscreen {
        return;
    }

    (*state).fullscreen = fullscreen;
    if fullscreen != 0 {
        (*state).was_maximized_before_fullscreen = ((*state).window_maximized != 0
            || gtk_window_is_maximized((*state).window as *mut GtkWindow) != 0)
            as c_int;
        (*state).restore_window_width = gtk_widget_get_width((*state).window);
        (*state).restore_window_height = gtk_widget_get_height((*state).window);
        (*state).restore_maximized_after_fullscreen = FALSE;
        remove_source_if_active(&mut (*state).maximize_restore_source);
        gtk_window_fullscreen((*state).window as *mut GtkWindow);
    } else {
        (*state).restore_maximized_after_fullscreen = (*state).was_maximized_before_fullscreen;
        gtk_window_unfullscreen((*state).window as *mut GtkWindow);
        schedule_maximized_restore_after_fullscreen(state);
        (*state).was_maximized_before_fullscreen = FALSE;
    }

    if !(*state).single_player.is_null() {
        single_player_set_fullscreen((*state).single_player, fullscreen);
    }
    if !(*state).grid_player.is_null() {
        grid_player_set_fullscreen((*state).grid_player, fullscreen);
    }

    show_window_overlay(state);
}

unsafe extern "C" fn on_window_fullscreen_changed(
    _object: *mut GObject,
    _pspec: *mut GParamSpec,
    user_data: *mut c_void,
) {
    schedule_maximized_restore_after_fullscreen(user_data as *mut AppState);
}

unsafe extern "C" fn on_window_maximized_changed(
    _object: *mut GObject,
    _pspec: *mut GParamSpec,
    user_data: *mut c_void,
) {
    let state = user_data as *mut AppState;

    if (*state).fullscreen != 0
        || (*state).window.is_null()
        || gtk_window_is_fullscreen((*state).window as *mut GtkWindow) != 0
    {
        return;
    }

    (*state).window_maximized = gtk_window_is_maximized((*state).window as *mut GtkWindow);
}

unsafe fn toggle_fullscreen(state: *mut AppState) {
    set_fullscreen(state, ((*state).fullscreen == 0) as c_int);
}

unsafe extern "C" fn on_content_fullscreen_requested(user_data: *mut c_void) {
    toggle_fullscreen(user_data as *mut AppState);
}

unsafe extern "C" fn on_content_settings_requested(user_data: *mut c_void) {
    show_settings_window(user_data as *mut AppState, SETTINGS_WINDOW_PAGE_CHANNELS);
}

unsafe fn destroy_active_content(state: *mut AppState) {
    if !(*state).single_player.is_null() {
        single_player_free((*state).single_player);
        (*state).single_player = ptr::null_mut();
    }
    if !(*state).grid_player.is_null() {
        grid_player_free((*state).grid_player);
        (*state).grid_player = ptr::null_mut();
    }

    if is_instance((*state).root_overlay, gtk_overlay_get_type()) {
        gtk_overlay_set_child((*state).root_overlay as *mut GtkOverlay, ptr::null_mut());
    }
}

unsafe fn clear_single_target(state: *mut AppState) {
    g_free((*state).single_target as *mut c_void);
    (*state).single_target = ptr::null_mut();
}

unsafe fn capture_single_handoff(state: *mut AppState) {
    if (*state).single_player.is_null() {
        return;
    }

    clear_single_target(state);
    (*state).single_target = single_player_dup_current_target((*state).single_player);
    (*state).has_single_target_handoff = TRUE;
    (*state).single_chat_paned_position =
        single_player_get_chat_paned_position((*state).single_player);
}

unsafe fn capture_grid_handoff(state: *mut AppState) {
    if (*state).grid_player.is_null() {
        return;
    }

    let handoff_session = grid_player_take_first_session((*state).grid_player);
    if !handoff_session.is_null() && handoff_session != (*state).primary_session {
        player_session_free((*state).primary_session);
        (*state).primary_session = handoff_session;
    }

    clear_single_target(state);
    if player_session_is_playing((*state).primary_session) != 0 {
        let channel = player_session_get_channel((*state).primary_session);
        (*state).single_target = if is_nonempty(channel) {
            g_strdup(channel)
        } else {
            ptr::null_mut()
        };
    } else {
        (*state).single_target = grid_player_dup_first_target((*state).grid_player);
    }
    (*state).has_single_target_handoff = TRUE;
}

unsafe fn create_single_content(state: *mut AppState) {
    let target = if (*state).has_single_target_handoff != 0 {
        (*state).single_target as *const c_char
    } else {
        (*state).startup_target
    };

    (*state).single_player = single_player_new(
        (*state).window as *mut GtkWindow,
        (*state).settings,
        (*state).primary_session,
        target,
        is_nonempty(target) as c_int,
        (*state).single_chat_paned_position,
        Some(on_content_fullscreen_requested),
        state as *mut c_void,
        Some(on_content_settings_requested),
        state as *mut c_void,
    );
    single_player_set_fullscreen((*state).single_player, (*state).fullscreen);
    gtk_overlay_set_child(
        (*state).root_overlay as *mut GtkOverlay,
        single_player_get_widget((*state).single_player),
    );
    (*state).content_mode = CONTENT_MODE_SINGLE;
    gtk_widget_set_tooltip_text((*state).layout_button, cstr!("Switch to grid player"));
    gtk_button_set_child(
        (*state).layout_button as *mut GtkButton,
        player_layout_icon_new(PLAYER_LAYOUT_ICON_GRID),
    );
}

unsafe fn create_grid_content(state: *mut AppState) {
    let mut targets: [*const c_char; GRID_PLAYER_MAX_TILES] = [ptr::null(); GRID_PLAYER_MAX_TILES];
    let mut target_storage: [*mut c_char; GRID_PLAYER_MAX_TILES] =
        [ptr::null_mut(); GRID_PLAYER_MAX_TILES];
    let mut target_count: usize = 0;

    if is_nonempty((*state).single_target) {
        target_storage[target_count] = dup_twitch_channel_name((*state).single_target);
        if !target_storage[target_count].is_null() {
            targets[target_count] = target_storage[target_count];
            target_count += 1;
        }
    }

    let grid_count = (*state).grid_target_count as usize;
    for i in 0..grid_count {
        if target_count >= GRID_PLAYER_MAX_TILES {
            break;
        }
        let target = *(*state).grid_targets.add(i);
        if !is_nonempty(target) {
            continue;
        }
        if !(*state).single_target.is_null() && g_strcmp0(target, (*state).single_target) == 0 {
            continue;
        }

        let channel = dup_twitch_channel_name(target);
        if channel.is_null() {
            continue;
        }

        let mut duplicate = false;
        for existing in targets.iter().take(target_count) {
            if g_ascii_strcasecmp(channel, *existing) == 0 {
                duplicate = true;
                break;
            }
        }
        if duplicate {
            g_free(channel as *mut c_void);
            continue;
        }

        target_storage[target_count] = channel;
        targets[target_count] = target_storage[target_count];
        target_count += 1;
    }

    (*state).grid_player = grid_player_new(
        (*state).window as *mut GtkWindow,
        (*state).settings,
        (*state).primary_session,
        targets.as_ptr(),
        target_count as c_uint,
        Some(on_content_fullscreen_requested),
        state as *mut c_void,
        Some(on_content_settings_requested),
        state as *mut c_void,
    );
    grid_player_set_fullscreen((*state).grid_player, (*state).fullscreen);
    gtk_overlay_set_child(
        (*state).root_overlay as *mut GtkOverlay,
        grid_player_get_widget((*state).grid_player),
    );
    grid_player_start((*state).grid_player);
    (*state).content_mode = CONTENT_MODE_GRID;
    gtk_widget_set_tooltip_text((*state).layout_button, cstr!("Switch to single player"));
    gtk_button_set_child(
        (*state).layout_button as *mut GtkButton,
        player_layout_icon_new(PLAYER_LAYOUT_ICON_SINGLE),
    );

    for target in target_storage {
        g_free(target as *mut c_void);
    }
}

unsafe fn set_layout_mode(state: *mut AppState, mode: c_int) {
    if (*state).single_player.is_null() && (*state).grid_player.is_null() {
        if mode == CONTENT_MODE_GRID {
            create_grid_content(state);
        } else {
            create_single_content(state);
        }
        show_window_overlay(state);
        return;
    }

    if (*state).content_mode == mode {
        show_window_overlay(state);
        return;
    }

    if mode == CONTENT_MODE_GRID {
        capture_single_handoff(state);
    } else {
        capture_grid_handoff(state);
    }

    destroy_active_content(state);
    if mode == CONTENT_MODE_GRID {
        create_grid_content(state);
    } else {
        create_single_content(state);
    }
    show_window_overlay(state);
}

unsafe extern "C" fn on_layout_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut AppState;
    let next_mode = if (*state).content_mode == CONTENT_MODE_SINGLE {
        CONTENT_MODE_GRID
    } else {
        CONTENT_MODE_SINGLE
    };
    set_layout_mode(state, next_mode);
}

unsafe extern "C" fn on_settings_saved(_settings: *mut AppSettings, user_data: *mut c_void) {
    let state = user_data as *mut AppState;

    if !(*state).single_player.is_null() {
        single_player_set_settings((*state).single_player, (*state).settings);
    }
    if !(*state).grid_player.is_null() {
        grid_player_set_settings((*state).grid_player, (*state).settings);
    }

    show_window_overlay(state);
}

unsafe fn show_settings_window(state: *mut AppState, initial_page: c_int) {
    settings_window_show(
        (*state).window as *mut GtkWindow,
        (*state).settings,
        initial_page,
        Some(on_settings_saved),
        state as *mut c_void,
    );
    show_window_overlay(state);
}

unsafe extern "C" fn on_settings_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    show_settings_window(user_data as *mut AppState, SETTINGS_WINDOW_PAGE_GENERAL);
}

unsafe extern "C" fn on_minimize_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut AppState;
    gtk_window_minimize((*state).window as *mut GtkWindow);
}

unsafe extern "C" fn on_fullscreen_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    toggle_fullscreen(user_data as *mut AppState);
}

unsafe extern "C" fn on_close_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let state = user_data as *mut AppState;
    gtk_window_close((*state).window as *mut GtkWindow);
}

unsafe extern "C" fn on_root_motion(
    _controller: *mut GtkEventControllerMotion,
    x: c_double,
    y: c_double,
    user_data: *mut c_void,
) {
    let state = user_data as *mut AppState;

    if player_motion_tracker_ignore_stationary(
        &mut (*state).motion_tracker,
        state as *mut c_void,
        x,
        y,
    ) != 0
    {
        return;
    }

    show_window_overlay(state);
}

unsafe extern "C" fn on_key_pressed(
    _controller: *mut GtkEventControllerKey,
    keyval: c_uint,
    _keycode: c_uint,
    modifiers: c_uint,
    user_data: *mut c_void,
) -> c_int {
    let state = user_data as *mut AppState;

    if !(*state).single_player.is_null() {
        return single_player_handle_key((*state).single_player, keyval, modifiers);
    }

    GDK_EVENT_PROPAGATE
}

unsafe fn install_css() {
    player_style_install_overlay_css();

    let provider = gtk_css_provider_new();
    let css = CString::new(concat!(
        ".video-footer {",
        "  background: rgba(0, 0, 0, 0.58);",
        "  padding: 4px 6px;",
        "  border-radius: 0;",
        "}",
        ".main-area {",
        "  background: #0e0e10;",
        "}",
        "paned.main-area > separator,",
        "paned.main-area > separator.wide,",
        ".main-area separator,",
        ".main-area separator.wide,",
        ".main-area > separator,",
        ".main-area > separator.wide {",
        "  background: transparent;",
        "  background-image: none;",
        "  border: none;",
        "  outline: none;",
        "  box-shadow: none;",
        "  color: transparent;",
        "  margin: 0;",
        "  padding: 0;",
        "  min-width: 1px;",
        "}",
        "paned.main-area > separator:hover,",
        "paned.main-area > separator.wide:hover,",
        ".main-area separator:hover,",
        ".main-area separator.wide:hover {",
        "  background: transparent;",
        "  background-image: none;",
        "  border: none;",
        "  outline: none;",
        "  box-shadow: none;",
        "}",
        ".chat-panel,",
        ".chat-scroll,",
        ".chat-scroll viewport,",
        ".chat-view,",
        ".chat-view text {",
        "  background: #0e0e10;",
        "  color: #efeff1;",
        "}",
        ".chat-view {",
        "  caret-color: transparent;",
        "  font-size: 14px;",
        "}",
        ".chat-emote {",
        "  background: transparent;",
        "}",
        ".chat-view text selection {",
        "  background: rgba(145, 70, 255, 0.35);",
        "  color: #ffffff;",
        "}",
        ".chat-scroll scrollbar {",
        "  background: transparent;",
        "}",
        ".chat-scroll scrollbar slider {",
        "  background: rgba(255, 255, 255, 0.28);",
        "  border-radius: 999px;",
        "  min-width: 4px;",
        "}",
        ".settings-overlay-button {",
        "  background: rgba(0, 0, 0, 0.30);",
        "}",
        ".settings-overlay-button:hover {",
        "  background: rgba(38, 38, 38, 0.62);",
        "}",
        ".video-footer button,",
        ".video-footer menubutton,",
        ".video-footer menubutton > button,",
        ".video-footer popover,",
        ".video-footer scale {",
        "  color: white;",
        "}",
        ".video-footer button,",
        ".video-footer menubutton > button {",
        "  background: rgba(30, 30, 30, 0.82);",
        "  color: white;",
        "  border-color: transparent;",
        "  outline-color: transparent;",
        "  box-shadow: none;",
        "  min-height: 0;",
        "}",
        ".video-footer button:hover,",
        ".video-footer menubutton > button:hover {",
        "  background: rgba(54, 54, 54, 0.90);",
        "}",
        ".stream-dropdown {",
        "  min-width: 140px;",
        "  min-height: 24px;",
        "}",
        ".stream-selector {",
        "  min-width: 140px;",
        "}",
        ".stream-dropdown,",
        ".stream-dropdown > button {",
        "  padding: 2px 8px;",
        "  min-height: 24px;",
        "}",
        ".stream-button-label {",
        "  color: white;",
        "  font-size: 13px;",
        "}",
        ".stream-info-labels {",
        "  margin-left: 2px;",
        "  margin-right: 4px;",
        "}",
        ".stream-title-label {",
        "  color: rgba(255, 255, 255, 0.92);",
        "  font-size: 12px;",
        "}",
        ".stream-metadata-label {",
        "  color: rgba(255, 255, 255, 0.78);",
        "  font-size: 11px;",
        "}",
        ".stream-popover contents {",
        "  background: rgba(28, 28, 28, 0.98);",
        "  padding: 0;",
        "  margin: 0;",
        "  border: none;",
        "  border-radius: 4px;",
        "  box-shadow: none;",
        "}",
        ".stream-popover {",
        "  padding: 0;",
        "  margin: 0;",
        "  border: none;",
        "  border-radius: 4px;",
        "  box-shadow: none;",
        "}",
        ".stream-menu {",
        "  background: rgba(28, 28, 28, 0.98);",
        "  padding: 2px 0;",
        "  margin: 0;",
        "}",
        ".stream-menu-item {",
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
        ".stream-menu-item box {",
        "  padding: 0;",
        "  margin: 0;",
        "}",
        ".stream-menu-item label {",
        "  color: white;",
        "  padding: 0;",
        "  margin: 0;",
        "}",
        ".stream-menu-item:hover {",
        "  background: rgba(74, 74, 74, 0.98);",
        "  color: white;",
        "}",
        ".settings-window {",
        "  background: #141417;",
        "  color: #efeff1;",
        "}",
        ".settings-sidebar {",
        "  background: #1f1f23;",
        "  border-right: 1px solid rgba(255, 255, 255, 0.10);",
        "  padding: 8px;",
        "}",
        ".settings-sidebar row {",
        "  border-radius: 6px;",
        "  padding: 10px 12px;",
        "}",
        ".settings-sidebar row:selected {",
        "  background: #2f2f35;",
        "}",
        ".settings-sidebar-label,",
        ".settings-page-title,",
        ".settings-section-title,",
        ".settings-channel-header label,",
        ".settings-empty-label,",
        ".settings-hint-label,",
        ".settings-status-label {",
        "  color: #efeff1;",
        "}",
        ".settings-page {",
        "  padding: 18px;",
        "  background: #141417;",
        "}",
        ".settings-page-title {",
        "  font-size: 20px;",
        "  font-weight: 700;",
        "}",
        ".settings-section-title {",
        "  color: rgba(239, 239, 241, 0.88);",
        "  font-size: 13px;",
        "  font-weight: 700;",
        "}",
        ".settings-section-header {",
        "  margin-top: 2px;",
        "}",
        ".settings-section-rule {",
        "  background: rgba(239, 239, 241, 0.16);",
        "  min-height: 1px;",
        "}",
        ".settings-channel-header label {",
        "  color: rgba(239, 239, 241, 0.70);",
        "  font-size: 12px;",
        "  font-weight: 700;",
        "}",
        ".settings-check {",
        "  color: #efeff1;",
        "  margin-top: 8px;",
        "  margin-bottom: 0;",
        "}",
        ".settings-hint-label {",
        "  color: rgba(239, 239, 241, 0.64);",
        "  font-size: 13px;",
        "  margin-top: -4px;",
        "  margin-left: 0;",
        "}",
        ".settings-hint-label link {",
        "  color: #8ab4ff;",
        "}",
        ".settings-hint-label link:hover {",
        "  color: #adc8ff;",
        "}",
        ".settings-channel-row entry {",
        "  background: #222226;",
        "  color: #ffffff;",
        "  border-color: rgba(255, 255, 255, 0.12);",
        "  font-size: 13px;",
        "  min-height: 28px;",
        "  padding: 3px 8px;",
        "}",
        ".settings-channel-row entry selection {",
        "  background: rgba(145, 70, 255, 0.55);",
        "  color: #ffffff;",
        "}",
        ".settings-page button,",
        ".settings-footer button {",
        "  background: #26262c;",
        "  color: #efeff1;",
        "  border-color: rgba(255, 255, 255, 0.12);",
        "  outline-color: transparent;",
        "  box-shadow: none;",
        "}",
        ".settings-page button:hover,",
        ".settings-footer button:hover {",
        "  background: #34343b;",
        "}",
        ".settings-page .settings-primary-button,",
        ".settings-footer .settings-primary-button {",
        "  background: #3a2b52;",
        "  color: #ffffff;",
        "}",
        ".settings-page .settings-primary-button:hover,",
        ".settings-footer .settings-primary-button:hover {",
        "  background: #4b3670;",
        "}",
        ".settings-page .settings-remove-button {",
        "  background: transparent;",
        "  border-color: transparent;",
        "  min-width: 24px;",
        "  min-height: 24px;",
        "  padding: 2px 4px;",
        "}",
        ".settings-page .settings-remove-button:hover {",
        "  background: rgba(255, 255, 255, 0.08);",
        "}",
        ".settings-footer {",
        "  background: #141417;",
        "  padding: 0 18px 18px 18px;",
        "}",
        ".settings-empty-label {",
        "  color: rgba(239, 239, 241, 0.62);",
        "  margin-top: 8px;",
        "  margin-bottom: 8px;",
        "}",
        ".settings-status-label {",
        "  color: #ffb4ab;",
        "}",
    ))
    .expect("static CSS has no NUL bytes");

    gtk_css_provider_load_from_string(provider, css.as_ptr());
    gtk_style_context_add_provider_for_display(
        gdk_display_get_default(),
        provider as *mut GtkStyleProvider,
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    g_object_unref(provider as *mut c_void);
    player_style_install_footer_css();
}

unsafe fn install_app_icon() {
    let theme = gtk_icon_theme_get_for_display(gdk_display_get_default());
    let icons_dir = g_build_filename(g_get_user_data_dir(), cstr!("icons"), ptr::null::<c_char>());
    gtk_icon_theme_add_search_path(theme, icons_dir);
    g_free(icons_dir as *mut c_void);
    gtk_window_set_default_icon_name(c_app_id());
}

unsafe fn get_executable_path(argv0: *const c_char) -> *mut c_char {
    let link_path = g_file_read_link(cstr!("/proc/self/exe"), ptr::null_mut());
    if !link_path.is_null() {
        return link_path;
    }

    if g_path_is_absolute(argv0) != 0 {
        return g_strdup(argv0);
    }

    let cwd = g_get_current_dir();
    let result = g_canonicalize_filename(argv0, cwd);
    g_free(cwd as *mut c_void);
    result
}

unsafe fn quote_desktop_exec_path(path: *const c_char) -> *mut c_char {
    let bytes = if path.is_null() {
        &[][..]
    } else {
        CStr::from_ptr(path).to_bytes()
    };
    let mut quoted = Vec::with_capacity(bytes.len() + 2);
    quoted.push(b'"');
    for byte in bytes {
        if matches!(*byte, b'"' | b'\\' | b'`' | b'$') {
            quoted.push(b'\\');
        }
        quoted.push(*byte);
    }
    quoted.push(b'"');
    dup_bytes(&quoted)
}

unsafe fn error_message(error: *mut GError) -> *const c_char {
    if error.is_null() || (*error).message.is_null() {
        cstr!("unknown error")
    } else {
        (*error).message
    }
}

unsafe fn log_debug_static(message: *const c_char) {
    g_log(
        G_LOG_DOMAIN.as_ptr() as *const c_char,
        G_LOG_LEVEL_DEBUG,
        cstr!("%s"),
        message,
    );
}

unsafe fn log_debug_error(format: *const c_char, message: *const c_char) {
    g_log(
        G_LOG_DOMAIN.as_ptr() as *const c_char,
        G_LOG_LEVEL_DEBUG,
        format,
        message,
    );
}

unsafe fn write_user_desktop_identity(argv0: *const c_char) {
    let applications_dir = g_build_filename(
        g_get_user_data_dir(),
        cstr!("applications"),
        ptr::null::<c_char>(),
    );
    let icons_dir = g_build_filename(
        g_get_user_data_dir(),
        cstr!("icons"),
        cstr!("hicolor"),
        cstr!("scalable"),
        cstr!("apps"),
        ptr::null::<c_char>(),
    );
    let desktop_path = g_build_filename(
        applications_dir,
        cstr!("local.twitchplayer.desktop"),
        ptr::null::<c_char>(),
    );
    let icon_path = g_build_filename(
        icons_dir,
        cstr!("local.twitchplayer.svg"),
        ptr::null::<c_char>(),
    );
    let exec_path = get_executable_path(argv0);
    let quoted_exec = quote_desktop_exec_path(exec_path);
    let mut error: *mut GError = ptr::null_mut();

    if g_mkdir_with_parents(applications_dir, 0o700) < 0
        || g_mkdir_with_parents(icons_dir, 0o700) < 0
    {
        log_debug_static(cstr!("could not create user desktop/icon directories"));
        g_free(applications_dir as *mut c_void);
        g_free(icons_dir as *mut c_void);
        g_free(desktop_path as *mut c_void);
        g_free(icon_path as *mut c_void);
        g_free(exec_path as *mut c_void);
        g_free(quoted_exec as *mut c_void);
        return;
    }

    if g_file_set_contents(
        icon_path,
        APP_ICON_SVG.as_ptr() as *const c_char,
        APP_ICON_SVG.len() as isize,
        &mut error,
    ) == 0
    {
        log_debug_error(
            cstr!("could not write user app icon: %s"),
            error_message(error),
        );
        g_clear_error(&mut error);
        g_free(applications_dir as *mut c_void);
        g_free(icons_dir as *mut c_void);
        g_free(desktop_path as *mut c_void);
        g_free(icon_path as *mut c_void);
        g_free(exec_path as *mut c_void);
        g_free(quoted_exec as *mut c_void);
        return;
    }

    let desktop = g_strdup_printf(
        cstr!(
            "[Desktop Entry]\nType=Application\nName=Twitch Player\nExec=%s %%u\nIcon=%s\nTerminal=false\nCategories=AudioVideo;Player;Network;\nStartupNotify=true\nStartupWMClass=%s\n"
        ),
        quoted_exec,
        icon_path,
        c_app_id(),
    );

    if g_file_set_contents(desktop_path, desktop, -1, &mut error) == 0 {
        log_debug_error(
            cstr!("could not write desktop entry: %s"),
            error_message(error),
        );
        g_clear_error(&mut error);
    }

    g_free(desktop as *mut c_void);
    g_free(applications_dir as *mut c_void);
    g_free(icons_dir as *mut c_void);
    g_free(desktop_path as *mut c_void);
    g_free(icon_path as *mut c_void);
    g_free(exec_path as *mut c_void);
    g_free(quoted_exec as *mut c_void);
}

unsafe extern "C" fn destroy_state(user_data: *mut c_void) {
    let state = user_data as *mut AppState;
    (*state).closing = TRUE;

    remove_source_if_active(&mut (*state).overlay_hide_source);
    remove_source_if_active(&mut (*state).maximize_restore_source);

    destroy_active_content(state);
    clear_single_target(state);
    if !(*state).primary_session.is_null() {
        player_session_free((*state).primary_session);
        (*state).primary_session = ptr::null_mut();
    }
    app_settings_free((*state).settings);
    (*state).settings = ptr::null_mut();
}

unsafe fn add_weak_pointer<T>(object: *mut T, slot: *mut *mut T) {
    g_object_add_weak_pointer(object as *mut GObject, slot as *mut *mut c_void);
}

unsafe extern "C" fn on_activate(application: *mut GtkApplication, user_data: *mut c_void) {
    let config = user_data as *mut StartupConfig;

    install_css();
    install_app_icon();

    let state = Box::into_raw(Box::new(AppState {
        window: ptr::null_mut(),
        root_overlay: ptr::null_mut(),
        top_left_controls: ptr::null_mut(),
        top_controls: ptr::null_mut(),
        settings_button: ptr::null_mut(),
        layout_button: ptr::null_mut(),
        settings: app_settings_load(),
        primary_session: player_session_new(),
        single_player: ptr::null_mut(),
        grid_player: ptr::null_mut(),
        startup_target: if config.is_null() {
            ptr::null()
        } else {
            (*config).startup_target
        },
        grid_targets: if config.is_null() {
            ptr::null()
        } else {
            (*config).grid_targets
        },
        grid_target_count: if config.is_null() {
            0
        } else {
            (*config).grid_target_count
        },
        single_target: ptr::null_mut(),
        has_single_target_handoff: FALSE,
        single_chat_paned_position: 0,
        content_mode: CONTENT_MODE_SINGLE,
        overlay_hide_source: 0,
        maximize_restore_source: 0,
        maximize_restore_attempts: 0,
        motion_tracker: PlayerMotionTracker::new(),
        closing: FALSE,
        fullscreen: FALSE,
        window_maximized: FALSE,
        was_maximized_before_fullscreen: FALSE,
        restore_maximized_after_fullscreen: FALSE,
        restore_window_width: 0,
        restore_window_height: 0,
    }));
    player_session_set_hwdec_enabled(
        (*state).primary_session,
        app_settings_get_hwdec_enabled((*state).settings),
    );

    (*state).window = gtk_application_window_new(application);
    gtk_window_set_title((*state).window as *mut GtkWindow, cstr!("Twitch Player"));
    gtk_window_set_default_size(
        (*state).window as *mut GtkWindow,
        DEFAULT_WINDOW_WIDTH,
        DEFAULT_WINDOW_HEIGHT,
    );
    gtk_window_set_decorated((*state).window as *mut GtkWindow, FALSE);
    gtk_window_set_icon_name((*state).window as *mut GtkWindow, c_app_id());
    g_signal_connect_data(
        (*state).window as *mut c_void,
        cstr!("notify::fullscreened"),
        on_window_fullscreen_changed as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        (*state).window as *mut c_void,
        cstr!("notify::maximized"),
        on_window_maximized_changed as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );

    (*state).root_overlay = gtk_overlay_new();
    add_weak_pointer((*state).root_overlay, &mut (*state).root_overlay);
    gtk_window_set_child((*state).window as *mut GtkWindow, (*state).root_overlay);
    add_resize_handles((*state).root_overlay as *mut GtkOverlay, state);

    (*state).top_left_controls = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    add_weak_pointer((*state).top_left_controls, &mut (*state).top_left_controls);
    gtk_widget_add_css_class((*state).top_left_controls, cstr!("top-overlay-controls"));
    gtk_widget_set_halign((*state).top_left_controls, GTK_ALIGN_START);
    gtk_widget_set_valign((*state).top_left_controls, GTK_ALIGN_START);
    gtk_overlay_add_overlay(
        (*state).root_overlay as *mut GtkOverlay,
        (*state).top_left_controls,
    );

    (*state).settings_button =
        player_overlay_button_new(player_settings_icon_new(), cstr!("Settings"));
    gtk_widget_add_css_class((*state).settings_button, cstr!("settings-overlay-button"));
    gtk_box_append(
        (*state).top_left_controls as *mut GtkBox,
        (*state).settings_button,
    );
    g_signal_connect_data(
        (*state).settings_button as *mut c_void,
        cstr!("clicked"),
        on_settings_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );

    (*state).layout_button = player_overlay_button_new(
        player_layout_icon_new(PLAYER_LAYOUT_ICON_GRID),
        cstr!("Switch to grid player"),
    );
    gtk_widget_add_css_class((*state).layout_button, cstr!("settings-overlay-button"));
    gtk_box_append(
        (*state).top_left_controls as *mut GtkBox,
        (*state).layout_button,
    );
    g_signal_connect_data(
        (*state).layout_button as *mut c_void,
        cstr!("clicked"),
        on_layout_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );

    (*state).top_controls = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    add_weak_pointer((*state).top_controls, &mut (*state).top_controls);
    gtk_widget_add_css_class((*state).top_controls, cstr!("top-overlay-controls"));
    gtk_widget_set_halign((*state).top_controls, GTK_ALIGN_END);
    gtk_widget_set_valign((*state).top_controls, GTK_ALIGN_START);
    gtk_overlay_add_overlay(
        (*state).root_overlay as *mut GtkOverlay,
        (*state).top_controls,
    );

    let minimize_button = player_overlay_button_new(
        player_window_icon_new(PLAYER_WINDOW_ICON_MINIMIZE),
        cstr!("Minimize"),
    );
    gtk_box_append((*state).top_controls as *mut GtkBox, minimize_button);
    g_signal_connect_data(
        minimize_button as *mut c_void,
        cstr!("clicked"),
        on_minimize_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );

    let fullscreen_button = player_overlay_button_new(
        player_window_icon_new(PLAYER_WINDOW_ICON_FULLSCREEN),
        cstr!("Fullscreen"),
    );
    gtk_box_append((*state).top_controls as *mut GtkBox, fullscreen_button);
    g_signal_connect_data(
        fullscreen_button as *mut c_void,
        cstr!("clicked"),
        on_fullscreen_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );

    let close_button = player_overlay_button_new(
        player_window_icon_new(PLAYER_WINDOW_ICON_CLOSE),
        cstr!("Close"),
    );
    gtk_widget_add_css_class(close_button, cstr!("close-button"));
    gtk_box_append((*state).top_controls as *mut GtkBox, close_button);
    g_signal_connect_data(
        close_button as *mut c_void,
        cstr!("clicked"),
        on_close_clicked as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );

    let motion = gtk_event_controller_motion_new();
    gtk_event_controller_set_propagation_phase(motion, GTK_PHASE_CAPTURE);
    g_signal_connect_data(
        motion as *mut c_void,
        cstr!("motion"),
        on_root_motion as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*state).root_overlay, motion as *mut c_void);

    let key_controller = gtk_event_controller_key_new();
    gtk_event_controller_set_propagation_phase(key_controller, GTK_PHASE_CAPTURE);
    g_signal_connect_data(
        key_controller as *mut c_void,
        cstr!("key-pressed"),
        on_key_pressed as *const c_void,
        state as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*state).window, key_controller as *mut c_void);

    g_object_set_data_full(
        (*state).window as *mut GObject,
        cstr!("app-state"),
        state as *mut c_void,
        Some(destroy_state),
    );

    let start_grid = !config.is_null() && (*config).start_in_grid != 0;
    set_layout_mode(
        state,
        if start_grid {
            CONTENT_MODE_GRID
        } else {
            CONTENT_MODE_SINGLE
        },
    );
    gtk_window_present((*state).window as *mut GtkWindow);
    schedule_window_overlay_hide(state);
}

pub fn run_from_env() -> i32 {
    let mut args: Vec<CString> = env::args_os()
        .map(|arg| {
            CString::new(arg.into_vec()).expect("process arguments cannot contain NUL bytes")
        })
        .collect();
    if args.is_empty() {
        args.push(CString::new("twitch-player").expect("static string cannot contain NUL bytes"));
    }

    unsafe { run_with_args(&args) }
}

unsafe fn run_with_args(args: &[CString]) -> c_int {
    let mut grid_targets: [*const c_char; GRID_PLAYER_MAX_TILES] =
        [ptr::null(); GRID_PLAYER_MAX_TILES];
    let mut grid_target_count: c_uint = 0;
    let mut start_in_grid = FALSE;
    let mut startup_target: *const c_char = ptr::null();
    let mut argv: Vec<*mut c_char> = args.iter().map(|arg| arg.as_ptr() as *mut c_char).collect();
    argv.push(ptr::null_mut());

    for arg in args.iter().skip(1) {
        let arg = arg.as_ptr();
        if CStr::from_ptr(arg).to_bytes() == b"--grid" {
            start_in_grid = TRUE;
            continue;
        }

        if startup_target.is_null() {
            startup_target = arg;
        }
        if (grid_target_count as usize) < GRID_PLAYER_MAX_TILES {
            grid_targets[grid_target_count as usize] = arg;
            grid_target_count += 1;
        }
    }

    let mut config = StartupConfig {
        startup_target,
        grid_targets: grid_targets.as_ptr(),
        grid_target_count,
        start_in_grid,
    };

    configure_rendering_defaults();
    g_set_prgname(c_app_id());
    g_set_application_name(cstr!("Twitch Player"));
    write_user_desktop_identity(argv[0]);

    let application = gtk_application_new(c_app_id(), G_APPLICATION_NON_UNIQUE);
    g_signal_connect_data(
        application as *mut c_void,
        cstr!("activate"),
        on_activate as *const c_void,
        &mut config as *mut StartupConfig as *mut c_void,
        ptr::null_mut(),
        0,
    );

    let status = g_application_run(application as *mut c_void, 1, argv.as_mut_ptr());
    g_object_unref(application as *mut c_void);
    status
}
