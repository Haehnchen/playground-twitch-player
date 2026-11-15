#![allow(clashing_extern_declarations)]

mod content;
mod desktop;
mod ffi;
mod runtime;
mod style;
#[cfg(test)]
mod tests {
    use super::{AppState, app_state_ref, app_state_unref, destroy_state};
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::atomic::Ordering;

    #[test]
    fn owner_release_waits_for_pending_callback_reference() {
        unsafe {
            let state = Box::into_raw(Box::new(AppState::new(
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            )));
            let pending = app_state_ref(state);

            destroy_state(state as *mut c_void);

            assert!((*pending).closing.load(Ordering::Acquire));
            assert_eq!((*pending).ref_count.load(Ordering::Acquire), 1);

            app_state_unref(pending);
        }
    }
}
mod window;

use self::content::{
    create_layout_popover, destroy_active_content, on_layout_menu_clicked, on_settings_clicked,
    set_layout_mode,
};
use self::desktop::{install_app_icon, write_user_desktop_identity};
use self::ffi::{
    G_APPLICATION_NON_UNIQUE, GObject, GTK_ALIGN_END, GTK_ALIGN_START, GTK_ORIENTATION_HORIZONTAL,
    GTK_PHASE_CAPTURE, GtkApplication, GtkBox, GtkOverlay, GtkWidget, GtkWindow, g_application_run,
    g_object_set_data_full, g_object_unref, g_set_application_name, g_set_prgname, g_setenv,
    g_signal_connect_data, gtk_application_new, gtk_application_window_new, gtk_box_append,
    gtk_box_new, gtk_event_controller_key_new, gtk_event_controller_motion_new,
    gtk_event_controller_set_propagation_phase, gtk_overlay_add_overlay, gtk_overlay_new,
    gtk_widget_add_controller, gtk_widget_add_css_class, gtk_widget_set_halign,
    gtk_widget_set_valign, gtk_widget_unparent, gtk_window_present, gtk_window_set_child,
    gtk_window_set_decorated, gtk_window_set_default_size, gtk_window_set_icon_name,
    gtk_window_set_title,
};
use self::runtime::{
    add_weak_pointer, connect_app_state_signal, remove_source_if_active, remove_weak_pointer,
};
use self::style::install_css;
use self::window::{
    add_resize_handles, on_close_clicked, on_fullscreen_clicked, on_key_pressed,
    on_minimize_clicked, on_root_motion, on_window_close_request, on_window_fullscreen_changed,
    on_window_maximized_changed, schedule_window_overlay_hide,
};
use crate::player::controls::player_overlay_button_new;
use crate::player::icons::{
    player_layout_menu_icon_new, player_settings_icon_new, player_window_icon_new,
};
use crate::player::layout::{
    PLAYER_LAYOUT_2X2_INDEX, PLAYER_LAYOUT_MAX_TILES, PLAYER_LAYOUT_SINGLE_INDEX,
};
use crate::player::motion::PlayerMotionTracker;
use crate::player::session::{
    PlayerSession, player_session_free, player_session_new, player_session_set_hwdec_enabled,
};
use crate::player::surface::PlayerSurface;
use crate::settings::{
    AppSettings, app_settings_free, app_settings_get_hwdec_enabled, app_settings_load,
};
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::os::unix::ffi::OsStringExt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::{env, ptr};
const DEFAULT_WINDOW_WIDTH: c_int = 1100;
const DEFAULT_WINDOW_HEIGHT: c_int = (DEFAULT_WINDOW_WIDTH * 9 + 8) / 16;
const PLAYER_SURFACE_MAX_TILES: usize = PLAYER_LAYOUT_MAX_TILES;
const FALSE: c_int = 0;
const TRUE: c_int = 1;
const PLAYER_WINDOW_ICON_MINIMIZE: c_int = 0;
const PLAYER_WINDOW_ICON_FULLSCREEN: c_int = 1;
const PLAYER_WINDOW_ICON_CLOSE: c_int = 2;

struct AppState {
    window: *mut GtkWidget,
    root_overlay: *mut GtkWidget,
    top_left_controls: *mut GtkWidget,
    top_controls: *mut GtkWidget,
    settings_button: *mut GtkWidget,
    layout_popover: *mut GtkWidget,

    settings: *mut AppSettings,
    primary_session: *mut PlayerSession,
    player_surface: *mut PlayerSurface,

