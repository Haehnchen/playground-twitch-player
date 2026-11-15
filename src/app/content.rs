use super::ffi::{
    GObject, GTK_ALIGN_FILL, GTK_ORIENTATION_HORIZONTAL, GTK_ORIENTATION_VERTICAL, GTK_POS_BOTTOM,
    GtkBox, GtkButton, GtkLabel, GtkOverlay, GtkPopover, GtkWidget, GtkWindow, g_ascii_strcasecmp,
    g_free, g_object_get_data, g_object_set_data, g_strcmp0, gtk_box_append, gtk_box_new,
    gtk_button_new, gtk_button_set_child, gtk_label_new, gtk_label_set_xalign,
    gtk_overlay_get_type, gtk_overlay_set_child, gtk_popover_new, gtk_popover_popdown,
    gtk_popover_popup, gtk_popover_set_child, gtk_popover_set_has_arrow, gtk_popover_set_position,
    gtk_separator_new, gtk_widget_add_css_class, gtk_widget_set_halign, gtk_widget_set_hexpand,
    gtk_widget_set_parent,
};
use super::runtime::{
    app_state_is_closing, app_state_source_data_free, connect_app_state_signal, is_instance,
    remove_source_if_active,
};
use super::window::{schedule_window_overlay_hide, show_window_overlay, toggle_fullscreen};
use super::{AppState, FALSE, PLAYER_SURFACE_MAX_TILES, TRUE, app_state_ref};
use crate::glib_strings::{dup_bytes, is_nonempty};
use crate::player::icons::player_layout_icon_new;
use crate::player::layout::PLAYER_LAYOUTS;
use crate::player::surface::{
    PlayerSurfaceCallbacks, player_surface_apply_layout, player_surface_free,
    player_surface_get_layout_index, player_surface_get_widget, player_surface_new,
    player_surface_set_fullscreen, player_surface_set_settings, player_surface_start,
};
use crate::settings::AppSettings;
use crate::settings::window::settings_window_show;
use crate::twitch::channel::normalize_twitch_channel;
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::ptr;
pub(super) const SETTINGS_WINDOW_PAGE_GENERAL: c_int = 0;
pub(super) const SETTINGS_WINDOW_PAGE_CHANNELS: c_int = 1;

pub(super) unsafe fn dup_twitch_channel_name(value: *const c_char) -> *mut c_char {
    unsafe {
        if !is_nonempty(value) {
            return ptr::null_mut();
        }

        normalize_twitch_channel(CStr::from_ptr(value).to_bytes())
            .map_or(ptr::null_mut(), |channel| dup_bytes(&channel))
    }
}

pub(super) unsafe extern "C" fn on_content_fullscreen_requested(user_data: *mut c_void) {
    unsafe {
        let state = user_data as *mut AppState;
        if !app_state_is_closing(state) {
            toggle_fullscreen(state);
        }
    }
}

pub(super) unsafe extern "C" fn on_content_settings_requested(user_data: *mut c_void) {
    unsafe {
        let state = user_data as *mut AppState;
        if !app_state_is_closing(state) {
            show_settings_window(state, SETTINGS_WINDOW_PAGE_CHANNELS);
        }
    }
}

pub(super) unsafe fn destroy_active_content(state: *mut AppState) {
    unsafe {
        if !(*state).player_surface.is_null() {
            player_surface_free((*state).player_surface);
            (*state).player_surface = ptr::null_mut();
        }

        if is_instance((*state).root_overlay, gtk_overlay_get_type()) {
            gtk_overlay_set_child((*state).root_overlay as *mut GtkOverlay, ptr::null_mut());
        }
    }
}

// Reuse the primary session when rebuilding the player layout.
pub(super) unsafe fn create_player_surface(state: *mut AppState) {
    unsafe {
        let mut targets: [*const c_char; PLAYER_SURFACE_MAX_TILES] =
            [ptr::null(); PLAYER_SURFACE_MAX_TILES];
        let mut target_storage: [*mut c_char; PLAYER_SURFACE_MAX_TILES] =
            [ptr::null_mut(); PLAYER_SURFACE_MAX_TILES];
        let mut target_count: usize = 0;

        // Keep the startup channel in slot 0 so the primary mpv session remains the stable player.
        let first_target = (*state).startup_target;

        if is_nonempty(first_target) {
            target_storage[target_count] = dup_twitch_channel_name(first_target);
            if !target_storage[target_count].is_null() {
                targets[target_count] = target_storage[target_count];
                target_count += 1;
            }
        }

        let initial_count = (*state).initial_target_count as usize;
        for i in 0..initial_count {
            if target_count >= PLAYER_SURFACE_MAX_TILES {
                break;
            }
            let target = *(*state).initial_targets.add(i);
            if !is_nonempty(target) {
                continue;
            }
            if !first_target.is_null() && g_strcmp0(target, first_target) == 0 {
                continue;
            }

            let channel = dup_twitch_channel_name(target);
            if channel.is_null() {
                continue;
            }

            // The surface may receive the same channel from startup and the 2x2 seed list.
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

        (*state).player_surface = player_surface_new(
            (*state).window as *mut GtkWindow,
            (*state).settings,
            (*state).primary_session,
            targets.as_ptr(),
            target_count as c_uint,
            PlayerSurfaceCallbacks {
                fullscreen: Some(on_content_fullscreen_requested),
                fullscreen_data: state as *mut c_void,
                settings: Some(on_content_settings_requested),
                settings_data: state as *mut c_void,
            },
        );
        player_surface_set_fullscreen((*state).player_surface, (*state).fullscreen);
        gtk_overlay_set_child(
            (*state).root_overlay as *mut GtkOverlay,
            player_surface_get_widget((*state).player_surface),
        );
        for target in target_storage {
            g_free(target as *mut c_void);
        }
    }
}

pub(super) unsafe fn apply_layout_template(state: *mut AppState, layout_index: usize) {
    unsafe {
        if (*state).player_surface.is_null() {
            create_player_surface(state);
        }

        player_surface_apply_layout((*state).player_surface, layout_index);
        player_surface_start((*state).player_surface);
    }
}

pub(super) unsafe fn set_layout_mode(state: *mut AppState, layout_index: usize) {
    unsafe {
        if layout_index >= PLAYER_LAYOUTS.len() {
            return;
        }

        if (*state).player_surface.is_null() {
            apply_layout_template(state, layout_index);
            show_window_overlay(state);
            return;
        }

        if player_surface_get_layout_index((*state).player_surface) == layout_index {
            show_window_overlay(state);
            return;
        }

        apply_layout_template(state, layout_index);
        show_window_overlay(state);
    }
}

pub(super) unsafe extern "C" fn on_layout_choice_clicked(
    button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;
        if app_state_is_closing(state) {
            return;
        }

        let stored_index =
            g_object_get_data(button as *mut GObject, c"layout-index".as_ptr()) as usize;
        if stored_index == 0 {
            return;
        }

        set_layout_mode(state, stored_index - 1);
        if !(*state).layout_popover.is_null() {
            gtk_popover_popdown((*state).layout_popover as *mut GtkPopover);
        }
    }
}

