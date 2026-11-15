use super::ffi::{
    G_IO_ERROR_FAILED, GCancellable, GError, GPtrArray, TwitchPlaybackInfo, g_ascii_strcasecmp,
    g_cancellable_is_cancelled, g_free, g_io_error_quark, g_ptr_array_add,
    g_ptr_array_new_with_free_func, g_ptr_array_sort, g_ptr_array_unref, g_set_error,
    g_uri_escape_string,
};
use super::requests::get_twitch_helix_request;
use super::utils::{twitch_followed_channel_free_destroy, twitch_stream_preview_free_destroy};
use super::{
    TwitchCurrentStream, TwitchFollowedChannel, TwitchStreamPreview, twitch_playback_info_empty,
};
use crate::glib_strings::dup_bytes;
use crate::twitch::json::{dup_string, parse, string, uint};
use gtk4::glib::{self, translate::IntoGlibPtr};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;

pub(super) fn parse_current_stream_response(
    json: &[u8],
) -> Result<*mut TwitchCurrentStream, glib::Error> {
    let value = parse(json)?;
    let stream = &value["data"]["user"]["stream"];
    if !stream.is_object() {
        return Ok(ptr::null_mut());
    }
    Ok(Box::into_raw(Box::new(TwitchCurrentStream {
        title: dup_bytes(string(&stream["title"]).unwrap_or("").as_bytes()),
        started_at: dup_string(&stream["createdAt"]),
        category_name: dup_string(&stream["game"]["name"]),
        viewer_count: uint(&stream["viewersCount"]),
    })))
}

pub(super) fn parse_live_channels_response(json: &[u8]) -> Result<*mut GPtrArray, glib::Error> {
    unsafe {
        let value = parse(json)?;
        let previews = g_ptr_array_new_with_free_func(Some(twitch_stream_preview_free_destroy));
        let Some(users) = value["data"]["users"].as_array() else {
            return Ok(previews);
        };
        for user in users {
            let stream = &user["stream"];
            let Some(login) = string(&user["login"]).filter(|s| !s.is_empty()) else {
                continue;
            };
            if !stream.is_object() {
                continue;
            }
            let display_name = string(&user["displayName"])
                .filter(|s| !s.is_empty())
                .unwrap_or(login);
            let preview = Box::new(TwitchStreamPreview {
                channel: dup_bytes(login.to_ascii_lowercase().as_bytes()),
                display_name: dup_bytes(display_name.as_bytes()),
                title: dup_bytes(string(&stream["title"]).unwrap_or("").as_bytes()),
                avatar_url: dup_string(&user["profileImageURL"]),
                preview_url: dup_string(&stream["previewImageURL"]),
                started_at: dup_string(&stream["createdAt"]),
                category_name: dup_string(&stream["game"]["name"]),
                viewer_count: uint(&stream["viewersCount"]),
            });
            g_ptr_array_add(previews, Box::into_raw(preview).cast());
        }
        g_ptr_array_sort(previews, Some(compare_stream_previews_by_viewers));
        Ok(previews)
    }
}

pub(super) fn parse_playback_info(value: &str) -> TwitchPlaybackInfo {
    let mut info = twitch_playback_info_empty();
    let Ok(value) = parse(value.as_bytes()) else {
        return info;
    };
    if !value.is_object() {
        return info;
    }
    info.available = 1;
    info.user_id = value["user_id"].as_i64().unwrap_or(0).max(0) as u64;
    info.authenticated = (info.user_id > 0) as c_int;
    info.subscriber = value["subscriber"].as_bool().unwrap_or(false) as c_int;
    info.turbo = value["turbo"].as_bool().unwrap_or(false) as c_int;
    info.show_ads = value["show_ads"].as_bool().unwrap_or(false) as c_int;
    info.hide_ads = value["hide_ads"].as_bool().unwrap_or(false) as c_int;
    info.server_ads = value["server_ads"].as_bool().unwrap_or(false) as c_int;
    info
}

pub(super) struct PlaybackToken {
    pub value: String,
    pub signature: String,
    pub info: TwitchPlaybackInfo,
}

// The token value contains another JSON document with playback claims.
pub(super) fn parse_playback_access_token_response(
    json: &[u8],
) -> Result<Option<PlaybackToken>, glib::Error> {
    let value = parse(json)?;
    let token = &value["data"]["streamPlaybackAccessToken"];
    let Some(value) = string(&token["value"]).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let Some(signature) = string(&token["signature"]).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    Ok(Some(PlaybackToken {
        value: value.to_owned(),
        signature: signature.to_owned(),
        info: parse_playback_info(value),
    }))
}

pub(super) const TWITCH_HELIX_USERS_URI: &[u8] = b"https://api.twitch.tv/helix/users\0";
pub(super) const TWITCH_HELIX_PAGE_SIZE: &[u8] = b"100";

pub(super) fn parse_helix_user_id_response(json: &[u8]) -> Result<*mut c_char, glib::Error> {
    let value = parse(json)?;
    Ok(value["data"]
        .as_array()
        .and_then(|users| users.first())
        .and_then(|user| string(&user["id"]))
        .filter(|id| !id.is_empty())
        .map_or(ptr::null_mut(), |id| dup_bytes(id.as_bytes())))
}

