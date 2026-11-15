use super::chat::{get_default_chat_width, on_chat_paned_layout_changed, on_tile_chat_clicked};
use super::ffi::{
    G_SOURCE_REMOVE, GDK_BUTTON_PRIMARY, GDK_BUTTON_SECONDARY, GTK_ALIGN_CENTER, GTK_ALIGN_END,
    GTK_ALIGN_FILL, GTK_ALIGN_START, GTK_EVENT_CONTROLLER_SCROLL_VERTICAL,
    GTK_ORIENTATION_HORIZONTAL, GTK_ORIENTATION_VERTICAL, GTK_PHASE_CAPTURE, GtkBox, GtkButton,
    GtkGLArea, GtkGestureSingle, GtkLabel, GtkOverlay, GtkPaned, GtkPopover, GtkRange, GtkScale,
    GtkWidget, PANGO_ELLIPSIZE_END, gtk_box_append, gtk_box_new, gtk_button_new,
    gtk_button_set_child, gtk_event_controller_legacy_new, gtk_event_controller_motion_new,
    gtk_event_controller_scroll_new, gtk_event_controller_set_propagation_phase,
    gtk_gesture_click_new, gtk_gesture_single_set_button, gtk_gl_area_new, gtk_label_new,
    gtk_label_set_ellipsize, gtk_label_set_xalign, gtk_overlay_add_overlay, gtk_overlay_new,
    gtk_overlay_set_child, gtk_paned_new, gtk_paned_set_resize_end_child,
    gtk_paned_set_resize_start_child, gtk_paned_set_shrink_end_child,
    gtk_paned_set_shrink_start_child, gtk_paned_set_start_child, gtk_paned_set_wide_handle,
    gtk_popover_popdown, gtk_range_set_value, gtk_scale_new_with_range, gtk_scale_set_draw_value,
    gtk_widget_add_controller, gtk_widget_add_css_class, gtk_widget_get_visible,
    gtk_widget_set_can_target, gtk_widget_set_halign, gtk_widget_set_hexpand,
    gtk_widget_set_margin_end, gtk_widget_set_size_request, gtk_widget_set_tooltip_text,
    gtk_widget_set_valign, gtk_widget_set_vexpand, gtk_widget_set_visible,
};
use super::input::{
    on_context_pressed, on_tile_motion, on_tile_scroll, on_video_legacy_event, on_video_pressed,
};
use super::layout::on_tile_focus_clicked;
use super::playback::{
    dup_twitch_channel_name, on_channel_button_clicked, on_channel_refresh_clicked,
    on_empty_tile_clicked, on_tile_close_clicked, sync_tile_from_session, target_to_label,
    update_tile_empty_state,
};
use super::render::{configure_gl_area, on_gl_realize, on_gl_render, on_gl_unrealize};
use super::runtime::{
    add_surface_timeout, add_weak_pointer, connect_tile_signal, remove_source_if_active,
    tile_is_closing,
};
use super::stream_details::{create_tile_twitch_playback_panel, on_tile_twitch_playback_clicked};
use super::streams::on_tile_stream_settings_clicked;
use super::{
    FALSE, MAX_TILES, PLAYER_CHAT_ICON_OPEN, PLAYER_TILE_FOCUS_ICON_EXPAND, PlayerSurface,
    StreamTile, TRUE,
};
use crate::glib_strings::is_nonempty;
use crate::player::channel_switcher_overlay::channel_switcher_overlay_is_visible;
use crate::player::controls::{
    player_footer_stream_info_get_widget, player_footer_stream_info_new, player_overlay_button_new,
    player_volume_mute_button_new, player_volume_set_muted, player_volume_sync_session_from_range,
    player_volume_toggle_muted,
};
use crate::player::icons::{
    player_chat_icon_new, player_plus_icon_new, player_refresh_icon_new,
    player_stream_settings_icon_new, player_tile_focus_icon_new, player_trash_icon_new,
};
use crate::player::session::{
    player_session_get_muted, player_session_get_volume, player_session_new,
    player_session_set_hwdec_enabled, player_session_toggle_stream_info,
};
use crate::player::stream_settings::player_stream_settings_popover_new;
use crate::settings::app_settings_get_hwdec_enabled;
use gtk4::prelude::*;
use std::ffi::{c_char, c_double, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

pub(super) const STREAM_DROPDOWN_WIDTH: c_int = 140;

pub(super) const PLAYER_VOLUME_SCALE_WIDTH: c_int = 96;

pub(super) const PLAYER_VOLUME_MIN: c_double = 0.0;

pub(super) const PLAYER_VOLUME_MAX: c_double = 130.0;

pub(super) unsafe fn tile_settings_popover_is_visible(tile: *mut StreamTile) -> bool {
    unsafe {
        !(*tile).stream_settings_popover.is_null()
            && gtk_widget_get_visible((*tile).stream_settings_popover) != 0
    }
}

pub(super) unsafe extern "C" fn on_volume_changed(range: *mut GtkRange, user_data: *mut c_void) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        player_volume_sync_session_from_range((*tile).session, range);
        if player_session_get_muted((*tile).session) != 0 {
            player_volume_set_muted((*tile).session, (*tile).mute_button, FALSE);
        }
    }
}

