use super::ffi::{
    G_PRIORITY_DEFAULT_IDLE, G_SOURCE_REMOVE, GObject, GTK_ORIENTATION_HORIZONTAL, GtkBox,
    GtkButton, GtkCheckButton, GtkEditable, GtkEntry, GtkLabel, GtkWidget, GtkWindow,
    g_clear_error, g_free, g_idle_add_full, g_object_get_data, g_object_ref,
    g_object_set_data_full, g_signal_connect_data, gtk_box_append, gtk_box_new, gtk_box_remove,
    gtk_button_new, gtk_button_set_child, gtk_button_set_has_frame, gtk_check_button_get_active,
    gtk_editable_get_text, gtk_editable_select_region, gtk_editable_set_text, gtk_entry_new,
    gtk_entry_set_placeholder_text, gtk_label_set_text, gtk_widget_add_css_class,
    gtk_widget_get_first_child, gtk_widget_get_next_sibling, gtk_widget_set_hexpand,
    gtk_widget_set_tooltip_text, gtk_widget_set_visible, gtk_window_close, gtk_window_set_focus,
};
use super::pages::label_text_or_empty;
use super::{ChannelRow, SettingsWindow, g_object_unref_destroy, notify_settings_saved};
use crate::glib_strings::dup_bytes;
use crate::player::icons::player_trash_icon_new;
use crate::settings::{
    app_settings_add_channel, app_settings_clear_channels, app_settings_save,
    app_settings_set_hwdec_enabled, app_settings_set_twitch_playback_auth_token,
};
use crate::twitch::stream_info::GError;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;

pub(super) fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|idx| idx + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

pub(super) fn is_valid_channel_name_bytes(channel: &[u8]) -> bool {
    channel
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
}

pub(super) unsafe extern "C" fn channel_row_free(data: *mut c_void) {
    unsafe {
        if !data.is_null() {
            drop(Box::from_raw(data as *mut ChannelRow));
        }
    }
}

pub(super) unsafe fn update_empty_state(view: *mut SettingsWindow) {
    unsafe {
        let has_rows = !gtk_widget_get_first_child((*view).channels_box).is_null();
        gtk_widget_set_visible((*view).empty_label, if has_rows { 0 } else { 1 });
    }
}

pub(super) unsafe extern "C" fn clear_channel_focus_after_remove(user_data: *mut c_void) -> c_int {
    unsafe {
        let window = user_data as *mut GtkWidget;
        let view = g_object_get_data(window as *mut GObject, c"settings-window".as_ptr())
            as *mut SettingsWindow;

        if view.is_null() {
            return G_SOURCE_REMOVE;
        }

        gtk_window_set_focus((*view).window as *mut GtkWindow, ptr::null_mut());
        let mut child = gtk_widget_get_first_child((*view).channels_box);
        while !child.is_null() {
            let row = g_object_get_data(child as *mut GObject, c"channel-row".as_ptr())
                as *mut ChannelRow;
            if !row.is_null() {
                gtk_editable_select_region((*row).channel_entry as *mut GtkEditable, 0, 0);
            }
            child = gtk_widget_get_next_sibling(child);
        }

        G_SOURCE_REMOVE
    }
}

pub(super) unsafe extern "C" fn on_remove_channel_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let row = user_data as *mut ChannelRow;
        let view = (*row).view;
        let row_widget = (*row).row;

        gtk_box_remove((*view).channels_box as *mut GtkBox, row_widget);
        update_empty_state(view);
        g_idle_add_full(
            G_PRIORITY_DEFAULT_IDLE,
            Some(clear_channel_focus_after_remove),
            g_object_ref((*view).window as *mut c_void),
            Some(g_object_unref_destroy),
        );
    }
}

