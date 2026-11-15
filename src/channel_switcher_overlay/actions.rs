//! Channel selection and search.

use super::cards::render_live_channels;
use super::ffi::*;
use super::runtime::{
    add_switcher_timeout, ptr_array_index, remove_source_if_active, switcher_is_closing,
};
use super::{ChannelSwitcherOverlay, channel_switcher_overlay_hide};
use crate::glib_strings::{dup_bytes, is_nonempty};
use crate::settings::{
    AppSettingsChannel, app_settings_get_channel, app_settings_get_channel_count,
};
use crate::twitch_channel::normalize_twitch_channel;
use crate::twitch_stream_info::TwitchStreamPreview;
use std::ffi::{CStr, c_char, c_double, c_int, c_uint, c_void};
use std::ptr;

const SEARCH_DEBOUNCE_MS: c_uint = 300;

pub(super) unsafe fn find_settings_channel(
    switcher: *mut ChannelSwitcherOverlay,
    channel_name: *const c_char,
) -> *const AppSettingsChannel {
    unsafe {
        let channel_count = app_settings_get_channel_count((*switcher).settings);

        for i in 0..channel_count {
            let channel = app_settings_get_channel((*switcher).settings, i);
            if !channel.is_null()
                && !(*channel).channel.is_null()
                && !channel_name.is_null()
                && g_ascii_strcasecmp((*channel).channel, channel_name) == 0
            {
                return channel;
            }
        }

        ptr::null()
    }
}

unsafe fn extract_twitch_channel_name(value: *const c_char) -> *mut c_char {
    unsafe {
        if value.is_null() {
            return ptr::null_mut();
        }

        normalize_twitch_channel(CStr::from_ptr(value).to_bytes())
            .map_or(ptr::null_mut(), |channel| dup_bytes(&channel))
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(super) unsafe extern "C" fn on_channel_button_clicked(
    button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let switcher = user_data as *mut ChannelSwitcherOverlay;
        if switcher_is_closing(switcher) {
            return;
        }

        let channel_name =
            g_object_get_data(button as *mut GObject, c"channel-name".as_ptr()) as *const c_char;
        let channel = find_settings_channel(switcher, channel_name);

        if !channel.is_null() {
            if let Some(callback) = (*switcher).activate_callback {
                callback(channel, (*switcher).user_data);
            }
        } else if is_nonempty(channel_name)
            && let Some(callback) = (*switcher).activate_callback
        {
            let mut dynamic_channel = AppSettingsChannel {
                channel: channel_name as *mut c_char,
                label: g_object_get_data(button as *mut GObject, c"channel-label".as_ptr())
                    as *mut c_char,
                url: g_strdup_printf(c"https://www.twitch.tv/%s".as_ptr(), channel_name),
            };
            if !is_nonempty(dynamic_channel.label) {
                dynamic_channel.label = dynamic_channel.channel;
            }
            callback(&dynamic_channel, (*switcher).user_data);
            g_free(dynamic_channel.url as *mut c_void);
        }

        channel_switcher_overlay_hide(switcher);
    }
}

unsafe fn activate_dynamic_channel(
    switcher: *mut ChannelSwitcherOverlay,
    channel_name: *const c_char,
    channel_label: *const c_char,
) {
    unsafe {
        if switcher_is_closing(switcher)
            || (*switcher).activate_callback.is_none()
            || !is_nonempty(channel_name)
        {
            return;
        }

        let configured_channel = find_settings_channel(switcher, channel_name);
        if !configured_channel.is_null() {
            if let Some(callback) = (*switcher).activate_callback {
                callback(configured_channel, (*switcher).user_data);
            }
            channel_switcher_overlay_hide(switcher);
            return;
        }

        let dynamic_channel = AppSettingsChannel {
            channel: channel_name as *mut c_char,
            label: if is_nonempty(channel_label) {
                channel_label as *mut c_char
            } else {
                channel_name as *mut c_char
            },
            url: g_strdup_printf(c"https://www.twitch.tv/%s".as_ptr(), channel_name),
        };

        if let Some(callback) = (*switcher).activate_callback {
            callback(&dynamic_channel, (*switcher).user_data);
        }
        g_free(dynamic_channel.url as *mut c_void);
        channel_switcher_overlay_hide(switcher);
    }
}

unsafe fn open_direct_channel(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if switcher_is_closing(switcher) || (*switcher).direct_channel_entry.is_null() {
            return;
        }

        let text = gtk_editable_get_text((*switcher).direct_channel_entry as *mut GtkEditable);
        let channel = extract_twitch_channel_name(text);
        activate_dynamic_channel(switcher, channel, channel);
        g_free(channel as *mut c_void);
    }
}

pub(super) unsafe extern "C" fn on_direct_channel_activate(
    _entry: *mut GtkEntry,
    user_data: *mut c_void,
) {
    unsafe {
        open_direct_channel(user_data as *mut ChannelSwitcherOverlay);
    }
}