pub(super) unsafe extern "C" fn on_mute_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        if !is_nonempty((*tile).channel) {
            return;
        }

        player_volume_toggle_muted((*tile).session, (*tile).mute_button);
        show_tile_overlay(tile);
    }
}

pub(super) unsafe extern "C" fn on_tile_stream_info_toggle_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        player_session_toggle_stream_info((*tile).session);
        if !(*tile).stream_settings_popover.is_null() {
            gtk_popover_popdown((*tile).stream_settings_popover as *mut GtkPopover);
        }
        show_tile_overlay(tile);
    }
}

pub(super) unsafe fn is_channel_menu_open(tile: *mut StreamTile) -> bool {
    unsafe { channel_switcher_overlay_is_visible((*tile).channel_switcher) != 0 }
}

pub(super) unsafe extern "C" fn hide_footers(user_data: *mut c_void) -> c_int {
    unsafe {
        let state = user_data as *mut PlayerSurface;

        (*state).footer_hide_source = 0;

        for i in 0..MAX_TILES {
            let tile = &mut (*state).tiles[i] as *mut StreamTile;
            if is_channel_menu_open(tile) || tile_settings_popover_is_visible(tile) {
                schedule_footer_hide(state);
                return G_SOURCE_REMOVE;
            }
        }

        (*state).visible_footer_tile = ptr::null_mut();

        if !(*state).closing.load(Ordering::Acquire) {
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
}

pub(super) unsafe fn schedule_footer_hide(state: *mut PlayerSurface) {
    unsafe {
        remove_source_if_active(&mut (*state).footer_hide_source);
        (*state).footer_hide_source = add_surface_timeout(1800, Some(hide_footers), state);
    }
}

pub(super) unsafe fn show_tile_overlay(tile: *mut StreamTile) {
    unsafe {
        let state = (*tile).app;

        if (*state).closing.load(Ordering::Acquire) {
            return;
        }

        if !(*state).top_controls.is_null() {
            gtk_widget_set_visible((*state).top_controls, TRUE);
        }
        for i in 0..MAX_TILES {
            if !(*state).tiles[i].footer.is_null() {
                let visible = std::ptr::eq(&(*state).tiles[i], tile) as c_int;
                gtk_widget_set_visible((*state).tiles[i].footer, visible);
            }
        }
        (*state).visible_footer_tile = tile;

        schedule_footer_hide(state);
    }
}

pub(super) unsafe fn create_tile_footer(tile: *mut StreamTile) -> *mut GtkWidget {
    unsafe {
        let box_ = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 4);
        gtk_widget_add_css_class(box_, c"player-footer".as_ptr());
        gtk_widget_add_css_class(box_, c"tile-footer".as_ptr());
        gtk_widget_add_css_class(box_, c"video-footer".as_ptr());

        let channel_selector = gtk_overlay_new();
        gtk_widget_add_css_class(channel_selector, c"channel-selector".as_ptr());
        gtk_widget_set_size_request(channel_selector, STREAM_DROPDOWN_WIDTH, -1);
        gtk_widget_set_hexpand(channel_selector, FALSE);

        (*tile).channel_combo = gtk_button_new();
        gtk_widget_add_css_class((*tile).channel_combo, c"channel-dropdown".as_ptr());
        (*tile).channel_label = gtk_label_new(c"".as_ptr());
        gtk_widget_add_css_class((*tile).channel_label, c"channel-button-label".as_ptr());
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
        connect_tile_signal(
            (*tile).channel_combo as *mut c_void,
            c"clicked".as_ptr(),
            on_channel_button_clicked as *const c_void,
            tile,
        );

        gtk_overlay_set_child(channel_selector as *mut GtkOverlay, (*tile).channel_combo);

        (*tile).channel_refresh_button =
            player_overlay_button_new(player_refresh_icon_new(), c"Resync video".as_ptr());
        gtk_widget_add_css_class(
            (*tile).channel_refresh_button,
            c"channel-refresh-button".as_ptr(),
        );
        gtk_widget_add_css_class(
            (*tile).channel_refresh_button,
            c"player-refresh-button".as_ptr(),
        );
        gtk_widget_set_halign((*tile).channel_refresh_button, GTK_ALIGN_END);
        gtk_widget_set_valign((*tile).channel_refresh_button, GTK_ALIGN_CENTER);
        gtk_widget_set_margin_end((*tile).channel_refresh_button, 3);
        gtk_overlay_add_overlay(
            channel_selector as *mut GtkOverlay,
            (*tile).channel_refresh_button,
        );
        connect_tile_signal(
            (*tile).channel_refresh_button as *mut c_void,
            c"clicked".as_ptr(),
            on_channel_refresh_clicked as *const c_void,
            tile,
        );

        (*tile).close_button =
            player_overlay_button_new(player_trash_icon_new(), c"Clear slot".as_ptr());
        gtk_widget_add_css_class((*tile).close_button, c"tile-close-button".as_ptr());
        connect_tile_signal(
            (*tile).close_button as *mut c_void,
            c"clicked".as_ptr(),
            on_tile_close_clicked as *const c_void,
            tile,
        );

        (*tile).stream_info = player_footer_stream_info_new();

        (*tile).volume_scale = gtk_scale_new_with_range(
            GTK_ORIENTATION_HORIZONTAL,
            PLAYER_VOLUME_MIN,
            PLAYER_VOLUME_MAX,
            1.0,
        );
        gtk_widget_add_css_class((*tile).volume_scale, c"volume-scale".as_ptr());
        gtk_range_set_value(
            (*tile).volume_scale as *mut GtkRange,
            player_session_get_volume((*tile).session),
        );
        gtk_scale_set_draw_value((*tile).volume_scale as *mut GtkScale, FALSE);
        gtk_widget_set_size_request((*tile).volume_scale, PLAYER_VOLUME_SCALE_WIDTH, -1);
        connect_tile_signal(
            (*tile).volume_scale as *mut c_void,
            c"value-changed".as_ptr(),
            on_volume_changed as *const c_void,
            tile,
        );

        (*tile).mute_button = player_volume_mute_button_new((*tile).session);
        connect_tile_signal(
            (*tile).mute_button as *mut c_void,
            c"clicked".as_ptr(),
            on_mute_clicked as *const c_void,
            tile,
        );

        (*tile).focus_button = player_overlay_button_new(
            player_tile_focus_icon_new(PLAYER_TILE_FOCUS_ICON_EXPAND),
            c"Focus tile".as_ptr(),
        );
        connect_tile_signal(
            (*tile).focus_button as *mut c_void,
            c"clicked".as_ptr(),
            on_tile_focus_clicked as *const c_void,
            tile,
        );

        (*tile).chat_toggle_button = player_overlay_button_new(
            player_chat_icon_new(PLAYER_CHAT_ICON_OPEN),
            c"Open chat".as_ptr(),
        );
        gtk_widget_add_css_class((*tile).chat_toggle_button, c"chat-toggle".as_ptr());
        connect_tile_signal(
            (*tile).chat_toggle_button as *mut c_void,
            c"clicked".as_ptr(),
            on_tile_chat_clicked as *const c_void,
            tile,
        );

        (*tile).stream_info_button = player_overlay_button_new(
            player_stream_settings_icon_new(),
            c"Stream settings".as_ptr(),
        );
        gtk_widget_add_css_class(
            (*tile).stream_info_button,
            c"stream-settings-button".as_ptr(),
        );
        connect_tile_signal(
            (*tile).stream_info_button as *mut c_void,
            c"clicked".as_ptr(),
            on_tile_stream_settings_clicked as *const c_void,
            tile,
        );

        let mut twitch_playback_button: *mut GtkWidget = ptr::null_mut();
        let mut info_button: *mut GtkWidget = ptr::null_mut();
        (*tile).stream_settings_popover = player_stream_settings_popover_new(
            (*tile).stream_info_button,
            &mut (*tile).quality_list_box,
            &mut (*tile).quality_status_label,
            &mut twitch_playback_button,
            &mut info_button,
        );
        connect_tile_signal(
            twitch_playback_button as *mut c_void,
            c"clicked".as_ptr(),
            on_tile_twitch_playback_clicked as *const c_void,
            tile,
        );
        connect_tile_signal(
            info_button as *mut c_void,
            c"clicked".as_ptr(),
            on_tile_stream_info_toggle_clicked as *const c_void,
            tile,
        );

        gtk_box_append(box_ as *mut GtkBox, channel_selector);
        gtk_box_append(box_ as *mut GtkBox, (*tile).close_button);
        gtk_box_append(
            box_ as *mut GtkBox,
            player_footer_stream_info_get_widget((*tile).stream_info),
        );
        gtk_box_append(box_ as *mut GtkBox, (*tile).mute_button);
        gtk_box_append(box_ as *mut GtkBox, (*tile).volume_scale);
        gtk_box_append(box_ as *mut GtkBox, (*tile).chat_toggle_button);
        gtk_box_append(box_ as *mut GtkBox, (*tile).stream_info_button);
        update_tile_empty_state(tile);

        box_
    }
}

pub(super) unsafe fn create_stream_tile(
    state: *mut PlayerSurface,
    index: c_uint,
    target: *const c_char,
) -> *mut GtkWidget {
    unsafe {
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

        // The outer overlay keeps separators inside the tile without affecting grid allocation.
        (*tile).frame = gtk_overlay_new();
        add_weak_pointer((*tile).frame, &mut (*tile).frame);
        gtk_widget_add_css_class((*tile).frame, c"tile-frame".as_ptr());
        gtk_widget_set_hexpand((*tile).frame, TRUE);
        gtk_widget_set_vexpand((*tile).frame, TRUE);

        // A tile is its own video/chat panel: start child is video, end child is optional chat.
        (*tile).container = gtk_paned_new(GTK_ORIENTATION_HORIZONTAL);
        add_weak_pointer((*tile).container, &mut (*tile).container);
        gtk_widget_add_css_class((*tile).container, c"tile-container".as_ptr());
        gtk_widget_set_hexpand((*tile).container, TRUE);
        gtk_widget_set_vexpand((*tile).container, TRUE);
        gtk_paned_set_wide_handle((*tile).container as *mut GtkPaned, FALSE);
        gtk_paned_set_resize_start_child((*tile).container as *mut GtkPaned, TRUE);
        gtk_paned_set_shrink_start_child((*tile).container as *mut GtkPaned, FALSE);
        gtk_paned_set_resize_end_child((*tile).container as *mut GtkPaned, FALSE);
        gtk_paned_set_shrink_end_child((*tile).container as *mut GtkPaned, TRUE);
        gtk_overlay_set_child((*tile).frame as *mut GtkOverlay, (*tile).container);

        let right_separator = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
        gtk_widget_add_css_class(right_separator, c"tile-separator".as_ptr());
        gtk_widget_add_css_class(right_separator, c"tile-separator-right".as_ptr());
        gtk_widget_set_can_target(right_separator, FALSE);
        gtk_widget_set_halign(right_separator, GTK_ALIGN_END);
        gtk_widget_set_valign(right_separator, GTK_ALIGN_FILL);
        gtk_widget_set_size_request(right_separator, 1, -1);
        gtk_overlay_add_overlay((*tile).frame as *mut GtkOverlay, right_separator);

        let bottom_separator = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
        gtk_widget_add_css_class(bottom_separator, c"tile-separator".as_ptr());
        gtk_widget_add_css_class(bottom_separator, c"tile-separator-bottom".as_ptr());
        gtk_widget_set_can_target(bottom_separator, FALSE);
        gtk_widget_set_halign(bottom_separator, GTK_ALIGN_FILL);
        gtk_widget_set_valign(bottom_separator, GTK_ALIGN_END);
        gtk_widget_set_size_request(bottom_separator, -1, 1);
        gtk_overlay_add_overlay((*tile).frame as *mut GtkOverlay, bottom_separator);
        connect_tile_signal(
            (*tile).container as *mut c_void,
            c"notify::position".as_ptr(),
            on_chat_paned_layout_changed as *const c_void,
            tile,
        );
        connect_tile_signal(
            (*tile).container as *mut c_void,
            c"notify::max-position".as_ptr(),
            on_chat_paned_layout_changed as *const c_void,
            tile,
        );

        (*tile).overlay = gtk_overlay_new();
        add_weak_pointer((*tile).overlay, &mut (*tile).overlay);
        gtk_widget_set_hexpand((*tile).overlay, TRUE);
        gtk_widget_set_vexpand((*tile).overlay, TRUE);
        gtk_paned_set_start_child((*tile).container as *mut GtkPaned, (*tile).overlay);
        (*tile).chat_width = get_default_chat_width();

        (*tile).gl_area = gtk_gl_area_new();
        add_weak_pointer((*tile).gl_area, &mut (*tile).gl_area);
        configure_gl_area((*tile).gl_area as *mut GtkGLArea);
        gtk_widget_set_hexpand((*tile).gl_area, TRUE);
        gtk_widget_set_vexpand((*tile).gl_area, TRUE);
        gtk_overlay_set_child((*tile).overlay as *mut GtkOverlay, (*tile).gl_area);

        (*tile).empty_label = gtk_button_new();
        let empty_icon_frame = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
        gtk_widget_add_css_class(empty_icon_frame, c"empty-stream-button-visible".as_ptr());
        gtk_widget_set_halign(empty_icon_frame, GTK_ALIGN_CENTER);
        gtk_widget_set_valign(empty_icon_frame, GTK_ALIGN_CENTER);
        gtk_box_append(empty_icon_frame as *mut GtkBox, player_plus_icon_new());
        gtk_button_set_child((*tile).empty_label as *mut GtkButton, empty_icon_frame);
        gtk_widget_add_css_class((*tile).empty_label, c"empty-stream-button".as_ptr());
        gtk_widget_set_tooltip_text((*tile).empty_label, c"Select channel".as_ptr());
        gtk_widget_set_halign((*tile).empty_label, GTK_ALIGN_CENTER);
        gtk_widget_set_valign((*tile).empty_label, GTK_ALIGN_CENTER);
        connect_tile_signal(
            (*tile).empty_label as *mut c_void,
            c"clicked".as_ptr(),
            on_empty_tile_clicked as *const c_void,
            tile,
        );
        gtk_overlay_add_overlay((*tile).overlay as *mut GtkOverlay, (*tile).empty_label);

        let status = gtk4::Label::builder()
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .can_target(false)
            .visible(false)
            .css_classes(["stream-status"])
            .build();
        gtk_overlay_add_overlay((*tile).overlay as *mut GtkOverlay, status.as_ptr().cast());
        (*tile).status_label = Some(status);

        (*tile).footer = create_tile_footer(tile);
        gtk_widget_set_halign((*tile).footer, GTK_ALIGN_FILL);
        gtk_widget_set_valign((*tile).footer, GTK_ALIGN_END);
        gtk_widget_set_visible((*tile).footer, FALSE);
        gtk_overlay_add_overlay((*tile).overlay as *mut GtkOverlay, (*tile).footer);
        (*tile).twitch_playback_panel = create_tile_twitch_playback_panel(tile);
        gtk_overlay_add_overlay(
            (*tile).overlay as *mut GtkOverlay,
            (*tile).twitch_playback_panel,
        );

        let video_click = gtk_gesture_click_new();
        gtk_gesture_single_set_button(video_click as *mut GtkGestureSingle, GDK_BUTTON_PRIMARY);
        connect_tile_signal(
            video_click as *mut c_void,
            c"pressed".as_ptr(),
            on_video_pressed as *const c_void,
            tile,
        );
        gtk_widget_add_controller((*tile).gl_area, video_click.cast());

        let context_click = gtk_gesture_click_new();
        gtk_gesture_single_set_button(context_click as *mut GtkGestureSingle, GDK_BUTTON_SECONDARY);
        connect_tile_signal(
            context_click as *mut c_void,
            c"pressed".as_ptr(),
            on_context_pressed as *const c_void,
            tile,
        );
        gtk_widget_add_controller((*tile).overlay, context_click.cast());

        let video_legacy = gtk_event_controller_legacy_new();
        connect_tile_signal(
            video_legacy as *mut c_void,
            c"event".as_ptr(),
            on_video_legacy_event as *const c_void,
            tile,
        );
        gtk_widget_add_controller((*tile).gl_area, video_legacy.cast());

        let video_motion = gtk_event_controller_motion_new();
        gtk_event_controller_set_propagation_phase(video_motion, GTK_PHASE_CAPTURE);
        connect_tile_signal(
            video_motion as *mut c_void,
            c"motion".as_ptr(),
            on_tile_motion as *const c_void,
            tile,
        );
        gtk_widget_add_controller((*tile).overlay, video_motion.cast());

        let tile_scroll = gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_VERTICAL);
        gtk_event_controller_set_propagation_phase(tile_scroll, GTK_PHASE_CAPTURE);
        connect_tile_signal(
            tile_scroll as *mut c_void,
            c"scroll".as_ptr(),
            on_tile_scroll as *const c_void,
            tile,
        );
        gtk_widget_add_controller((*tile).overlay, tile_scroll.cast());

        connect_tile_signal(
            (*tile).gl_area as *mut c_void,
            c"realize".as_ptr(),
            on_gl_realize as *const c_void,
            tile,
        );
        connect_tile_signal(
            (*tile).gl_area as *mut c_void,
            c"unrealize".as_ptr(),
            on_gl_unrealize as *const c_void,
            tile,
        );
        connect_tile_signal(
            (*tile).gl_area as *mut c_void,
            c"render".as_ptr(),
            on_gl_render as *const c_void,
            tile,
        );

        update_tile_empty_state(tile);

        (*tile).frame
    }
}
