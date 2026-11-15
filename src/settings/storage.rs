use super::channels::{
    app_settings_add_channel, app_settings_get_channel, app_settings_get_channel_count,
};
use super::ffi::{
    G_FILE_TEST_EXISTS, GError, JsonNode, JsonObject, g_build_filenamev, g_clear_error,
    g_file_error_from_errno, g_file_error_quark, g_file_test, g_free, g_get_user_config_dir,
    g_mkdir_with_parents, g_object_unref, g_set_error, g_strerror, json_array_get_element,
    json_array_get_length, json_builder_add_boolean_value, json_builder_add_int_value,
    json_builder_add_string_value, json_builder_begin_array, json_builder_begin_object,
    json_builder_end_array, json_builder_end_object, json_builder_get_root, json_builder_new,
    json_builder_set_member_name, json_generator_new, json_generator_set_pretty,
    json_generator_set_root, json_generator_to_file, json_node_get_array, json_node_get_node_type,
    json_node_get_object, json_node_unref, json_object_get_boolean_member_with_default,
    json_object_get_int_member_with_default, json_object_get_member,
    json_object_get_string_member_with_default, json_parser_get_root, json_parser_load_from_file,
    json_parser_new,
};
use super::{
    AppSettings, app_settings_get_hwdec_enabled, app_settings_get_twitch_oauth_expires_at,
    app_settings_get_twitch_oauth_token, app_settings_get_twitch_playback_auth_token,
    app_settings_get_twitch_refresh_token, app_settings_new, app_settings_set_twitch_auth_tokens,
    app_settings_set_twitch_oauth_token, app_settings_set_twitch_playback_auth_token,
};
use crate::glib_strings::is_nonempty;
use std::ffi::{c_char, c_int, c_void};
use std::{io, ptr};
pub(super) const JSON_NODE_OBJECT: c_int = 0;
pub(super) const JSON_NODE_ARRAY: c_int = 1;

pub(super) unsafe fn build_filename(parts: &[*const c_char]) -> *mut c_char {
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

pub(super) unsafe fn load_channels(settings: *mut AppSettings, root: *mut JsonObject) {
    unsafe {
        let channels_node = json_object_get_member(root, c"channels".as_ptr());
        if channels_node.is_null() || json_node_get_node_type(channels_node) != JSON_NODE_ARRAY {
            return;
        }

        let channels = json_node_get_array(channels_node);
        for i in 0..json_array_get_length(channels) {
            let node = json_array_get_element(channels, i);
            if node.is_null() || json_node_get_node_type(node) != JSON_NODE_OBJECT {
                continue;
            }

            let item = json_node_get_object(node);
            let label =
                json_object_get_string_member_with_default(item, c"label".as_ptr(), c"".as_ptr());
            let channel =
                json_object_get_string_member_with_default(item, c"channel".as_ptr(), c"".as_ptr());
            let url =
                json_object_get_string_member_with_default(item, c"url".as_ptr(), c"".as_ptr());
            app_settings_add_channel(settings, label, channel, url);
        }
    }
}

/// Load saved settings, keeping defaults when no settings file is available.
pub fn app_settings_load() -> *mut AppSettings {
    unsafe {
        let settings = app_settings_new();
        let path = app_settings_get_path();

        if g_file_test(path, G_FILE_TEST_EXISTS) == 0 {
            g_free(path as *mut c_void);
            return settings;
        }

        let parser = json_parser_new();
        let mut error: *mut GError = ptr::null_mut();
        if json_parser_load_from_file(parser, path, &mut error) == 0 {
            g_clear_error(&mut error);
            g_object_unref(parser as *mut c_void);
            g_free(path as *mut c_void);
            return settings;
        }

        load_json(settings, json_parser_get_root(parser));

        g_object_unref(parser as *mut c_void);
        g_free(path as *mut c_void);
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
            g_free(config_dir as *mut c_void);
            g_free(path as *mut c_void);
            return 0;
        }

        let root = to_json(settings);
        let generator = json_generator_new();
        json_generator_set_root(generator, root);
        json_generator_set_pretty(generator, 1);
        let result = json_generator_to_file(generator, path, error);

        g_object_unref(generator as *mut c_void);
        json_node_unref(root);
        g_free(config_dir as *mut c_void);
        g_free(path as *mut c_void);
        result
    }
}