pub(super) unsafe fn parse_followed_channels_page(
    json: &[u8],
    channels: *mut GPtrArray,
) -> Result<*mut c_char, glib::Error> {
    unsafe {
        let value = parse(json)?;
        if let Some(data) = value["data"].as_array() {
            for user in data {
                let Some(login) = string(&user["broadcaster_login"]).filter(|s| !s.is_empty())
                else {
                    continue;
                };
                let display_name = string(&user["broadcaster_name"])
                    .filter(|s| !s.is_empty())
                    .unwrap_or(login);
                let channel = Box::new(TwitchFollowedChannel {
                    channel: dup_bytes(login.to_ascii_lowercase().as_bytes()),
                    display_name: dup_bytes(display_name.as_bytes()),
                });
                g_ptr_array_add(channels, Box::into_raw(channel).cast());
            }
        }
        if let Some(cursor) = string(&value["pagination"]["cursor"]).filter(|s| !s.is_empty()) {
            return Ok(dup_bytes(cursor.as_bytes()));
        }
        Ok(ptr::null_mut())
    }
}

pub(super) unsafe extern "C" fn compare_stream_previews_by_viewers(
    a: *const c_void,
    b: *const c_void,
) -> c_int {
    unsafe {
        let preview_a = *(a as *const *const TwitchStreamPreview);
        let preview_b = *(b as *const *const TwitchStreamPreview);

        if (*preview_a).viewer_count == (*preview_b).viewer_count {
            return g_ascii_strcasecmp((*preview_a).display_name, (*preview_b).display_name);
        }

        if (*preview_a).viewer_count > (*preview_b).viewer_count {
            -1
        } else {
            1
        }
    }
}

/// Follow pagination cursors until all followed channels are collected.
///
/// # Safety
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
/// - `cancel`: null or live `GCancellable`.
/// - `error`: null or writable, initially null `GError*` slot.
/// - Result owned; release with matching token/array free function.
pub unsafe fn twitch_stream_info_fetch_followed_channels(
    client_id: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    unsafe {
        let user_response = get_twitch_helix_request(
            TWITCH_HELIX_USERS_URI.as_ptr() as *const c_char,
            client_id,
            oauth_token,
            cancel,
            error,
        );
        if user_response.is_null() {
            return ptr::null_mut();
        }

        let user_id = parse_helix_user_id_response(CStr::from_ptr(user_response).to_bytes());
        g_free(user_response.cast());
        let user_id = match user_id {
            Ok(id) => id,
            Err(err) => {
                glib::ffi::g_propagate_error(error.cast(), err.into_glib_ptr());
                return ptr::null_mut();
            }
        };
        if user_id.is_null() {
            if error.is_null() || (*error).is_null() {
                g_set_error(
                    error,
                    g_io_error_quark(),
                    G_IO_ERROR_FAILED,
                    c"Twitch user could not be read from the token".as_ptr(),
                );
            }
            return ptr::null_mut();
        }

        let channels = g_ptr_array_new_with_free_func(Some(twitch_followed_channel_free_destroy));
        let escaped_user_id = g_uri_escape_string(user_id, ptr::null(), 1);
        let mut cursor: *mut c_char = ptr::null_mut();

        loop {
            let escaped_cursor = if cursor.is_null() {
                ptr::null_mut()
            } else {
                g_uri_escape_string(cursor, ptr::null(), 1)
            };
            let mut request_uri = Vec::new();
            request_uri
                .extend_from_slice(b"https://api.twitch.tv/helix/channels/followed?user_id=");
            request_uri.extend_from_slice(CStr::from_ptr(escaped_user_id).to_bytes());
            request_uri.extend_from_slice(b"&first=");
            request_uri.extend_from_slice(TWITCH_HELIX_PAGE_SIZE);
            if !escaped_cursor.is_null() {
                request_uri.extend_from_slice(b"&after=");
                request_uri.extend_from_slice(CStr::from_ptr(escaped_cursor).to_bytes());
            }
            request_uri.push(0);
            g_free(escaped_cursor as *mut c_void);

            let page_response = get_twitch_helix_request(
                request_uri.as_ptr() as *const c_char,
                client_id,
                oauth_token,
                cancel,
                error,
            );
            if page_response.is_null() {
                g_free(cursor as *mut c_void);
                g_free(escaped_user_id as *mut c_void);
                g_free(user_id as *mut c_void);
                g_ptr_array_unref(channels);
                return ptr::null_mut();
            }

            let next_cursor =
                parse_followed_channels_page(CStr::from_ptr(page_response).to_bytes(), channels);
            g_free(page_response.cast());
            let next_cursor = match next_cursor {
                Ok(cursor) => cursor,
                Err(err) => {
                    glib::ffi::g_propagate_error(error.cast(), err.into_glib_ptr());
                    g_free(cursor.cast());
                    g_free(escaped_user_id.cast());
                    g_free(user_id.cast());
                    g_ptr_array_unref(channels);
                    return ptr::null_mut();
                }
            };

            g_free(cursor as *mut c_void);
            cursor = next_cursor;
            if cursor.is_null() || (!cancel.is_null() && g_cancellable_is_cancelled(cancel) != 0) {
                break;
            }
        }

        g_free(cursor as *mut c_void);
        g_free(escaped_user_id as *mut c_void);
        g_free(user_id as *mut c_void);
        channels
    }
}
