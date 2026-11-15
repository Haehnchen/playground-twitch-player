use super::channels::{
    app_settings_add_channel, app_settings_get_channel, app_settings_get_channel_count,
};
use super::ffi::{
    GError, g_build_filenamev, g_file_error_from_errno, g_file_error_quark, g_file_get_contents,
    g_file_set_contents, g_free, g_get_user_config_dir, g_mkdir_with_parents, g_set_error,
    g_strerror,
};
use super::{
    AppSettings, app_settings_get_hwdec_enabled, app_settings_get_twitch_oauth_expires_at,
    app_settings_get_twitch_oauth_token, app_settings_get_twitch_playback_auth_token,
    app_settings_get_twitch_refresh_token, app_settings_new, app_settings_set_twitch_auth_tokens,
    app_settings_set_twitch_playback_auth_token,
};
use crate::glib_strings::{dup_bytes, is_nonempty};
use serde_json::{Value, json};
use std::ffi::{CStr, c_char, c_int};
use std::{io, ptr, slice};

unsafe fn build_filename(parts: &[*const c_char]) -> *mut c_char {
    unsafe {
        let mut args: Vec<*mut c_char> = parts.iter().map(|part| *part as *mut c_char).collect();
        args.push(ptr::null_mut());
        g_build_filenamev(args.as_mut_ptr())
    }
}

pub fn app_settings_get_path() -> *mut c_char {
    unsafe {
        build_filename(&[
            g_get_user_config_dir(),
            c"twitch-player".as_ptr(),
            c"settings.json".as_ptr(),
        ])
    }
}

/// Load saved settings, keeping defaults when no settings file is available.
pub fn app_settings_load() -> *mut AppSettings {
    unsafe {
        let settings = app_settings_new();
        let path = app_settings_get_path();
        let mut contents: *mut c_char = ptr::null_mut();
        let mut length = 0;
        if g_file_get_contents(path, &mut contents, &mut length, ptr::null_mut()) != 0 {
            let bytes = slice::from_raw_parts(contents.cast(), length);
            if let Ok(json) = serde_json::from_slice(bytes) {
                load_json(settings, &json);
            }
        }
        g_free(contents.cast());
        g_free(path.cast());
        settings
    }
}

/// Write settings as JSON in the user's config directory.
///
/// # Safety
/// - `settings`: null or live instance; serialize reads and mutations.
/// - `error`: null or writable, initially null slot; `E` must represent `GError`.
pub unsafe fn app_settings_save<E>(settings: *mut AppSettings, error: *mut *mut E) -> c_int {
    unsafe {
        let error = error as *mut *mut GError;
        let config_dir = build_filename(&[g_get_user_config_dir(), c"twitch-player".as_ptr()]);
        let path = app_settings_get_path();
        if g_mkdir_with_parents(config_dir, 0o700) < 0 {
            let errno = io::Error::last_os_error().raw_os_error().unwrap_or(0);
            g_set_error(
                error,
                g_file_error_quark(),
                g_file_error_from_errno(errno),
                c"Could not create %s: %s".as_ptr(),
                config_dir,
                g_strerror(errno),
            );
            g_free(config_dir.cast());
            g_free(path.cast());
            return 0;
        }

        let json = serde_json::to_vec_pretty(&to_json(settings))
            .expect("settings contain only JSON values");
        let result = g_file_set_contents(path, json.as_ptr().cast(), json.len() as isize, error);
        g_free(config_dir.cast());
        g_free(path.cast());
        result
    }
}

// Missing or incorrectly typed fields keep their existing defaults.
pub(super) unsafe fn load_json(settings: *mut AppSettings, root: &Value) {
    unsafe {
        if !root.is_object() {
            return;
        }
        (*settings).hwdec_enabled = root["hwdec"].as_bool().unwrap_or(true).into();
        let [token, refresh, playback] = [
            "twitch_oauth_token",
            "twitch_refresh_token",
            "twitch_playback_auth_token",
        ]
        .map(|key| dup_bytes(root[key].as_str().unwrap_or("").as_bytes()));
        app_settings_set_twitch_auth_tokens(
            settings,
            token,
            refresh,
            root["twitch_oauth_expires_at"].as_i64().unwrap_or(0),
        );
        app_settings_set_twitch_playback_auth_token(settings, playback);
        for value in [token, refresh, playback] {
            g_free(value.cast());
        }

        if let Some(channels) = root["channels"].as_array() {
            for entry in channels.iter().filter(|entry| entry.is_object()) {
                let [label, channel, url] = ["label", "channel", "url"]
                    .map(|key| dup_bytes(entry[key].as_str().unwrap_or("").as_bytes()));
                app_settings_add_channel(settings, label, channel, url);
                for value in [label, channel, url] {
                    g_free(value.cast());
                }
            }
        }
    }
}

pub(super) unsafe fn to_json(settings: *mut AppSettings) -> Value {
    unsafe {
        let channels: Vec<_> = (0..app_settings_get_channel_count(settings))
            .map(|index| {
                let channel = app_settings_get_channel(settings, index);
                json!({
                    "label": CStr::from_ptr((*channel).label).to_string_lossy(),
                    "channel": CStr::from_ptr((*channel).channel).to_string_lossy(),
                    "url": CStr::from_ptr((*channel).url).to_string_lossy(),
                })
            })
            .collect();
        let mut root = json!({
            "hwdec": app_settings_get_hwdec_enabled(settings) != 0,
            "channels": channels,
        });
        for (key, value) in [
            (
                "twitch_oauth_token",
                app_settings_get_twitch_oauth_token(settings),
            ),
            (
                "twitch_refresh_token",
                app_settings_get_twitch_refresh_token(settings),
            ),
            (
                "twitch_playback_auth_token",
                app_settings_get_twitch_playback_auth_token(settings),
            ),
        ] {
            if is_nonempty(value) {
                root[key] = json!(CStr::from_ptr(value).to_string_lossy());
            }
        }
        let expires_at = app_settings_get_twitch_oauth_expires_at(settings);
        if expires_at > 0 {
            root["twitch_oauth_expires_at"] = json!(expires_at);
        }
        root
    }
}
