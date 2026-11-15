use super::controls::show_tile_overlay;
use super::ffi::{
    GTK_ALIGN_CENTER, GTK_ALIGN_FILL, GTK_ORIENTATION_HORIZONTAL, GTK_ORIENTATION_VERTICAL, GtkBox,
    GtkButton, GtkLabel, GtkPopover, GtkWidget, gtk_box_append, gtk_box_new, gtk_button_new,
    gtk_button_set_child, gtk_label_new, gtk_label_set_text, gtk_label_set_wrap,
    gtk_label_set_xalign, gtk_popover_popdown, gtk_widget_add_css_class, gtk_widget_set_halign,
    gtk_widget_set_hexpand, gtk_widget_set_tooltip_text, gtk_widget_set_valign,
    gtk_widget_set_vexpand, gtk_widget_set_visible,
};
use super::runtime::{connect_tile_signal, tile_is_closing};
use super::streams::request_tile_qualities_update;
use super::{FALSE, StreamTile, TRUE};
use crate::glib_strings::is_nonempty;
use crate::player::icons::{player_stream_settings_icon_new, player_window_icon_new};
use crate::settings::app_settings_get_twitch_playback_auth_token;
use crate::twitch::stream_info::TwitchPlaybackInfo;
use std::ffi::{c_char, c_int, c_void};
pub(super) const PLAYER_WINDOW_ICON_CLOSE: c_int = 2;

pub(super) unsafe fn set_tile_twitch_playback_text(tile: *mut StreamTile, text: *const c_char) {
    unsafe {
        if !(*tile).twitch_playback_status_label.is_null() {
            gtk_label_set_text((*tile).twitch_playback_status_label as *mut GtkLabel, text);
        }
    }
}

pub(super) unsafe fn set_tile_twitch_playback_not_checked(tile: *mut StreamTile) {
    unsafe {
        set_tile_twitch_playback_text(
        tile,
        c"Connection: Not checked\nUser ID: -\nSubscription claim: -\nTurbo: -\nShow ads: -\nHide ads: -\nServer-side ads: -\nAd-free playback: -".as_ptr(),
    );
    }
}

pub(super) unsafe fn set_tile_twitch_playback_loading(tile: *mut StreamTile) {
    unsafe {
        if is_nonempty(app_settings_get_twitch_playback_auth_token(
            (*(*tile).app).settings,
        )) {
            set_tile_twitch_playback_text(
            tile,
            c"Connection: Checking token...\nUser ID: -\nSubscription claim: -\nTurbo: -\nShow ads: -\nHide ads: -\nServer-side ads: -\nAd-free playback: Checking...".as_ptr(),
        );
        } else {
            set_tile_twitch_playback_text(
            tile,
            c"Connection: Anonymous\nUser ID: -\nSubscription claim: -\nTurbo: -\nShow ads: -\nHide ads: -\nServer-side ads: -\nAd-free playback: Checking...".as_ptr(),
        );
        }
    }
}

pub(super) unsafe fn set_tile_twitch_playback_unavailable(tile: *mut StreamTile) {
    unsafe {
        set_tile_twitch_playback_text(
        tile,
        c"Connection: Unavailable\nUser ID: -\nSubscription claim: -\nTurbo: -\nShow ads: -\nHide ads: -\nServer-side ads: -\nAd-free playback: Anonymous fallback".as_ptr(),
    );
    }
}

pub(super) unsafe fn set_tile_twitch_playback_info(
    tile: *mut StreamTile,
    info: &TwitchPlaybackInfo,
) {
    unsafe {
        if info.available == 0 {
            set_tile_twitch_playback_unavailable(tile);
            return;
        }

        let token_configured = is_nonempty(app_settings_get_twitch_playback_auth_token(
            (*(*tile).app).settings,
        ));
        let connection = if info.authenticated != 0 {
            "Authenticated"
        } else if token_configured {
            "Token not recognized"
        } else {
            "Anonymous"
        };
        if info.authenticated != 0 {
            set_tile_twitch_playback_authenticated_info(tile, info, connection);
            return;
        }
        let mut text = format!(
        "Connection: {connection}\nUser ID: -\nSubscription claim: -\nTurbo: -\nShow ads: {}\nHide ads: {}\nServer-side ads: {}\nAd-free playback: {}",
        yes_no(info.show_ads),
        yes_no(info.hide_ads),
        yes_no(info.server_ads),
        ad_free_playback_status(info),
    )
    .into_bytes();
        text.push(0);
        set_tile_twitch_playback_text(tile, text.as_ptr() as *const c_char);
    }
}

pub(super) fn yes_no(value: c_int) -> &'static str {
    if value != 0 { "Yes" } else { "No" }
}

pub(super) fn ad_free_playback_status(info: &TwitchPlaybackInfo) -> &'static str {
    if info.hide_ads != 0 || info.show_ads == 0 {
        "Granted"
    } else {
        "Not granted"
    }
}

