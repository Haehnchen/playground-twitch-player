use super::controls::show_tile_overlay;
use super::ffi::{
    GDK_BUTTON_PRESS, GDK_BUTTON_PRIMARY, GDK_BUTTON_RELEASE, GDK_BUTTON1_MASK, GDK_CONTROL_MASK,
    GDK_EVENT_PROPAGATE, GDK_EVENT_STOP, GDK_KEY_M_LOWER, GDK_KEY_M_UPPER, GDK_MOTION_NOTIFY,
    GdkDevice, GdkEvent, GdkToplevel, GtkEventControllerLegacy, GtkEventControllerMotion,
    GtkEventControllerScroll, GtkGestureClick, GtkWidget, gdk_button_event_get_button,
    gdk_event_get_device, gdk_event_get_event_type, gdk_event_get_modifier_state,
    gdk_event_get_position, gdk_event_get_time, gdk_toplevel_begin_move, gdk_toplevel_get_type,
    gtk_native_get_surface, gtk_widget_get_native, gtk_widget_get_sensitive,
};
use super::layout::preferred_single_tile;
use super::layout::request_tile_fullscreen_toggle;
use super::playback::show_tile_channel_switcher;
use super::runtime::{is_instance, tile_is_closing};
use super::{FALSE, PlayerSurface, StreamTile, TRUE};
use crate::player::channel_switcher_overlay::channel_switcher_overlay_is_visible;
use crate::player::controls::{player_volume_apply_scroll, player_volume_toggle_muted};
use crate::player::motion::player_motion_tracker_ignore_stationary;
use std::ffi::{c_double, c_int, c_uint, c_void};
use std::ptr;

pub(super) unsafe fn get_toplevel_event_data_from_event(
    window: *mut GtkWidget,
    event: *mut GdkEvent,
    toplevel_out: *mut *mut GdkToplevel,
    device_out: *mut *mut GdkDevice,
    x_out: *mut c_double,
    y_out: *mut c_double,
    timestamp_out: *mut c_uint,
) -> c_int {
    unsafe {
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
}

pub(super) unsafe fn begin_window_move_from_event(
    state: *mut PlayerSurface,
    event: *mut GdkEvent,
    button: c_uint,
) {
    unsafe {
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
}

pub(super) unsafe extern "C" fn on_tile_motion(
    _controller: *mut GtkEventControllerMotion,
    x: c_double,
    y: c_double,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

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
}

pub(super) unsafe extern "C" fn on_video_pressed(
    _gesture: *mut GtkGestureClick,
    n_press: c_int,
    _x: c_double,
    _y: c_double,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if n_press == 2 && !tile_is_closing(tile) {
            request_tile_fullscreen_toggle(tile);
        }
    }
}

pub(super) unsafe extern "C" fn on_video_legacy_event(
    _controller: *mut GtkEventControllerLegacy,
    event: *mut GdkEvent,
    user_data: *mut c_void,
) -> c_int {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return GDK_EVENT_PROPAGATE;
        }

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
}

pub(super) unsafe extern "C" fn on_tile_scroll(
    _controller: *mut GtkEventControllerScroll,
    dx: c_double,
    dy: c_double,
    user_data: *mut c_void,
) -> c_int {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return GDK_EVENT_PROPAGATE;
        }

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
}

pub(super) unsafe extern "C" fn on_context_pressed(
    _gesture: *mut GtkGestureClick,
    n_press: c_int,
    x: c_double,
    y: c_double,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        if n_press != 1 {
            return;
        }

        show_tile_channel_switcher(tile, x, y);
        show_tile_overlay(tile);
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
pub unsafe fn player_surface_handle_key(
    player: *mut PlayerSurface,
    keyval: c_uint,
    modifiers: c_uint,
) -> c_int {
    unsafe {
        if player.is_null() || (modifiers & GDK_CONTROL_MASK) != 0 {
            return GDK_EVENT_PROPAGATE;
        }

        if keyval != GDK_KEY_M_LOWER && keyval != GDK_KEY_M_UPPER {
            return GDK_EVENT_PROPAGATE;
        }

        let tile = preferred_single_tile(player);
        if tile.is_null() || (*tile).mute_button.is_null() {
            return GDK_EVENT_PROPAGATE;
        }

        player_volume_toggle_muted((*tile).session, (*tile).mute_button);
        show_tile_overlay(tile);
        GDK_EVENT_STOP
    }
}
