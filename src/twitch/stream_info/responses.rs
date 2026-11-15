use super::ffi::{
    G_IO_ERROR_FAILED, GCancellable, GError, g_cancellable_is_cancelled, g_free, g_io_error_quark,
    g_set_error, g_uri_escape_string,
};
use super::requests::get_twitch_helix_request;
use super::{
    TwitchCurrentStream, TwitchFollowedChannel, TwitchPlaybackInfo, TwitchStreamPreview,
    twitch_playback_info_empty,
};
use crate::glib_strings::dup_bytes;
use crate::twitch::json::{owned_string, parse, string, uint};
use gtk4::glib::{self, translate::IntoGlibPtr};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
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
        title: owned_string(&stream["title"]),
        started_at: owned_string(&stream["createdAt"]),
        category_name: owned_string(&stream["game"]["name"]),
        viewer_count: uint(&stream["viewersCount"]),
    })))
}

pub(super) fn parse_live_channels_response(
    json: &[u8],
) -> Result<Vec<TwitchStreamPreview>, glib::Error> {
    let value = parse(json)?;
    let mut previews = Vec::new();
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
        let preview = TwitchStreamPreview {
            channel: CString::new(login.to_ascii_lowercase()).unwrap(),
            display_name: CString::new(display_name).unwrap(),
            title: owned_string(&stream["title"]),
            avatar_url: owned_string(&user["profileImageURL"]),
            preview_url: owned_string(&stream["previewImageURL"]),
            started_at: owned_string(&stream["createdAt"]),
            category_name: owned_string(&stream["game"]["name"]),
            viewer_count: uint(&stream["viewersCount"]),
        };
        previews.push(preview);
    }
    previews.sort_by(compare_stream_previews_by_viewers);
    Ok(previews)
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

pub(super) fn parse_followed_channels_page(
    json: &[u8],
    channels: &mut Vec<TwitchFollowedChannel>,
) -> Result<*mut c_char, glib::Error> {
    let value = parse(json)?;
    if let Some(data) = value["data"].as_array() {
        for user in data {
            let Some(login) = string(&user["broadcaster_login"]).filter(|s| !s.is_empty()) else {
                continue;
            };
            let display_name = string(&user["broadcaster_name"])
                .filter(|s| !s.is_empty())
                .unwrap_or(login);
            let channel = TwitchFollowedChannel {
                channel: CString::new(login.to_ascii_lowercase()).unwrap(),
                display_name: CString::new(display_name).unwrap(),
            };
            channels.push(channel);
        }
    }
    if let Some(cursor) = string(&value["pagination"]["cursor"]).filter(|s| !s.is_empty()) {
        return Ok(dup_bytes(cursor.as_bytes()));
    }
    Ok(ptr::null_mut())
}

pub(super) fn compare_stream_previews_by_viewers(
    a: &TwitchStreamPreview,
    b: &TwitchStreamPreview,
) -> std::cmp::Ordering {
    b.viewer_count.cmp(&a.viewer_count).then_with(|| {
        a.display_name
            .as_bytes()
            .to_ascii_lowercase()
            .cmp(&b.display_name.as_bytes().to_ascii_lowercase())
    })
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
) -> Option<Vec<TwitchFollowedChannel>> {
    unsafe {
        let user_response = get_twitch_helix_request(
            TWITCH_HELIX_USERS_URI.as_ptr() as *const c_char,
            client_id,
            oauth_token,
            cancel,
            error,
        );
        if user_response.is_null() {
            return None;
        }

        let user_id = parse_helix_user_id_response(CStr::from_ptr(user_response).to_bytes());
        g_free(user_response.cast());
        let user_id = match user_id {
            Ok(id) => id,
            Err(err) => {
                glib::ffi::g_propagate_error(error.cast(), err.into_glib_ptr());
                return None;
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
            return None;
        }

        let mut channels = Vec::new();
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
                return None;
            }

            let next_cursor = parse_followed_channels_page(
                CStr::from_ptr(page_response).to_bytes(),
                &mut channels,
            );
            g_free(page_response.cast());
            let next_cursor = match next_cursor {
                Ok(cursor) => cursor,
                Err(err) => {
                    glib::ffi::g_propagate_error(error.cast(), err.into_glib_ptr());
                    g_free(cursor.cast());
                    g_free(escaped_user_id.cast());
                    g_free(user_id.cast());
                    return None;
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
        Some(channels)
    }
}