pub(super) unsafe fn set_tile_twitch_playback_authenticated_info(
    tile: *mut StreamTile,
    info: &TwitchPlaybackInfo,
    connection: &str,
) {
    unsafe {
        let subscription = if info.subscriber != 0 { "Yes" } else { "No" };
        let turbo = if info.turbo != 0 {
            "Active"
        } else {
            "Not active"
        };
        let mut text = format!(
        "Connection: {connection}\nUser ID: {}\nSubscription claim: {subscription}\nTurbo: {turbo}\nShow ads: {}\nHide ads: {}\nServer-side ads: {}\nAd-free playback: {}",
        info.user_id,
        yes_no(info.show_ads),
        yes_no(info.hide_ads),
        yes_no(info.server_ads),
        ad_free_playback_status(info),
    )
    .into_bytes();
        text.push(0);
        set_tile_twitch_playback_text(tile, text.as_ptr() as *const c_char);
    }
}

pub(super) unsafe fn hide_tile_twitch_playback_panel(tile: *mut StreamTile) {
    unsafe {
        if !(*tile).twitch_playback_panel.is_null() {
            gtk_widget_set_visible((*tile).twitch_playback_panel, FALSE);
        }
    }
}

pub(super) unsafe extern "C" fn on_tile_twitch_playback_close_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if !tile_is_closing(tile) {
            hide_tile_twitch_playback_panel(tile);
        }
    }
}

pub(super) unsafe extern "C" fn on_tile_twitch_playback_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) || (*tile).twitch_playback_panel.is_null() {
            return;
        }
        if !(*tile).stream_settings_popover.is_null() {
            gtk_popover_popdown((*tile).stream_settings_popover as *mut GtkPopover);
        }
        gtk_widget_set_visible((*tile).twitch_playback_panel, TRUE);
        request_tile_qualities_update(tile, TRUE);
        show_tile_overlay(tile);
    }
}

pub(super) unsafe extern "C" fn on_tile_twitch_playback_settings_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        let state = (*tile).app;

        hide_tile_twitch_playback_panel(tile);
        if !(*tile).stream_settings_popover.is_null() {
            gtk_popover_popdown((*tile).stream_settings_popover as *mut GtkPopover);
        }
        if let Some(callback) = (*state).settings_callback {
            callback((*state).settings_user_data);
        }
    }
}

pub(super) unsafe fn create_tile_twitch_playback_panel(tile: *mut StreamTile) -> *mut GtkWidget {
    unsafe {
        let panel = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
        gtk_widget_add_css_class(panel, c"twitch-playback-panel".as_ptr());
        gtk_widget_set_halign(panel, GTK_ALIGN_CENTER);
        gtk_widget_set_valign(panel, GTK_ALIGN_CENTER);
        gtk_widget_set_hexpand(panel, FALSE);
        gtk_widget_set_vexpand(panel, FALSE);
        gtk_widget_set_visible(panel, FALSE);

        let header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
        gtk_widget_add_css_class(header, c"twitch-playback-header".as_ptr());
        gtk_widget_set_halign(header, GTK_ALIGN_FILL);
        gtk_widget_set_hexpand(header, TRUE);

        let title = gtk_label_new(c"Twitch Playback".as_ptr());
        gtk_widget_add_css_class(title, c"twitch-playback-title".as_ptr());
        gtk_label_set_xalign(title as *mut GtkLabel, 0.0);
        gtk_widget_set_halign(title, GTK_ALIGN_FILL);
        gtk_widget_set_hexpand(title, TRUE);

        let settings_button = gtk_button_new();
        gtk_button_set_child(
            settings_button as *mut GtkButton,
            player_stream_settings_icon_new(),
        );
        gtk_widget_add_css_class(settings_button, c"twitch-playback-settings".as_ptr());
        gtk_widget_set_tooltip_text(
            settings_button,
            c"Playback authentication settings".as_ptr(),
        );
        connect_tile_signal(
            settings_button as *mut c_void,
            c"clicked".as_ptr(),
            on_tile_twitch_playback_settings_clicked as *const c_void,
            tile,
        );

        let close_button = gtk_button_new();
        gtk_button_set_child(
            close_button as *mut GtkButton,
            player_window_icon_new(PLAYER_WINDOW_ICON_CLOSE),
        );
        gtk_widget_add_css_class(close_button, c"twitch-playback-close".as_ptr());
        gtk_widget_set_tooltip_text(close_button, c"Close".as_ptr());
        connect_tile_signal(
            close_button as *mut c_void,
            c"clicked".as_ptr(),
            on_tile_twitch_playback_close_clicked as *const c_void,
            tile,
        );

        gtk_box_append(header as *mut GtkBox, title);
        gtk_box_append(header as *mut GtkBox, settings_button);
        gtk_box_append(header as *mut GtkBox, close_button);
        gtk_box_append(panel as *mut GtkBox, header);

        (*tile).twitch_playback_status_label =
            gtk_label_new(c"Connection: Not checked\nSubscription: -\nTurbo: -\nAds: -".as_ptr());
        gtk_widget_add_css_class(
            (*tile).twitch_playback_status_label,
            c"twitch-playback-status".as_ptr(),
        );
        gtk_label_set_xalign((*tile).twitch_playback_status_label as *mut GtkLabel, 0.0);
        gtk_label_set_wrap((*tile).twitch_playback_status_label as *mut GtkLabel, TRUE);
        gtk_widget_set_halign((*tile).twitch_playback_status_label, GTK_ALIGN_FILL);
        gtk_widget_set_hexpand((*tile).twitch_playback_status_label, TRUE);
        gtk_box_append(panel as *mut GtkBox, (*tile).twitch_playback_status_label);

        panel
    }
}