pub(super) unsafe fn create_channel_row(
    view: *mut SettingsWindow,
    channel: *const c_char,
) -> *mut GtkWidget {
    unsafe {
        let row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
        gtk_widget_add_css_class(row, c"settings-channel-row".as_ptr());

        let row_data = Box::into_raw(Box::new(ChannelRow {
            view,
            row,
            channel_entry: ptr::null_mut(),
        }));
        g_object_set_data_full(
            row as *mut GObject,
            c"channel-row".as_ptr(),
            row_data as *mut c_void,
            Some(channel_row_free),
        );

        (*row_data).channel_entry = gtk_entry_new();
        gtk_entry_set_placeholder_text(
            (*row_data).channel_entry as *mut GtkEntry,
            c"Twitch Channel".as_ptr(),
        );
        gtk_editable_set_text(
            (*row_data).channel_entry as *mut GtkEditable,
            label_text_or_empty(channel),
        );
        gtk_widget_set_hexpand((*row_data).channel_entry, 1);
        gtk_box_append(row as *mut GtkBox, (*row_data).channel_entry);

        let remove_button = gtk_button_new();
        gtk_button_set_child(remove_button as *mut GtkButton, player_trash_icon_new());
        gtk_button_set_has_frame(remove_button as *mut GtkButton, 0);
        gtk_widget_add_css_class(remove_button, c"settings-remove-button".as_ptr());
        gtk_widget_set_tooltip_text(remove_button, c"Remove".as_ptr());
        gtk_box_append(row as *mut GtkBox, remove_button);
        g_signal_connect_data(
            remove_button as *mut c_void,
            c"clicked".as_ptr(),
            on_remove_channel_clicked as *const c_void,
            row_data as *mut c_void,
            ptr::null_mut(),
            0,
        );

        row
    }
}

pub(super) unsafe fn add_channel_row(view: *mut SettingsWindow, channel: *const c_char) {
    unsafe {
        gtk_box_append(
            (*view).channels_box as *mut GtkBox,
            create_channel_row(view, channel),
        );
        update_empty_state(view);
    }
}

pub(super) unsafe extern "C" fn on_add_channel_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        add_channel_row(user_data as *mut SettingsWindow, c"".as_ptr());
    }
}

// Validate channel entries before updating and saving settings.
pub(super) unsafe extern "C" fn on_save_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    unsafe {
        let view = user_data as *mut SettingsWindow;

        gtk_label_set_text((*view).status_label as *mut GtkLabel, c"".as_ptr());
        let mut channels: Vec<Vec<u8>> = Vec::new();
        let mut child = gtk_widget_get_first_child((*view).channels_box);
        while !child.is_null() {
            let row = g_object_get_data(child as *mut GObject, c"channel-row".as_ptr())
                as *mut ChannelRow;
            if !row.is_null() {
                let channel = gtk_editable_get_text((*row).channel_entry as *mut GtkEditable);
                let trimmed = trim_ascii(CStr::from_ptr(channel).to_bytes());
                if !trimmed.is_empty() {
                    if !is_valid_channel_name_bytes(trimmed) {
                        gtk_label_set_text(
                            (*view).status_label as *mut GtkLabel,
                            c"Invalid channel name. Use a-z, 0-9 and _ only.".as_ptr(),
                        );
                        return;
                    }
                    channels.push(trimmed.to_vec());
                }
            }
            child = gtk_widget_get_next_sibling(child);
        }

        app_settings_set_hwdec_enabled(
            (*view).settings,
            if (*view).hwdec_check.is_null()
                || gtk_check_button_get_active((*view).hwdec_check as *mut GtkCheckButton) != 0
            {
                1
            } else {
                0
            },
        );
        if !(*view).twitch_playback_auth_entry.is_null() {
            app_settings_set_twitch_playback_auth_token(
                (*view).settings,
                gtk_editable_get_text((*view).twitch_playback_auth_entry as *mut GtkEditable),
            );
        }
        app_settings_clear_channels((*view).settings);
        for channel in channels {
            let channel = dup_bytes(&channel);
            app_settings_add_channel((*view).settings, ptr::null(), channel, ptr::null());
            g_free(channel as *mut c_void);
        }

        let mut error: *mut GError = ptr::null_mut();
        if app_settings_save((*view).settings, &mut error) == 0 {
            gtk_label_set_text((*view).status_label as *mut GtkLabel, (*error).message);
            g_clear_error(&mut error);
            return;
        }

        notify_settings_saved(view);
        gtk_window_close((*view).window as *mut GtkWindow);
    }
}
