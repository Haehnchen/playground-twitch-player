mod channels;
mod ffi;
mod storage;

pub use self::channels::{
    app_settings_add_channel, app_settings_clear_channels, app_settings_get_channel,
    app_settings_get_channel_count,
};
pub use self::ffi::{GError, GPtrArray};
pub use self::storage::{app_settings_get_path, app_settings_load, app_settings_save};

use self::channels::{app_settings_channel_free, dup_bytes, trimmed_bytes};
use self::ffi::{g_free, g_ptr_array_new_with_free_func, g_ptr_array_unref, g_strdup};
use crate::glib_strings::is_nonempty;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct AppSettingsChannel {
    pub label: *mut c_char,
    pub channel: *mut c_char,
    pub url: *mut c_char,
}

pub struct AppSettings {
    channels: *mut GPtrArray,

    twitch_oauth_token: *mut c_char,
    twitch_refresh_token: *mut c_char,
    twitch_oauth_expires_at: i64,
    twitch_playback_auth_token: *mut c_char,

    hwdec_enabled: c_int,

    ref_count: AtomicUsize,
}

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

pub fn app_settings_new() -> *mut AppSettings {
    unsafe {
        Box::into_raw(Box::new(AppSettings {
            channels: g_ptr_array_new_with_free_func(Some(app_settings_channel_free)),

            twitch_oauth_token: ptr::null_mut(),
            twitch_refresh_token: ptr::null_mut(),
            twitch_oauth_expires_at: 0,
            twitch_playback_auth_token: ptr::null_mut(),

            hwdec_enabled: 1,

            ref_count: AtomicUsize::new(1),
        }))
    }
}

pub(crate) unsafe fn app_settings_ref(settings: *mut AppSettings) -> *mut AppSettings {
    unsafe {
        if !settings.is_null() {
            (*settings).ref_count.fetch_add(1, Ordering::Relaxed);
        }
        settings
    }
}

unsafe fn app_settings_unref(settings: *mut AppSettings) {
    unsafe {
        if settings.is_null() || (*settings).ref_count.fetch_sub(1, Ordering::AcqRel) != 1 {
            return;
        }

        let settings = Box::from_raw(settings);
        g_ptr_array_unref(settings.channels);
        g_free(settings.twitch_oauth_token as *mut c_void);
        g_free(settings.twitch_refresh_token as *mut c_void);
        g_free(settings.twitch_playback_auth_token as *mut c_void);
    }
}

