mod channels;
mod ffi;
mod storage;

pub use self::channels::{
    app_settings_add_channel, app_settings_clear_channels, app_settings_get_channel,
    app_settings_get_channel_count,
};
pub use self::ffi::GError;
pub use self::storage::{app_settings_get_path, app_settings_load, app_settings_save};

use self::channels::trimmed_bytes;
use std::ffi::{CStr, CString, c_char, c_int};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct AppSettingsChannel {
    pub label: CString,
    pub channel: CString,
    pub url: CString,
}

pub struct AppSettings {
    channels: Vec<AppSettingsChannel>,

    twitch_oauth_token: Option<CString>,
    twitch_refresh_token: Option<CString>,
    twitch_oauth_expires_at: i64,
    twitch_playback_auth_token: Option<CString>,

    hwdec_enabled: c_int,

    ref_count: AtomicUsize,
}

pub fn app_settings_new() -> *mut AppSettings {
    Box::into_raw(Box::new(AppSettings {
        channels: Vec::new(),

        twitch_oauth_token: None,
        twitch_refresh_token: None,
        twitch_oauth_expires_at: 0,
        twitch_playback_auth_token: None,

        hwdec_enabled: 1,

        ref_count: AtomicUsize::new(1),
    }))
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

        drop(Box::from_raw(settings));
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

        (*settings)
            .twitch_oauth_token
            .as_ref()
            .map_or(ptr::null(), |token| token.as_ptr())
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

        (*settings)
            .twitch_refresh_token
            .as_ref()
            .map_or(ptr::null(), |token| token.as_ptr())
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

        (*settings)
            .twitch_playback_auth_token
            .as_ref()
            .map_or(ptr::null(), |token| token.as_ptr())
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
        (*settings).twitch_playback_auth_token =
            (!token.is_empty()).then(|| CString::new(token).expect("trimmed C string"));
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

        (*settings).twitch_oauth_token = copy_token(oauth_token);
        if (*settings).twitch_oauth_token.is_none() {
            (*settings).twitch_refresh_token = None;
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

        // Copy both inputs before replacing tokens; callers may pass borrowed getters.
        let oauth = copy_token(oauth_token);
        let refresh = copy_token(refresh_token);
        (*settings).twitch_oauth_expires_at = if oauth.is_some() { oauth_expires_at } else { 0 };
        (*settings).twitch_oauth_token = oauth;
        (*settings).twitch_refresh_token = refresh;
    }
}

unsafe fn copy_token(token: *const c_char) -> Option<CString> {
    unsafe { (!token.is_null() && *token != 0).then(|| CStr::from_ptr(token).to_owned()) }
}

pub mod window;

#[cfg(test)]
mod tests {
    use super::channels::app_settings_get_channel_count;
    use super::*;
    use std::ffi::CStr;
    use std::sync::atomic::Ordering;

    #[test]
    fn token_setters_accept_their_own_borrowed_values() {
        unsafe {
            let settings = app_settings_new();
            app_settings_set_twitch_auth_tokens(
                settings,
                c"access".as_ptr(),
                c"refresh".as_ptr(),
                42,
            );
            app_settings_set_twitch_auth_tokens(
                settings,
                app_settings_get_twitch_refresh_token(settings),
                app_settings_get_twitch_oauth_token(settings),
                84,
            );
            assert_eq!(
                CStr::from_ptr(app_settings_get_twitch_oauth_token(settings)),
                c"refresh"
            );
            assert_eq!(
                CStr::from_ptr(app_settings_get_twitch_refresh_token(settings)),
                c"access"
            );
            app_settings_set_twitch_oauth_token(settings, ptr::null());
            assert!(app_settings_get_twitch_refresh_token(settings).is_null());
            assert_eq!(app_settings_get_twitch_oauth_expires_at(settings), 0);
            app_settings_free(settings);
        }
    }

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
                ((*channel).label.as_ptr(), "Papaplatte Live"),
                ((*channel).channel.as_ptr(), "papaplatte"),
                ((*channel).url.as_ptr(), "https://www.twitch.tv/papaplatte"),
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
            assert_eq!((*channel).channel.as_c_str(), c"papaplatte");
            assert_eq!(
                (*channel).url.as_c_str(),
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
