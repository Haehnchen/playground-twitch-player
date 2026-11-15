use super::ffi::GtkButton;
use super::{SettingsWindow, notify_settings_saved};
use crate::player::icons::player_trash_icon_new;
use crate::settings::{
    app_settings_add_channel, app_settings_clear_channels, app_settings_save,
    app_settings_set_hwdec_enabled, app_settings_set_twitch_playback_auth_token,
};
use crate::twitch::stream_info::GError;
use gtk4::{
    glib::{self, translate::*},
    prelude::*,
};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    ptr,
};

pub(super) fn is_valid_channel_name_bytes(channel: &[u8]) -> bool {
    channel
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
}

pub(super) unsafe fn update_empty_state(view: *mut SettingsWindow) {
    unsafe {
        let list: gtk4::Box = from_glib_none((*view).channels_box.cast());
        let empty: gtk4::Label = from_glib_none((*view).empty_label.cast());
        empty.set_visible(list.first_child().is_none());
    }
}

pub(super) unsafe fn add_channel_row(view: *mut SettingsWindow, channel: *const c_char) {
    unsafe {
        let list: gtk4::Box = from_glib_none((*view).channels_box.cast());
        let window: gtk4::Window = from_glib_none((*view).window.cast());
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        row.add_css_class("settings-channel-row");
        let entry = gtk4::Entry::builder()
            .placeholder_text("Twitch Channel")
            .hexpand(true)
            .build();
        if !channel.is_null() {
            entry.set_text(&CStr::from_ptr(channel).to_string_lossy());
        }
        row.append(&entry);
        let remove = gtk4::Button::builder()
            .has_frame(false)
            .tooltip_text("Remove")
            .build();
        let icon: gtk4::Widget = from_glib_none(player_trash_icon_new::<gtk4::ffi::GtkWidget>());
        remove.set_child(Some(&icon));
        remove.add_css_class("settings-remove-button");
        let weak_row = row.downgrade();
        let weak_list = list.downgrade();
        let weak_window = window.downgrade();
        remove.connect_clicked(move |_| {
            if let (Some(row), Some(list)) = (weak_row.upgrade(), weak_list.upgrade()) {
                list.remove(&row);
                update_empty_state(view);
                let weak_list = weak_list.clone();
                let weak_window = weak_window.clone();
                glib::idle_add_local_once(move || {
                    if let (Some(window), Some(list)) = (weak_window.upgrade(), weak_list.upgrade())
                    {
                        gtk4::prelude::GtkWindowExt::set_focus(&window, gtk4::Widget::NONE);
                        let mut child = list.first_child();
                        while let Some(row) = child {
                            if let Some(entry) = row.first_child().and_downcast::<gtk4::Entry>() {
                                entry.select_region(0, 0);
                            }
                            child = row.next_sibling();
                        }
                    }
                });
            }
        });
        row.append(&remove);
        list.append(&row);
        update_empty_state(view);
    }
}

pub(super) unsafe extern "C" fn on_add_channel_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        add_channel_row(user_data.cast(), c"".as_ptr());
    }
}

// Validate all entries before updating settings or notifying the player.
pub(super) unsafe extern "C" fn on_save_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    unsafe {
        let view = user_data as *mut SettingsWindow;
        let status: gtk4::Label = from_glib_none((*view).status_label.cast());
        let list: gtk4::Box = from_glib_none((*view).channels_box.cast());
        let window: gtk4::Window = from_glib_none((*view).window.cast());
        status.set_text("");
        let mut channels = Vec::new();
        let mut child = list.first_child();
        while let Some(row) = child {
            if let Some(entry) = row.first_child().and_downcast::<gtk4::Entry>() {
                let text = entry.text();
                let trimmed = text.trim_ascii();
                if !trimmed.is_empty() {
                    if !is_valid_channel_name_bytes(trimmed.as_bytes()) {
                        status.set_text("Invalid channel name. Use a-z, 0-9 and _ only.");
                        return;
                    }
                    channels.push(CString::new(trimmed).unwrap());
                }
            }
            child = row.next_sibling();
        }
        let hwdec: gtk4::CheckButton = from_glib_none((*view).hwdec_check.cast());
        app_settings_set_hwdec_enabled((*view).settings, hwdec.is_active().into());
        let token: gtk4::Entry = from_glib_none((*view).twitch_playback_auth_entry.cast());
        let token = CString::new(token.text().as_str()).unwrap();
        app_settings_set_twitch_playback_auth_token((*view).settings, token.as_ptr());
        app_settings_clear_channels((*view).settings);
        for channel in channels {
            app_settings_add_channel((*view).settings, ptr::null(), channel.as_ptr(), ptr::null());
        }
        let mut error: *mut GError = ptr::null_mut();
        if app_settings_save((*view).settings, &mut error) == 0 {
            let error: glib::Error = from_glib_full(error.cast());
            status.set_text(error.message());
            return;
        }
        notify_settings_saved(view);
        window.close();
    }
}