    startup_target: *const c_char,
    initial_targets: *const *const c_char,
    initial_target_count: c_uint,

    overlay_hide_source: c_uint,
    maximize_restore_source: c_uint,
    maximize_restore_attempts: c_uint,
    motion_tracker: PlayerMotionTracker,
    closing: AtomicBool,

    fullscreen: c_int,
    window_maximized: c_int,
    was_maximized_before_fullscreen: c_int,
    restore_maximized_after_fullscreen: c_int,
    restore_window_width: c_int,
    restore_window_height: c_int,

    ref_count: AtomicUsize,
}

struct StartupConfig {
    startup_target: *const c_char,
    initial_targets: *const *const c_char,
    initial_target_count: c_uint,
    start_in_2x2: c_int,
}

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

type GClosureNotify = unsafe extern "C" fn(*mut c_void, *mut c_void);

type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;

type GType = usize;

unsafe fn c_app_id() -> *const c_char {
    c"local.twitchplayer".as_ptr()
}

unsafe fn configure_rendering_defaults() {
    unsafe {
        g_setenv(c"GSK_RENDERER".as_ptr(), c"gl".as_ptr(), FALSE);
    }
}

unsafe fn app_state_ref(state: *mut AppState) -> *mut AppState {
    unsafe {
        if !state.is_null() {
            (*state).ref_count.fetch_add(1, Ordering::Relaxed);
        }
        state
    }
}

unsafe fn app_state_unref(state: *mut AppState) {
    unsafe {
        if state.is_null() || (*state).ref_count.fetch_sub(1, Ordering::AcqRel) != 1 {
            return;
        }

        drop(Box::from_raw(state));
    }
}

// Stop callbacks and release playback before freeing settings.
unsafe fn shutdown_state(state: *mut AppState) {
    unsafe {
        if (*state).closing.swap(true, Ordering::AcqRel) {
            return;
        }

        remove_source_if_active(&mut (*state).overlay_hide_source);
        remove_source_if_active(&mut (*state).maximize_restore_source);

        if !(*state).layout_popover.is_null() {
            gtk_widget_unparent((*state).layout_popover);
        }
        destroy_active_content(state);
        if !(*state).primary_session.is_null() {
            player_session_free((*state).primary_session);
            (*state).primary_session = ptr::null_mut();
        }
        app_settings_free((*state).settings);
        (*state).settings = ptr::null_mut();
    }
}

unsafe extern "C" fn destroy_state(user_data: *mut c_void) {
    unsafe {
        let state = user_data as *mut AppState;
        shutdown_state(state);

        remove_weak_pointer(&mut (*state).layout_popover);
        remove_weak_pointer(&mut (*state).top_controls);
        remove_weak_pointer(&mut (*state).top_left_controls);
        remove_weak_pointer(&mut (*state).root_overlay);
        (*state).settings_button = ptr::null_mut();
        (*state).window = ptr::null_mut();

        app_state_unref(state);
    }
}