pub(super) unsafe extern "C" fn on_layout_menu_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;
        if app_state_is_closing(state) || (*state).layout_popover.is_null() {
            return;
        }

        remove_source_if_active(&mut (*state).overlay_hide_source);
        gtk_popover_popup((*state).layout_popover as *mut GtkPopover);
    }
}

pub(super) unsafe extern "C" fn on_layout_popover_closed(
    _popover: *mut GtkPopover,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;
        if !app_state_is_closing(state) {
            schedule_window_overlay_hide(state);
        }
    }
}

pub(super) unsafe fn create_layout_popover(
    state: *mut AppState,
    relative_to: *mut GtkWidget,
) -> *mut GtkWidget {
    unsafe {
        let popover = gtk_popover_new();
        gtk_widget_add_css_class(popover, c"stream-settings-popover".as_ptr());
        gtk_popover_set_position(popover as *mut GtkPopover, GTK_POS_BOTTOM);
        gtk_popover_set_has_arrow(popover as *mut GtkPopover, FALSE);
        gtk_widget_set_parent(popover, relative_to);

        let menu = gtk_box_new(GTK_ORIENTATION_VERTICAL, 2);
        gtk_widget_add_css_class(menu, c"stream-settings-menu".as_ptr());
        gtk_popover_set_child(popover as *mut GtkPopover, menu);

        let mut three_by_two_section_started = false;

        for (layout_index, layout) in PLAYER_LAYOUTS.iter().enumerate() {
            if layout.is_three_by_two() && !three_by_two_section_started {
                let divider = gtk_separator_new(GTK_ORIENTATION_HORIZONTAL);
                gtk_widget_add_css_class(divider, c"stream-settings-divider".as_ptr());
                gtk_box_append(menu as *mut GtkBox, divider);
                three_by_two_section_started = true;
            }

            let button = gtk_button_new();
            let content = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
            let label_text =
                CString::new(layout.name).expect("layout names cannot contain NUL bytes");
            let label = gtk_label_new(label_text.as_ptr());

            gtk_widget_add_css_class(button, c"stream-settings-item".as_ptr());
            gtk_widget_set_halign(button, GTK_ALIGN_FILL);
            gtk_widget_set_hexpand(button, TRUE);
            gtk_label_set_xalign(label as *mut GtkLabel, 0.0);
            gtk_widget_set_hexpand(label, TRUE);
            gtk_box_append(content as *mut GtkBox, player_layout_icon_new(layout));
            gtk_box_append(content as *mut GtkBox, label);
            gtk_button_set_child(button as *mut GtkButton, content);
            g_object_set_data(
                button as *mut GObject,
                c"layout-index".as_ptr(),
                (layout_index + 1) as *mut c_void,
            );
            connect_app_state_signal(
                button as *mut c_void,
                c"clicked".as_ptr(),
                on_layout_choice_clicked as *const c_void,
                state,
            );
            gtk_box_append(menu as *mut GtkBox, button);
        }

        connect_app_state_signal(
            popover as *mut c_void,
            c"closed".as_ptr(),
            on_layout_popover_closed as *const c_void,
            state,
        );

        popover
    }
}

pub(super) unsafe extern "C" fn on_settings_saved(
    _settings: *mut AppSettings,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;
        if app_state_is_closing(state) {
            return;
        }

        if !(*state).player_surface.is_null() {
            player_surface_set_settings((*state).player_surface, (*state).settings);
        }

        show_window_overlay(state);
    }
}

pub(super) unsafe fn show_settings_window(state: *mut AppState, initial_page: c_int) {
    unsafe {
        if app_state_is_closing(state) {
            return;
        }

        settings_window_show(
            (*state).window as *mut GtkWindow,
            (*state).settings,
            initial_page,
            Some(on_settings_saved),
            app_state_ref(state) as *mut c_void,
            Some(app_state_source_data_free),
        );
        show_window_overlay(state);
    }
}

pub(super) unsafe extern "C" fn on_settings_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let state = user_data as *mut AppState;
        if !app_state_is_closing(state) {
            show_settings_window(state, SETTINGS_WINDOW_PAGE_GENERAL);
        }
    }
}
