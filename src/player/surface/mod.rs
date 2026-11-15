mod streams;

mod chat;
mod controls;
mod ffi;
mod input;
mod layout;
mod playback;
mod render;
mod runtime;
mod stream_details;
mod style;
#[cfg(test)]
mod tests;

pub use self::ffi::{GtkWidget, GtkWindow};
pub use self::input::player_surface_handle_key;
pub use self::layout::player_surface_set_fullscreen;
pub use self::layout::{
    player_surface_apply_2x2_template, player_surface_apply_layout,
    player_surface_apply_single_template, player_surface_get_layout_index,
    player_surface_is_single_template,
};

use self::chat::capture_chat_width_from_paned;
use self::controls::create_stream_tile;
use self::controls::{schedule_footer_hide, show_tile_overlay};
use self::ffi::{
    GtkGrid, GtkOverlay, GtkPaned, g_strdup, gtk_grid_attach, gtk_grid_new,
    gtk_grid_set_column_homogeneous, gtk_grid_set_row_homogeneous, gtk_overlay_new,
    gtk_overlay_set_child, gtk_paned_set_end_child, gtk_widget_add_css_class,
    gtk_widget_set_hexpand, gtk_widget_set_vexpand, gtk_widget_unparent,
};
use self::layout::{
    get_target_at, get_target_count, preferred_single_tile, restore_active_layout_with_primary_slot,
};
use self::playback::{
    ensure_tile_session, load_tile_stream, set_tile_status, sync_tile_from_session,
    update_tile_empty_state,
};
use self::render::{PlayerRenderContext, clear_tile_render_context};
use self::runtime::{
    add_surface_seconds_timeout, add_weak_pointer, clear_pointer, disable_tile_wakeup_callback,
    remove_source_if_active, remove_weak_pointer,
};
use self::streams::clear_tile_stream_qualities;
use self::streams::{
    refresh_surface_stream_titles, request_tile_title_update, reset_tile_stream_title,
};
use self::style::install_css;
use crate::chat::panel::{ChatPanel, chat_panel_free};
use crate::glib_strings::is_nonempty;
use crate::player::channel_switcher_overlay::{
    ChannelSwitcherOverlay, SharedPickerCache, channel_switcher_overlay_free,
    channel_switcher_overlay_set_settings,
};
use crate::player::controls::{PlayerFooterStreamInfo, player_footer_stream_info_free};
use crate::player::layout::{
    PLAYER_LAYOUT_2X2_INDEX, PLAYER_LAYOUT_MAX_TILES, PLAYER_LAYOUTS, PlayerLayoutCell,
};
use crate::player::motion::PlayerMotionTracker;
use crate::player::session::{
    PlayerSession, player_session_free, player_session_is_playing, player_session_set_hwdec_enabled,
};
use crate::player::stream_quality::PlayerStreamQualityState;
use crate::settings::{AppSettings, app_settings_get_hwdec_enabled};
use crate::twitch::stream_info::GCancellable;
use std::ffi::{c_char, c_double, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
const MAX_TILES: usize = PLAYER_LAYOUT_MAX_TILES;
const STREAM_TITLE_REFRESH_SECONDS: c_uint = 3 * 60;
const DEFAULT_CHAT_WIDTH: c_int = 220;
const FALSE: c_int = 0;
const TRUE: c_int = 1;
const G_LOG_DOMAIN: &[u8] = b"twitch-player-surface\0";
const PLAYER_TILE_FOCUS_ICON_EXPAND: c_int = 0;
const PLAYER_CHAT_ICON_OPEN: c_int = 0;
const PLAYER_STARTING_STREAM_STATUS: Option<&str> = Some("Starting stream");

#[derive(Default)]
struct StreamTile {
    app: *mut PlayerSurface,
    index: c_uint,
    label: *mut c_char,
    channel: *mut c_char,

    frame: *mut GtkWidget,
    container: *mut GtkWidget,
    overlay: *mut GtkWidget,
    gl_area: *mut GtkWidget,
    footer: *mut GtkWidget,
    channel_combo: *mut GtkWidget,
    channel_label: *mut GtkWidget,
    channel_refresh_button: *mut GtkWidget,
    close_button: *mut GtkWidget,
    empty_label: *mut GtkWidget,
    status_label: Option<gtk4::Label>,
    focus_button: *mut GtkWidget,
    chat_toggle_button: *mut GtkWidget,
    stream_info_button: *mut GtkWidget,
    mute_button: *mut GtkWidget,
    volume_scale: *mut GtkWidget,

    stream_settings_popover: *mut GtkWidget,
    quality_list_box: *mut GtkWidget,
    quality_status_label: *mut GtkWidget,
    twitch_playback_panel: *mut GtkWidget,
    twitch_playback_status_label: *mut GtkWidget,

    channel_switcher: *mut ChannelSwitcherOverlay,
    chat_panel: *mut ChatPanel,

    session: *mut PlayerSession,
    title_cancel: *mut GCancellable,
    stream_info: *mut PlayerFooterStreamInfo,
    stream_quality: PlayerStreamQualityState,

    mpv_gl: Option<Box<PlayerRenderContext>>,
    last_render_width: c_int,
    last_render_height: c_int,
    render_queued: c_int,
    event_queued: c_int,
    render_warmup_source: c_uint,
    title_generation: c_uint,
    render_warmup_frames: c_int,

    chat_width: c_int,
    last_chat_paned_width: c_int,
    chat_position_source: c_uint,
    chat_visible: c_int,

    owns_session: c_int,
    title_fetch_in_progress: c_int,
    wakeup_callback_ref_held: c_int,
    render_callback_ref_held: c_int,
}

#[derive(Default)]
pub struct PlayerSurface {
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
    picker_cache: SharedPickerCache,

    visible_footer_tile: *mut StreamTile,
    footer_hide_source: c_uint,
    title_refresh_source: c_uint,
    focused_tile: c_uint,
    motion_tracker: PlayerMotionTracker,

    fullscreen_callback: PlayerSurfaceFullscreenCallback,
    fullscreen_user_data: *mut c_void,
    settings_callback: PlayerSurfaceSettingsCallback,
    settings_user_data: *mut c_void,

    move_press_x: c_double,
    move_press_y: c_double,
    move_pressed: c_int,

    closing: AtomicBool,
    fullscreen: c_int,
    tile_focused: c_int,
    active_layout_index: usize,
    single_template_active: c_int,
    single_template_tile: c_uint,
    video_fullscreen_active: c_int,
    video_fullscreen_restore_app_fullscreen: c_int,
    video_fullscreen_restore_tile_focused: c_int,
    video_fullscreen_restore_focused_tile: c_uint,

    started: c_int,
    ref_count: AtomicUsize,
}

struct StreamTitleCallbackData {
    tile: *mut StreamTile,
    generation: c_uint,
}

struct StreamQualityCallbackData {
    tile: *mut StreamTile,
    generation: c_uint,
}

type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

type GClosureNotify = unsafe extern "C" fn(*mut c_void, *mut c_void);

type GType = usize;

pub type PlayerSurfaceFullscreenCallback = Option<unsafe extern "C" fn(*mut c_void)>;

pub type PlayerSurfaceSettingsCallback = Option<unsafe extern "C" fn(*mut c_void)>;

pub struct PlayerSurfaceCallbacks {
    pub fullscreen: PlayerSurfaceFullscreenCallback,
    pub fullscreen_data: *mut c_void,
    pub settings: PlayerSurfaceSettingsCallback,
    pub settings_data: *mut c_void,
}

unsafe fn player_surface_ref(state: *mut PlayerSurface) -> *mut PlayerSurface {
    unsafe {
        if !state.is_null() {
            (*state).ref_count.fetch_add(1, Ordering::Relaxed);
        }
        state
    }
}

unsafe fn player_surface_unref(state: *mut PlayerSurface) {
    unsafe {
        if state.is_null() || (*state).ref_count.fetch_sub(1, Ordering::AcqRel) != 1 {
            return;
        }

        drop(Box::from_raw(state));
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
/// - Release owner once; no subsequent pointer access.
pub unsafe fn player_surface_free(player: *mut PlayerSurface) {
    unsafe {
        let state = player;

        if state.is_null() {
            return;
        }

        if (*state).closing.swap(true, Ordering::AcqRel) {
            return;
        }

        remove_source_if_active(&mut (*state).footer_hide_source);
        remove_source_if_active(&mut (*state).title_refresh_source);

        for i in 0..MAX_TILES {
            let tile = &mut (*state).tiles[i] as *mut StreamTile;

            remove_source_if_active(&mut (*tile).chat_position_source);
            if !(*tile).container.is_null() {
                capture_chat_width_from_paned(tile);
                gtk_paned_set_end_child((*tile).container as *mut GtkPaned, ptr::null_mut());
            }
            clear_tile_render_context(tile);
            reset_tile_stream_title(tile);
            clear_tile_stream_qualities(tile);
            disable_tile_wakeup_callback(tile);
            if (*tile).owns_session != 0 {
                player_session_free((*tile).session);
            }
            (*tile).session = ptr::null_mut();

            clear_pointer(&mut (*tile).label);
            clear_pointer(&mut (*tile).channel);
            remove_weak_pointer(&mut (*tile).frame);
            remove_weak_pointer(&mut (*tile).container);
            remove_weak_pointer(&mut (*tile).overlay);
            remove_weak_pointer(&mut (*tile).gl_area);
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
            (*tile).status_label.take();
            (*tile).chat_toggle_button = ptr::null_mut();
            (*tile).stream_info_button = ptr::null_mut();
            (*tile).mute_button = ptr::null_mut();
            (*tile).volume_scale = ptr::null_mut();
            if !(*tile).stream_settings_popover.is_null() {
                gtk_widget_unparent((*tile).stream_settings_popover);
            }
            (*tile).stream_settings_popover = ptr::null_mut();
            (*tile).quality_list_box = ptr::null_mut();
            (*tile).quality_status_label = ptr::null_mut();
            (*tile).twitch_playback_panel = ptr::null_mut();
            (*tile).twitch_playback_status_label = ptr::null_mut();
            channel_switcher_overlay_free((*tile).channel_switcher);
            (*tile).channel_switcher = ptr::null_mut();
            chat_panel_free((*tile).chat_panel);
            (*tile).chat_panel = ptr::null_mut();
        }

        for i in 0..MAX_TILES {
            clear_pointer(&mut (*state).targets[i]);
        }

        remove_weak_pointer(&mut (*state).grid);
        remove_weak_pointer(&mut (*state).root_overlay);
        (*state).primary_session = ptr::null_mut();
        (*state).settings = ptr::null_mut();
        player_surface_unref(state);
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `window`: live `GtkWindow`; settings/session outlive the surface.
/// - Non-null `targets`: `target_count` readable C-string pointers.
/// - Callbacks: declared C ABI, no unwind; data outlives surface.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn player_surface_new<W>(
    window: *mut W,
    settings: *mut AppSettings,
    primary_session: *mut PlayerSession,
    targets: *const *const c_char,
    target_count: c_uint,
    callbacks: PlayerSurfaceCallbacks,
) -> *mut PlayerSurface {
    unsafe {
        let window = window as *mut GtkWindow;
        install_css();

        let state = Box::into_raw(Box::new(PlayerSurface {
            ref_count: AtomicUsize::new(1),
            ..Default::default()
        }));
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
        (*state).fullscreen_callback = callbacks.fullscreen;
        (*state).fullscreen_user_data = callbacks.fullscreen_data;
        (*state).settings_callback = callbacks.settings;
        (*state).settings_user_data = callbacks.settings_data;
        (*state).active_layout_index = PLAYER_LAYOUT_2X2_INDEX;

        (*state).root_overlay = gtk_overlay_new();
        add_weak_pointer((*state).root_overlay, &mut (*state).root_overlay);
        gtk_widget_add_css_class((*state).root_overlay, c"grid-root".as_ptr());
        gtk_widget_set_hexpand((*state).root_overlay, TRUE);
        gtk_widget_set_vexpand((*state).root_overlay, TRUE);

        (*state).grid = gtk_grid_new();
        add_weak_pointer((*state).grid, &mut (*state).grid);
        gtk_widget_add_css_class((*state).grid, c"stream-grid".as_ptr());
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

            let cell = PLAYER_LAYOUTS[PLAYER_LAYOUT_2X2_INDEX]
                .cells
                .get(i as usize)
                .copied()
                .unwrap_or(PlayerLayoutCell::new(0, 0, 1, 1));
            gtk_grid_attach(
                (*state).grid as *mut GtkGrid,
                tile_widget,
                cell.column,
                cell.row,
                cell.column_span,
                cell.row_span,
            );
            (*state).grid_items[i as usize] = tile_widget;
        }
        restore_active_layout_with_primary_slot(state, 0);

        schedule_footer_hide(state);
        (*state).title_refresh_source = add_surface_seconds_timeout(
            STREAM_TITLE_REFRESH_SECONDS,
            Some(refresh_surface_stream_titles),
            state,
        );

        state
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
/// - Result: borrowed GTK widget; compatible `W`; invalid after widget destruction.
pub unsafe fn player_surface_get_widget<W>(player: *mut PlayerSurface) -> *mut W {
    unsafe {
        if !player.is_null() {
            (*player).root_overlay as *mut W
        } else {
            ptr::null_mut()
        }
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
pub unsafe fn player_surface_start(player: *mut PlayerSurface) {
    unsafe {
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
                    set_tile_status(tile, None);
                    request_tile_title_update(tile, TRUE);
                    continue;
                }
                load_tile_stream(tile);
            }
        }
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
pub unsafe fn player_surface_show_overlay(player: *mut PlayerSurface) {
    unsafe {
        if player.is_null() {
            return;
        }

        let tile = if (*player).tile_focused != 0 && (*player).focused_tile < MAX_TILES as c_uint {
            &mut (*player).tiles[(*player).focused_tile as usize] as *mut StreamTile
        } else {
            preferred_single_tile(player)
        };
        show_tile_overlay(tile);
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
/// - `settings`: live until replacement/free; serialize mutations.
pub unsafe fn player_surface_set_settings(player: *mut PlayerSurface, settings: *mut AppSettings) {
    unsafe {
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
}