pub(super) unsafe extern "C" fn on_direct_channel_icon_pressed(
    _entry: *mut GtkEntry,
    icon_pos: c_int,
    user_data: *mut c_void,
) {
    unsafe {
        if icon_pos == GTK_ENTRY_ICON_SECONDARY {
            open_direct_channel(user_data as *mut ChannelSwitcherOverlay);
        }
    }
}

unsafe fn string_contains_casefold(haystack: *const c_char, needle: *const c_char) -> bool {
    unsafe {
        if !is_nonempty(needle) {
            return true;
        }
        if !is_nonempty(haystack) {
            return false;
        }

        let haystack_folded = g_utf8_casefold(haystack, -1);
        let needle_folded = g_utf8_casefold(needle, -1);
        let found = if haystack_folded.is_null() || needle_folded.is_null() {
            false
        } else {
            let haystack_bytes = CStr::from_ptr(haystack_folded).to_bytes();
            let needle_bytes = CStr::from_ptr(needle_folded).to_bytes();
            find_bytes(haystack_bytes, needle_bytes).is_some()
        };
        g_free(haystack_folded as *mut c_void);
        g_free(needle_folded as *mut c_void);
        found
    }
}

pub(super) unsafe fn preview_matches_filter(
    preview: *mut TwitchStreamPreview,
    filter: *const c_char,
) -> bool {
    unsafe {
        string_contains_casefold((*preview).channel, filter)
            || string_contains_casefold((*preview).display_name, filter)
            || string_contains_casefold((*preview).title, filter)
            || string_contains_casefold((*preview).category_name, filter)
    }
}

pub(super) unsafe fn current_search_filter(switcher: *mut ChannelSwitcherOverlay) -> *const c_char {
    unsafe {
        if !(*switcher).search_entry.is_null() {
            gtk_editable_get_text((*switcher).search_entry as *mut GtkEditable)
        } else {
            c"".as_ptr()
        }
    }
}

unsafe extern "C" fn apply_search_filter(user_data: *mut c_void) -> c_int {
    unsafe {
        let switcher = user_data as *mut ChannelSwitcherOverlay;

        (*switcher).search_debounce_source = 0;
        if switcher_is_closing(switcher) {
            return G_SOURCE_REMOVE;
        }
        render_live_channels(switcher);

        G_SOURCE_REMOVE
    }
}

// Debounce filtering and stop batches for the previous query.
pub(super) unsafe extern "C" fn on_search_changed(
    _editable: *mut GtkEditable,
    user_data: *mut c_void,
) {
    unsafe {
        let switcher = user_data as *mut ChannelSwitcherOverlay;
        if switcher_is_closing(switcher) {
            return;
        }

        remove_source_if_active(&mut (*switcher).search_debounce_source);
        if (*switcher).panel.is_null()
            || gtk_widget_get_visible((*switcher).panel) == 0
            || (*switcher).previews.is_null()
        {
            return;
        }

        remove_source_if_active(&mut (*switcher).preview_card_build_source);
        (*switcher).search_debounce_source =
            add_switcher_timeout(SEARCH_DEBOUNCE_MS, Some(apply_search_filter), switcher);
    }
}

// Allow Enter to select a channel before its card has been built.
pub(super) unsafe fn activate_first_matching_channel(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if switcher.is_null() || (*switcher).previews.is_null() {
            return;
        }

        let filter = current_search_filter(switcher);
        for i in 0..(*(*switcher).previews).len {
            let preview = ptr_array_index((*switcher).previews, i);
            if preview_matches_filter(preview, filter) {
                activate_dynamic_channel(switcher, (*preview).channel, (*preview).display_name);
                return;
            }
        }
    }
}

pub(super) unsafe extern "C" fn on_search_activate(
    _entry: *mut GtkSearchEntry,
    user_data: *mut c_void,
) {
    unsafe {
        let switcher = user_data as *mut ChannelSwitcherOverlay;
        if switcher_is_closing(switcher) {
            return;
        }

        remove_source_if_active(&mut (*switcher).search_debounce_source);
        render_live_channels(switcher);
        activate_first_matching_channel(switcher);
    }
}

pub(super) unsafe extern "C" fn on_close_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    unsafe {
        let switcher = user_data as *mut ChannelSwitcherOverlay;
        if !switcher_is_closing(switcher) {
            channel_switcher_overlay_hide(switcher);
        }
    }
}

pub(super) unsafe extern "C" fn on_settings_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let switcher = user_data as *mut ChannelSwitcherOverlay;
        if switcher_is_closing(switcher) {
            return;
        }

        let callback = (*switcher).settings_callback;
        let callback_data = (*switcher).settings_user_data;

        channel_switcher_overlay_hide(switcher);
        if let Some(callback) = callback {
            callback(callback_data);
        }
    }
}

pub(super) unsafe extern "C" fn on_backdrop_pressed(
    _gesture: *mut GtkGestureClick,
    n_press: c_int,
    _x: c_double,
    _y: c_double,
    user_data: *mut c_void,
) {
    unsafe {
        let switcher = user_data as *mut ChannelSwitcherOverlay;
        if n_press == 1 && !switcher_is_closing(switcher) {
            channel_switcher_overlay_hide(switcher);
        }
    }
}
