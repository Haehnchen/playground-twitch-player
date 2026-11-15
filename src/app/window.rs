use super::ffi::{
    G_SOURCE_CONTINUE, G_SOURCE_REMOVE, GDK_BUTTON_PRIMARY, GDK_EVENT_PROPAGATE,
    GDK_SURFACE_EDGE_EAST, GDK_SURFACE_EDGE_NORTH, GDK_SURFACE_EDGE_NORTH_EAST,
    GDK_SURFACE_EDGE_NORTH_WEST, GDK_SURFACE_EDGE_SOUTH, GDK_SURFACE_EDGE_SOUTH_EAST,
    GDK_SURFACE_EDGE_SOUTH_WEST, GDK_SURFACE_EDGE_WEST, GObject, GParamSpec, GTK_ALIGN_END,
    GTK_ALIGN_FILL, GTK_ALIGN_START, GTK_ORIENTATION_VERTICAL, GdkDevice, GdkToplevel, GtkButton,
    GtkEventControllerKey, GtkEventControllerMotion, GtkGesture, GtkGestureClick, GtkGestureSingle,
    GtkOverlay, GtkWidget, GtkWindow, g_object_get_data, g_object_set_data, gdk_event_get_device,
    gdk_event_get_position, gdk_event_get_time, gdk_toplevel_begin_resize, gdk_toplevel_get_type,
    gtk_box_new, gtk_gesture_click_new, gtk_gesture_get_last_event,
    gtk_gesture_get_last_updated_sequence, gtk_gesture_single_set_button, gtk_native_get_surface,
    gtk_overlay_add_overlay, gtk_widget_add_controller, gtk_widget_add_css_class,
    gtk_widget_get_height, gtk_widget_get_native, gtk_widget_get_width,
    gtk_widget_set_cursor_from_name, gtk_widget_set_halign, gtk_widget_set_hexpand,
    gtk_widget_set_size_request, gtk_widget_set_valign, gtk_widget_set_vexpand,
    gtk_widget_set_visible, gtk_window_close, gtk_window_fullscreen, gtk_window_get_type,
    gtk_window_is_fullscreen, gtk_window_is_maximized, gtk_window_maximize, gtk_window_minimize,
    gtk_window_set_default_size, gtk_window_unfullscreen,
};
use super::runtime::{
    add_app_state_timeout, app_state_is_closing, connect_app_state_signal, is_instance,
    remove_source_if_active,
};
use super::{AppState, FALSE, TRUE, shutdown_state};
use crate::player::motion::player_motion_tracker_ignore_stationary;
use crate::player::surface::{
    player_surface_handle_key, player_surface_is_single_template, player_surface_set_fullscreen,
    player_surface_show_overlay,
};
use std::ffi::{c_char, c_double, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::Ordering;
pub(super) const OVERLAY_HIDE_DELAY_MS: c_uint = 1800;
pub(super) const MAXIMIZE_RESTORE_ATTEMPTS: c_uint = 12;

pub(super) unsafe extern "C" fn hide_window_overlay(user_data: *mut c_void) -> c_int {
    unsafe {
        let state = user_data as *mut AppState;
        (*state).overlay_hide_source = 0;

        if !(*state).closing.load(Ordering::Acquire) {
            gtk_widget_set_visible((*state).top_left_controls, FALSE);
            gtk_widget_set_visible((*state).top_controls, FALSE);
        }

        G_SOURCE_REMOVE
    }
}

pub(super) unsafe fn schedule_window_overlay_hide(state: *mut AppState) {
    unsafe {
        remove_source_if_active(&mut (*state).overlay_hide_source);
        if app_state_is_closing(state) {
            return;
        }
        (*state).overlay_hide_source =
            add_app_state_timeout(OVERLAY_HIDE_DELAY_MS, Some(hide_window_overlay), state);
    }
}

pub(super) unsafe fn show_window_overlay(state: *mut AppState) {
    unsafe {
        if (*state).closing.load(Ordering::Acquire) {
            return;
        }

        gtk_widget_set_visible((*state).top_left_controls, TRUE);
        gtk_widget_set_visible((*state).top_controls, TRUE);
        if !(*state).player_surface.is_null()
            && player_surface_is_single_template((*state).player_surface) != 0
        {
            player_surface_show_overlay((*state).player_surface);
        }
        schedule_window_overlay_hide(state);
    }
}

pub(super) unsafe fn get_toplevel_event_data(
    window: *mut GtkWidget,
    gesture: *mut GtkGesture,
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
}

pub(super) unsafe fn begin_window_resize(
    state: *mut AppState,
    gesture: *mut GtkGesture,
    edge: c_int,
) {
    unsafe {
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
}

pub(super) unsafe extern "C" fn on_resize_pressed(
    gesture: *mut GtkGestureClick,
    n_press: c_int,
    _x: c_double,
    _y: c_double,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;
        if app_state_is_closing(state) {
            return;
        }

        if n_press != 1 {
            return;
        }

        let edge =
            g_object_get_data(gesture as *mut GObject, c"resize-edge".as_ptr()) as isize as c_int;
        begin_window_resize(state, gesture as *mut GtkGesture, edge);
    }
}

pub(super) unsafe fn create_resize_handle(
    state: *mut AppState,
    edge: c_int,
    halign: c_int,
    valign: c_int,
    width: c_int,
    height: c_int,
    cursor: *const c_char,
) -> *mut GtkWidget {
    unsafe {
        let handle = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
        gtk_widget_add_css_class(handle, c"resize-handle".as_ptr());
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
            c"resize-edge".as_ptr(),
            edge as isize as *mut c_void,
        );
        connect_app_state_signal(
            click as *mut c_void,
            c"pressed".as_ptr(),
            on_resize_pressed as *const c_void,
            state,
        );
        gtk_widget_add_controller(handle, click.cast());

        handle
    }
}

