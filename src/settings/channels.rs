use super::{AppSettings, AppSettingsChannel};
use crate::twitch::channel::normalize_twitch_channel;
use std::ffi::{CStr, CString, c_char, c_uint};
use std::ptr;

pub(super) unsafe fn trimmed_bytes(value: *const c_char) -> Vec<u8> {
    if value.is_null() {
        return Vec::new();
    }
    unsafe { CStr::from_ptr(value).to_bytes().trim_ascii().to_vec() }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
pub unsafe fn app_settings_get_channel_count(settings: *const AppSettings) -> c_uint {
    unsafe {
        if settings.is_null() {
            return 0;
        }

        (*settings).channels.len() as c_uint
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
/// - Result: borrowed until channel mutation or settings destruction.
pub unsafe fn app_settings_get_channel(
    settings: *const AppSettings,
    index: c_uint,
) -> *const AppSettingsChannel {
    unsafe {
        if settings.is_null() || index >= (*settings).channels.len() as c_uint {
            return ptr::null();
        }

        &(&(*settings).channels)[index as usize]
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
pub unsafe fn app_settings_clear_channels(settings: *mut AppSettings) {
    unsafe {
        if !settings.is_null() {
            (*settings).channels.clear();
        }
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn app_settings_add_channel(
    settings: *mut AppSettings,
    label: *const c_char,
    channel: *const c_char,
    url: *const c_char,
) {
    unsafe {
        if settings.is_null() {
            return;
        }

        let trimmed_label = trimmed_bytes(label);
        let trimmed_channel = trimmed_bytes(channel);
        let trimmed_url = trimmed_bytes(url);
        let derived_channel = normalize_twitch_channel(&trimmed_channel)
            .or_else(|| normalize_twitch_channel(&trimmed_url));

        if derived_channel.is_none() && trimmed_url.is_empty() && trimmed_channel.is_empty() {
            return;
        }

        let stored_channel = derived_channel.unwrap_or(trimmed_channel);
        let stored_url = if !trimmed_url.is_empty() {
            trimmed_url
        } else if !stored_channel.is_empty() {
            let mut value = b"https://www.twitch.tv/".to_vec();
            value.extend_from_slice(&stored_channel);
            value
        } else {
            Vec::new()
        };
        let stored_label = if !trimmed_label.is_empty() {
            trimmed_label
        } else if !stored_channel.is_empty() {
            stored_channel.clone()
        } else {
            stored_url.clone()
        };

        let entry = AppSettingsChannel {
            label: CString::new(stored_label).expect("C string label"),
            channel: CString::new(stored_channel).expect("C string channel"),
            url: CString::new(stored_url).expect("C string URL"),
        };
        (*settings).channels.push(entry);
    }
}