/// # Safety
/// - `settings`: null or owned reference; release each reference once.
/// - No borrowed-field access after the last release.
pub unsafe fn app_settings_free(settings: *mut AppSettings) {
    unsafe {
        app_settings_unref(settings);
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
pub unsafe fn app_settings_get_hwdec_enabled(settings: *const AppSettings) -> c_int {
    unsafe {
        if settings.is_null() {
            return 1;
        }

        (*settings).hwdec_enabled
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
pub unsafe fn app_settings_set_hwdec_enabled(settings: *mut AppSettings, enabled: c_int) {
    unsafe {
        if !settings.is_null() {
            (*settings).hwdec_enabled = enabled;
        }
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
/// - Result: borrowed until token mutation or settings destruction.
pub unsafe fn app_settings_get_twitch_oauth_token(settings: *const AppSettings) -> *const c_char {
    unsafe {
        if settings.is_null() {
            return ptr::null();
        }

        (*settings).twitch_oauth_token
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
/// - Result: borrowed until token mutation or settings destruction.
pub unsafe fn app_settings_get_twitch_refresh_token(settings: *const AppSettings) -> *const c_char {
    unsafe {
        if settings.is_null() {
            return ptr::null();
        }

        (*settings).twitch_refresh_token
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
pub unsafe fn app_settings_get_twitch_oauth_expires_at(settings: *const AppSettings) -> i64 {
    unsafe {
        if settings.is_null() {
            return 0;
        }

        (*settings).twitch_oauth_expires_at
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
/// - Result: borrowed until token mutation or settings destruction.
pub unsafe fn app_settings_get_twitch_playback_auth_token(
    settings: *const AppSettings,
) -> *const c_char {
    unsafe {
        if settings.is_null() {
            return ptr::null();
        }

        (*settings).twitch_playback_auth_token
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn app_settings_set_twitch_playback_auth_token(
    settings: *mut AppSettings,
    token: *const c_char,
) {
    unsafe {
        if settings.is_null() {
            return;
        }

        let token = trimmed_bytes(token);
        let new_token = if token.is_empty() {
            ptr::null_mut()
        } else {
            dup_bytes(&token)
        };
        g_free((*settings).twitch_playback_auth_token as *mut c_void);
        (*settings).twitch_playback_auth_token = new_token;
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn app_settings_set_twitch_oauth_token(
    settings: *mut AppSettings,
    oauth_token: *const c_char,
) {
    unsafe {
        if settings.is_null() {
            return;
        }

        let new_oauth_token = if is_nonempty(oauth_token) {
            g_strdup(oauth_token)
        } else {
            ptr::null_mut()
        };
        g_free((*settings).twitch_oauth_token as *mut c_void);
        (*settings).twitch_oauth_token = new_oauth_token;
        if (*settings).twitch_oauth_token.is_null() {
            g_free((*settings).twitch_refresh_token as *mut c_void);
            (*settings).twitch_refresh_token = ptr::null_mut();
            (*settings).twitch_oauth_expires_at = 0;
        }
    }
}

/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn app_settings_set_twitch_auth_tokens(
    settings: *mut AppSettings,
    oauth_token: *const c_char,
    refresh_token: *const c_char,
    oauth_expires_at: i64,
) {
    unsafe {
        if settings.is_null() {
            return;
        }

        let new_oauth_token = if is_nonempty(oauth_token) {
            g_strdup(oauth_token)
        } else {
            ptr::null_mut()
        };
        let new_refresh_token = if is_nonempty(refresh_token) {
            g_strdup(refresh_token)
        } else {
            ptr::null_mut()
        };

        g_free((*settings).twitch_oauth_token as *mut c_void);
        (*settings).twitch_oauth_token = new_oauth_token;
        g_free((*settings).twitch_refresh_token as *mut c_void);
        (*settings).twitch_refresh_token = new_refresh_token;
        (*settings).twitch_oauth_expires_at = if (*settings).twitch_oauth_token.is_null() {
            0
        } else {
            oauth_expires_at
        };
    }
}

pub mod window;

#[cfg(test)]
mod tests {
    use super::channels::app_settings_get_channel_count;
    use super::*;
    use std::ffi::CStr;
    use std::sync::atomic::Ordering;

    #[test]
    fn owner_release_waits_for_async_reference() {
        unsafe {
            let settings = app_settings_new();
            let pending = app_settings_ref(settings);

            app_settings_free(settings);

            assert_eq!((*pending).ref_count.load(Ordering::Acquire), 1);
            assert_eq!(app_settings_get_channel_count(pending), 0);

            app_settings_free(pending);
        }
    }

    #[test]
    fn channels_and_tokens_round_trip_through_json() {
        unsafe {
            let settings = app_settings_new();
            app_settings_set_hwdec_enabled(settings, 0);
            app_settings_set_twitch_auth_tokens(
                settings,
                c"token-123".as_ptr(),
                c"refresh-456".as_ptr(),
                123456789,
            );
            app_settings_set_twitch_playback_auth_token(
                settings,
                c"  playback-token-789  ".as_ptr(),
            );
            app_settings_add_channel(
                settings,
                c"Papaplatte Live".as_ptr(),
                c"https://www.twitch.tv/PapaPlatte".as_ptr(),
                ptr::null(),
            );

            let encoded = serde_json::to_string(&storage::to_json(settings)).unwrap();
            let json = serde_json::from_str(&encoded).unwrap();
            app_settings_free(settings);
            let restored = app_settings_new();
            storage::load_json(restored, &json);

            assert_eq!(app_settings_get_hwdec_enabled(restored), 0);
            assert_eq!(
                app_settings_get_twitch_oauth_expires_at(restored),
                123456789
            );
            assert_eq!(app_settings_get_channel_count(restored), 1);
            let channel = app_settings_get_channel(restored, 0);
            for (actual, expected) in [
                (app_settings_get_twitch_oauth_token(restored), "token-123"),
                (
                    app_settings_get_twitch_refresh_token(restored),
                    "refresh-456",
                ),
                (
                    app_settings_get_twitch_playback_auth_token(restored),
                    "playback-token-789",
                ),
                ((*channel).label as *const c_char, "Papaplatte Live"),
                ((*channel).channel as *const c_char, "papaplatte"),
                (
                    (*channel).url as *const c_char,
                    "https://www.twitch.tv/papaplatte",
                ),
            ] {
                assert_eq!(CStr::from_ptr(actual).to_str().unwrap(), expected);
            }
            app_settings_free(restored);
        }
    }

    #[test]
    fn missing_and_unknown_fields_keep_defaults() {
        for json in [
            serde_json::json!({}),
            serde_json::json!({"future_setting": true}),
            serde_json::Value::Null,
        ] {
            unsafe {
                let settings = app_settings_new();
                storage::load_json(settings, &json);
                assert_eq!(app_settings_get_hwdec_enabled(settings), 1);
                assert_eq!(app_settings_get_channel_count(settings), 0);
                assert!(app_settings_get_twitch_oauth_token(settings).is_null());
                app_settings_free(settings);
            }
        }
    }

    #[test]
    fn null_fields_and_invalid_channel_entries_are_ignored() {
        let json = serde_json::json!({
            "hwdec": false,
            "twitch_oauth_token": null,
            "twitch_oauth_expires_at": null,
            "channels": [null, 42, {}, {"channel": "PapaPlatte", "label": null, "url": null}],
        });
        unsafe {
            let settings = app_settings_new();
            storage::load_json(settings, &json);
            assert_eq!(app_settings_get_hwdec_enabled(settings), 0);
            assert_eq!(app_settings_get_channel_count(settings), 1);
            assert!(app_settings_get_twitch_oauth_token(settings).is_null());
            let channel = app_settings_get_channel(settings, 0);
            assert_eq!(CStr::from_ptr((*channel).channel), c"papaplatte");
            assert_eq!(
                CStr::from_ptr((*channel).url),
                c"https://www.twitch.tv/papaplatte"
            );
            app_settings_free(settings);
        }
    }

    #[test]
    fn json_preserves_unicode_and_escaping_without_empty_tokens() {
        unsafe {
            let settings = app_settings_new();
            app_settings_add_channel(
                settings,
                c"Grüße \"live\"\nChat".as_ptr(),
                c"example".as_ptr(),
                ptr::null(),
            );
            let json = storage::to_json(settings);
            assert!(json.get("twitch_oauth_token").is_none());
            assert!(json.get("twitch_refresh_token").is_none());
            assert!(json.get("twitch_playback_auth_token").is_none());
            assert!(json.get("twitch_oauth_expires_at").is_none());
            let encoded = serde_json::to_string(&json).unwrap();
            let decoded: serde_json::Value = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded["channels"][0]["label"], "Grüße \"live\"\nChat");
            app_settings_free(settings);
        }
    }
}