pub(super) unsafe fn add_resize_handles(overlay: *mut GtkOverlay, state: *mut AppState) {
    unsafe {
        gtk_overlay_add_overlay(
            overlay,
            create_resize_handle(
                state,
                GDK_SURFACE_EDGE_NORTH,
                GTK_ALIGN_FILL,
                GTK_ALIGN_START,
                -1,
                6,
                c"n-resize".as_ptr(),
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
                c"s-resize".as_ptr(),
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
                c"w-resize".as_ptr(),
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
                c"e-resize".as_ptr(),
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
                c"nw-resize".as_ptr(),
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
                c"ne-resize".as_ptr(),
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
                c"sw-resize".as_ptr(),
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
                c"se-resize".as_ptr(),
            ),
        );
    }
}

pub(super) unsafe extern "C" fn restore_maximized_after_fullscreen(
    user_data: *mut c_void,
) -> c_int {
    unsafe {
        let state = user_data as *mut AppState;
        let window = if is_instance((*state).window, gtk_window_get_type()) {
            (*state).window as *mut GtkWindow
        } else {
            ptr::null_mut()
        };

        if (*state).closing.load(Ordering::Acquire)
            || window.is_null()
            || (*state).restore_maximized_after_fullscreen == 0
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
}

pub(super) unsafe fn schedule_maximized_restore_after_fullscreen(state: *mut AppState) {
    unsafe {
        remove_source_if_active(&mut (*state).maximize_restore_source);

        if app_state_is_closing(state) || (*state).restore_maximized_after_fullscreen == 0 {
            return;
        }

        (*state).maximize_restore_attempts = 0;
        (*state).maximize_restore_source =
            add_app_state_timeout(50, Some(restore_maximized_after_fullscreen), state);
    }
}

// Remember the window size and maximized state across fullscreen changes.
pub(super) unsafe fn set_fullscreen(state: *mut AppState, fullscreen: c_int) {
    unsafe {
        if app_state_is_closing(state) || (*state).fullscreen == fullscreen {
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

        if !(*state).player_surface.is_null() {
            player_surface_set_fullscreen((*state).player_surface, fullscreen);
        }

        show_window_overlay(state);
    }
}

pub(super) unsafe extern "C" fn on_window_fullscreen_changed(
    _object: *mut GObject,
    _pspec: *mut GParamSpec,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;
        if !app_state_is_closing(state) {
            schedule_maximized_restore_after_fullscreen(state);
        }
    }
}

pub(super) unsafe extern "C" fn on_window_maximized_changed(
    _object: *mut GObject,
    _pspec: *mut GParamSpec,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;

        if app_state_is_closing(state)
            || (*state).fullscreen != 0
            || (*state).window.is_null()
            || gtk_window_is_fullscreen((*state).window as *mut GtkWindow) != 0
        {
            return;
        }

        (*state).window_maximized = gtk_window_is_maximized((*state).window as *mut GtkWindow);
    }
}

pub(super) unsafe extern "C" fn on_window_close_request(
    _window: *mut GtkWindow,
    user_data: *mut c_void,
) -> c_int {
    unsafe {
        shutdown_state(user_data as *mut AppState);
        GDK_EVENT_PROPAGATE
    }
}

pub(super) unsafe fn toggle_fullscreen(state: *mut AppState) {
    unsafe {
        set_fullscreen(state, ((*state).fullscreen == 0) as c_int);
    }
}

pub(super) unsafe extern "C" fn on_minimize_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;
        if !app_state_is_closing(state) && !(*state).window.is_null() {
            gtk_window_minimize((*state).window as *mut GtkWindow);
        }
    }
}

pub(super) unsafe extern "C" fn on_fullscreen_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;
        if !app_state_is_closing(state) {
            toggle_fullscreen(state);
        }
    }
}

pub(super) unsafe extern "C" fn on_close_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    unsafe {
        let state = user_data as *mut AppState;
        if !app_state_is_closing(state) && !(*state).window.is_null() {
            gtk_window_close((*state).window as *mut GtkWindow);
        }
    }
}

pub(super) unsafe extern "C" fn on_root_motion(
    _controller: *mut GtkEventControllerMotion,
    x: c_double,
    y: c_double,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;
        if app_state_is_closing(state) {
            return;
        }

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
}

pub(super) unsafe extern "C" fn on_key_pressed(
    _controller: *mut GtkEventControllerKey,
    keyval: c_uint,
    _keycode: c_uint,
    modifiers: c_uint,
    user_data: *mut c_void,
) -> c_int {
    unsafe {
        let state = user_data as *mut AppState;
        if app_state_is_closing(state) {
            return GDK_EVENT_PROPAGATE;
        }

        if !(*state).player_surface.is_null() {
            return player_surface_handle_key((*state).player_surface, keyval, modifiers);
        }

        GDK_EVENT_PROPAGATE
    }
}