unsafe extern "C" fn on_activate(application: *mut GtkApplication, user_data: *mut c_void) {
    unsafe {
        // GtkApplication has initialized GTK on this main thread.
        gtk4::set_initialized();
        let config = user_data as *mut StartupConfig;

        install_css();
        install_app_icon();

        let state = Box::into_raw(Box::new(AppState::new(
            app_settings_load(),
            player_session_new(),
            config,
        )));
        player_session_set_hwdec_enabled(
            (*state).primary_session,
            app_settings_get_hwdec_enabled((*state).settings),
        );

        (*state).window = gtk_application_window_new(application);
        gtk_window_set_title((*state).window as *mut GtkWindow, c"Twitch Player".as_ptr());
        gtk_window_set_default_size(
            (*state).window as *mut GtkWindow,
            DEFAULT_WINDOW_WIDTH,
            DEFAULT_WINDOW_HEIGHT,
        );
        gtk_window_set_decorated((*state).window as *mut GtkWindow, FALSE);
        gtk_window_set_icon_name((*state).window as *mut GtkWindow, c_app_id());
        connect_app_state_signal(
            (*state).window as *mut c_void,
            c"notify::fullscreened".as_ptr(),
            on_window_fullscreen_changed as *const c_void,
            state,
        );
        connect_app_state_signal(
            (*state).window as *mut c_void,
            c"notify::maximized".as_ptr(),
            on_window_maximized_changed as *const c_void,
            state,
        );
        connect_app_state_signal(
            (*state).window as *mut c_void,
            c"close-request".as_ptr(),
            on_window_close_request as *const c_void,
            state,
        );

        (*state).root_overlay = gtk_overlay_new();
        add_weak_pointer((*state).root_overlay, &mut (*state).root_overlay);
        gtk_window_set_child((*state).window as *mut GtkWindow, (*state).root_overlay);
        add_resize_handles((*state).root_overlay as *mut GtkOverlay, state);

        (*state).top_left_controls = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
        add_weak_pointer((*state).top_left_controls, &mut (*state).top_left_controls);
        gtk_widget_add_css_class((*state).top_left_controls, c"top-overlay-controls".as_ptr());
        gtk_widget_set_halign((*state).top_left_controls, GTK_ALIGN_START);
        gtk_widget_set_valign((*state).top_left_controls, GTK_ALIGN_START);
        gtk_overlay_add_overlay(
            (*state).root_overlay as *mut GtkOverlay,
            (*state).top_left_controls,
        );

        (*state).settings_button =
            player_overlay_button_new(player_settings_icon_new(), c"Settings".as_ptr());
        gtk_widget_add_css_class(
            (*state).settings_button,
            c"settings-overlay-button".as_ptr(),
        );
        gtk_box_append(
            (*state).top_left_controls as *mut GtkBox,
            (*state).settings_button,
        );
        connect_app_state_signal(
            (*state).settings_button as *mut c_void,
            c"clicked".as_ptr(),
            on_settings_clicked as *const c_void,
            state,
        );

        let layout_menu_button =
            player_overlay_button_new(player_layout_menu_icon_new(), c"Select layout".as_ptr());
        gtk_widget_add_css_class(layout_menu_button, c"settings-overlay-button".as_ptr());
        gtk_box_append(
            (*state).top_left_controls as *mut GtkBox,
            layout_menu_button,
        );
        connect_app_state_signal(
            layout_menu_button as *mut c_void,
            c"clicked".as_ptr(),
            on_layout_menu_clicked as *const c_void,
            state,
        );

        (*state).layout_popover = create_layout_popover(state, layout_menu_button);
        add_weak_pointer((*state).layout_popover, &mut (*state).layout_popover);

        (*state).top_controls = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
        add_weak_pointer((*state).top_controls, &mut (*state).top_controls);
        gtk_widget_add_css_class((*state).top_controls, c"top-overlay-controls".as_ptr());
        gtk_widget_set_halign((*state).top_controls, GTK_ALIGN_END);
        gtk_widget_set_valign((*state).top_controls, GTK_ALIGN_START);
        gtk_overlay_add_overlay(
            (*state).root_overlay as *mut GtkOverlay,
            (*state).top_controls,
        );

        let minimize_button = player_overlay_button_new(
            player_window_icon_new(PLAYER_WINDOW_ICON_MINIMIZE),
            c"Minimize".as_ptr(),
        );
        gtk_box_append((*state).top_controls as *mut GtkBox, minimize_button);
        connect_app_state_signal(
            minimize_button as *mut c_void,
            c"clicked".as_ptr(),
            on_minimize_clicked as *const c_void,
            state,
        );

        let fullscreen_button = player_overlay_button_new(
            player_window_icon_new(PLAYER_WINDOW_ICON_FULLSCREEN),
            c"Fullscreen".as_ptr(),
        );
        gtk_box_append((*state).top_controls as *mut GtkBox, fullscreen_button);
        connect_app_state_signal(
            fullscreen_button as *mut c_void,
            c"clicked".as_ptr(),
            on_fullscreen_clicked as *const c_void,
            state,
        );

        let close_button = player_overlay_button_new(
            player_window_icon_new(PLAYER_WINDOW_ICON_CLOSE),
            c"Close".as_ptr(),
        );
        gtk_widget_add_css_class(close_button, c"close-button".as_ptr());
        gtk_box_append((*state).top_controls as *mut GtkBox, close_button);
        connect_app_state_signal(
            close_button as *mut c_void,
            c"clicked".as_ptr(),
            on_close_clicked as *const c_void,
            state,
        );

        let motion = gtk_event_controller_motion_new();
        gtk_event_controller_set_propagation_phase(motion, GTK_PHASE_CAPTURE);
        connect_app_state_signal(
            motion as *mut c_void,
            c"motion".as_ptr(),
            on_root_motion as *const c_void,
            state,
        );
        gtk_widget_add_controller((*state).root_overlay, motion as *mut c_void);

        let key_controller = gtk_event_controller_key_new();
        gtk_event_controller_set_propagation_phase(key_controller, GTK_PHASE_CAPTURE);
        connect_app_state_signal(
            key_controller as *mut c_void,
            c"key-pressed".as_ptr(),
            on_key_pressed as *const c_void,
            state,
        );
        gtk_widget_add_controller((*state).window, key_controller as *mut c_void);

        g_object_set_data_full(
            (*state).window as *mut GObject,
            c"app-state".as_ptr(),
            state as *mut c_void,
            Some(destroy_state),
        );

        let start_2x2 = !config.is_null() && (*config).start_in_2x2 != 0;
        set_layout_mode(
            state,
            if start_2x2 {
                PLAYER_LAYOUT_2X2_INDEX
            } else {
                PLAYER_LAYOUT_SINGLE_INDEX
            },
        );
        gtk_window_present((*state).window as *mut GtkWindow);
        schedule_window_overlay_hide(state);
    }
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
    unsafe {
        let mut initial_targets: [*const c_char; PLAYER_SURFACE_MAX_TILES] =
            [ptr::null(); PLAYER_SURFACE_MAX_TILES];
        let mut initial_target_count: c_uint = 0;
        let mut start_in_2x2 = FALSE;
        let mut startup_target: *const c_char = ptr::null();
        let mut argv: Vec<*mut c_char> =
            args.iter().map(|arg| arg.as_ptr() as *mut c_char).collect();
        argv.push(ptr::null_mut());

        for arg in args.iter().skip(1) {
            let arg = arg.as_ptr();
            if CStr::from_ptr(arg).to_bytes() == b"--grid" {
                start_in_2x2 = TRUE;
                continue;
            }

            if startup_target.is_null() {
                startup_target = arg;
            }
            if (initial_target_count as usize) < PLAYER_SURFACE_MAX_TILES {
                initial_targets[initial_target_count as usize] = arg;
                initial_target_count += 1;
            }
        }

        let mut config = StartupConfig {
            startup_target,
            initial_targets: initial_targets.as_ptr(),
            initial_target_count,
            start_in_2x2,
        };

        configure_rendering_defaults();
        g_set_prgname(c_app_id());
        g_set_application_name(c"Twitch Player".as_ptr());
        write_user_desktop_identity(argv[0]);

        let application = gtk_application_new(c_app_id(), G_APPLICATION_NON_UNIQUE);
        g_signal_connect_data(
            application as *mut c_void,
            c"activate".as_ptr(),
            on_activate as *const c_void,
            &mut config as *mut StartupConfig as *mut c_void,
            None,
            0,
        );

        let status = g_application_run(application as *mut c_void, 1, argv.as_mut_ptr());
        g_object_unref(application as *mut c_void);
        status
    }
}

