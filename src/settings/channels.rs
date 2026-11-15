use super::ffi::{g_free, g_ptr_array_add, g_ptr_array_set_size, g_strdup};
use super::{AppSettings, AppSettingsChannel};
use crate::twitch::channel::normalize_twitch_channel;
use std::ffi::{CStr, CString, c_char, c_uint, c_void};
use std::ptr;

pub(super) unsafe extern "C" fn app_settings_channel_free(data: *mut c_void) {
    unsafe {
        if data.is_null() {
            return;
        }

        let channel = Box::from_raw(data as *mut AppSettingsChannel);
        g_free(channel.label as *mut c_void);
        g_free(channel.channel as *mut c_void);
        g_free(channel.url as *mut c_void);
    }
}

pub(super) unsafe fn trimmed_bytes(value: *const c_char) -> Vec<u8> {
    if value.is_null() {
        return Vec::new();
    }
    unsafe { CStr::from_ptr(value).to_bytes().trim_ascii().to_vec() }
}

pub(super) unsafe fn dup_bytes(bytes: &[u8]) -> *mut c_char {
    unsafe {
        let string = CString::new(bytes).unwrap_or_default();
        g_strdup(string.as_ptr())
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
pub unsafe fn app_settings_get_channel_count(settings: *const AppSettings) -> c_uint {
    unsafe {
        if settings.is_null() {
            return 0;
        }

        (*(*settings).channels).len
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
        if settings.is_null() || index >= (*(*settings).channels).len {
            return ptr::null();
        }

        *(*(*settings).channels).pdata.add(index as usize) as *const AppSettingsChannel
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
pub unsafe fn app_settings_clear_channels(settings: *mut AppSettings) {
    unsafe {
        if !settings.is_null() {
            g_ptr_array_set_size((*settings).channels, 0);
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

        let entry = Box::new(AppSettingsChannel {
            label: dup_bytes(&stored_label),
            channel: dup_bytes(&stored_channel),
            url: dup_bytes(&stored_url),
        });
        g_ptr_array_add((*settings).channels, Box::into_raw(entry) as *mut c_void);
    }
}