// Read settings from a JSON node; invalid roots leave the defaults in place.
pub(super) unsafe fn load_json(settings: *mut AppSettings, root_node: *mut JsonNode) {
    unsafe {
        if root_node.is_null() || json_node_get_node_type(root_node) != JSON_NODE_OBJECT {
            return;
        }
        let root = json_node_get_object(root_node);
        (*settings).hwdec_enabled =
            json_object_get_boolean_member_with_default(root, c"hwdec".as_ptr(), 1);
        app_settings_set_twitch_oauth_token(
            settings,
            json_object_get_string_member_with_default(
                root,
                c"twitch_oauth_token".as_ptr(),
                ptr::null(),
            ),
        );
        app_settings_set_twitch_auth_tokens(
            settings,
            app_settings_get_twitch_oauth_token(settings),
            json_object_get_string_member_with_default(
                root,
                c"twitch_refresh_token".as_ptr(),
                ptr::null(),
            ),
            json_object_get_int_member_with_default(root, c"twitch_oauth_expires_at".as_ptr(), 0),
        );
        app_settings_set_twitch_playback_auth_token(
            settings,
            json_object_get_string_member_with_default(
                root,
                c"twitch_playback_auth_token".as_ptr(),
                ptr::null(),
            ),
        );
        load_channels(settings, root);
    }
}

// Return an owned JSON node for saving or inspecting the settings.
pub(super) unsafe fn to_json(settings: *mut AppSettings) -> *mut JsonNode {
    unsafe {
        let builder = json_builder_new();
        json_builder_begin_object(builder);
        json_builder_set_member_name(builder, c"hwdec".as_ptr());
        json_builder_add_boolean_value(builder, app_settings_get_hwdec_enabled(settings));

        let oauth_token = app_settings_get_twitch_oauth_token(settings);
        if is_nonempty(oauth_token) {
            json_builder_set_member_name(builder, c"twitch_oauth_token".as_ptr());
            json_builder_add_string_value(builder, oauth_token);
        }
        let refresh_token = app_settings_get_twitch_refresh_token(settings);
        if is_nonempty(refresh_token) {
            json_builder_set_member_name(builder, c"twitch_refresh_token".as_ptr());
            json_builder_add_string_value(builder, refresh_token);
        }
        let oauth_expires_at = app_settings_get_twitch_oauth_expires_at(settings);
        if oauth_expires_at > 0 {
            json_builder_set_member_name(builder, c"twitch_oauth_expires_at".as_ptr());
            json_builder_add_int_value(builder, oauth_expires_at);
        }
        let playback_auth_token = app_settings_get_twitch_playback_auth_token(settings);
        if is_nonempty(playback_auth_token) {
            json_builder_set_member_name(builder, c"twitch_playback_auth_token".as_ptr());
            json_builder_add_string_value(builder, playback_auth_token);
        }

        json_builder_set_member_name(builder, c"channels".as_ptr());
        json_builder_begin_array(builder);
        for i in 0..app_settings_get_channel_count(settings) {
            let channel = app_settings_get_channel(settings, i);
            json_builder_begin_object(builder);
            json_builder_set_member_name(builder, c"label".as_ptr());
            json_builder_add_string_value(builder, (*channel).label);
            json_builder_set_member_name(builder, c"channel".as_ptr());
            json_builder_add_string_value(builder, (*channel).channel);
            json_builder_set_member_name(builder, c"url".as_ptr());
            json_builder_add_string_value(builder, (*channel).url);
            json_builder_end_object(builder);
        }
        json_builder_end_array(builder);
        json_builder_end_object(builder);

        let root = json_builder_get_root(builder);
        g_object_unref(builder as *mut c_void);
        root
    }
}