impl AppState {
    unsafe fn new(
        settings: *mut AppSettings,
        primary_session: *mut PlayerSession,
        config: *const StartupConfig,
    ) -> Self {
        unsafe {
            Self {
                window: ptr::null_mut(),
                root_overlay: ptr::null_mut(),
                top_left_controls: ptr::null_mut(),
                top_controls: ptr::null_mut(),
                settings_button: ptr::null_mut(),
                layout_popover: ptr::null_mut(),

                settings,
                primary_session,
                player_surface: ptr::null_mut(),

                startup_target: if config.is_null() {
                    ptr::null()
                } else {
                    (*config).startup_target
                },
                initial_targets: if config.is_null() {
                    ptr::null()
                } else {
                    (*config).initial_targets
                },
                initial_target_count: if config.is_null() {
                    0
                } else {
                    (*config).initial_target_count
                },

                overlay_hide_source: 0,
                maximize_restore_source: 0,
                maximize_restore_attempts: 0,
                motion_tracker: PlayerMotionTracker::new(),
                closing: AtomicBool::new(false),

                fullscreen: FALSE,
                window_maximized: FALSE,
                was_maximized_before_fullscreen: FALSE,
                restore_maximized_after_fullscreen: FALSE,
                restore_window_width: 0,
                restore_window_height: 0,

                ref_count: AtomicUsize::new(1),
            }
        }
    }
}
