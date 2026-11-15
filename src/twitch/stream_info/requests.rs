use super::ffi::{GCancellable, GError};
use super::twitch_stream_info_error_quark;
use crate::glib_strings::{dup_bytes, is_nonempty};
use crate::twitch::http;
use gtk4::{
    gio,
    glib::{self, translate::*},
};
use serde_json::json;
use std::ffi::{CStr, c_char, c_int, c_uint};
use std::ptr;

pub(super) const TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED: c_int = 0;
pub(super) const TWITCH_GQL_QUERY: &str =
    "query($login:String!){user(login:$login){stream{title viewersCount createdAt game{name}}}}";
pub(super) const TWITCH_GQL_LIVE_CHANNELS_QUERY: &str = "query($logins:[String!]!){users(logins:$logins){login displayName profileImageURL(width:70) stream{title viewersCount createdAt previewImageURL(width:320,height:180) game{name}}}}";
pub(super) const TWITCH_GQL_PLAYBACK_ACCESS_TOKEN_QUERY: &str = "query($login:String!){streamPlaybackAccessToken(channelName:$login,params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:\"site\"}){value signature}}";

pub(super) unsafe fn build_gql_login_body(query: &str, channel: *const c_char) -> *mut c_char {
    unsafe {
        let login = if channel.is_null() {
            std::borrow::Cow::Borrowed("")
        } else {
            CStr::from_ptr(channel).to_string_lossy()
        };
        let body = json!({"query": query, "variables": {"login": login}});
        dup_bytes(body.to_string().as_bytes())
    }
}

pub(super) unsafe fn build_stream_title_request_body(channel: *const c_char) -> *mut c_char {
    unsafe { build_gql_login_body(TWITCH_GQL_QUERY, channel) }
}

pub(super) unsafe fn build_live_channels_request_body(
    channels: *const *const c_char,
    channel_count: c_uint,
) -> *mut c_char {
    unsafe {
        let mut logins = Vec::new();
        for i in 0..channel_count {
            let channel = *channels.add(i as usize);
            if is_nonempty(channel) {
                logins.push(CStr::from_ptr(channel).to_string_lossy());
            }
        }
        let body =
            json!({"query": TWITCH_GQL_LIVE_CHANNELS_QUERY, "variables": {"logins": logins}});
        dup_bytes(body.to_string().as_bytes())
    }
}

pub(super) unsafe fn build_playback_access_token_request_body(
    channel: *const c_char,
) -> *mut c_char {
    unsafe { build_gql_login_body(TWITCH_GQL_PLAYBACK_ACCESS_TOKEN_QUERY, channel) }
}

fn http_error(status: u32, hls: bool) -> glib::Error {
    if status == 401 && !hls {
        glib::Error::with_domain(
            unsafe { from_glib(twitch_stream_info_error_quark()) },
            TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED,
            "Twitch access token is expired or invalid",
        )
    } else {
        glib::Error::new(
            gio::IOErrorEnum::Failed,
            &format!(
                "Twitch{} returned HTTP {status}",
                if hls { " HLS" } else { "" }
            ),
        )
    }
}

fn sanitize_oauth_token(token: &str) -> &str {
    ["oauth:", "Bearer ", "OAuth "]
        .iter()
        .find_map(|prefix| token.strip_prefix(prefix))
        .unwrap_or(token)
}

pub(super) unsafe fn post_twitch_gql_request(
    body: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut c_char {
    unsafe {
        let token = if !is_nonempty(oauth_token) {
            String::new()
        } else {
            format!(
                "OAuth {}",
                sanitize_oauth_token(&CStr::from_ptr(oauth_token).to_string_lossy())
            )
        };
        let mut headers = vec![
            ("Client-ID", "kimne78kx3ncx6brgo4mv6wki5h1ko"),
            ("Accept", "application/json"),
        ];
        if !token.is_empty() {
            headers.push(("Authorization", &token));
        }
        let cancel: Option<gio::Cancellable> = from_glib_none(cancel.cast());
        let response = http::send(
            "POST",
            "https://gql.twitch.tv/gql",
            &headers,
            Some(("application/json", CStr::from_ptr(body).to_bytes())),
            cancel.as_ref(),
        );
        match response.and_then(|response| {
            if (200..300).contains(&response.status) {
                Ok(response.body)
            } else {
                Err(http_error(response.status, false))
            }
        }) {
            Ok(bytes) => dup_bytes(&bytes),
            Err(err) => {
                glib::ffi::g_propagate_error(error.cast(), err.into_glib_ptr());
                ptr::null_mut()
            }
        }
    }
}

pub(super) unsafe fn get_twitch_helix_request(
    uri: *const c_char,
    client_id: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut c_char {
    unsafe {
        let oauth_token = if oauth_token.is_null() {
            std::borrow::Cow::Borrowed("")
        } else {
            CStr::from_ptr(oauth_token).to_string_lossy()
        };
        let token = format!("Bearer {}", sanitize_oauth_token(&oauth_token));
        let client_id = CStr::from_ptr(client_id).to_string_lossy();
        let uri = CStr::from_ptr(uri).to_string_lossy();
        let cancel: Option<gio::Cancellable> = from_glib_none(cancel.cast());
        let response = http::send(
            "GET",
            &uri,
            &[
                ("Client-ID", &client_id),
                ("Authorization", &token),
                ("Accept", "application/json"),
            ],
            None,
            cancel.as_ref(),
        );
        match response.and_then(|response| {
            if (200..300).contains(&response.status) {
                Ok(response.body)
            } else {
                Err(http_error(response.status, false))
            }
        }) {
            Ok(bytes) => dup_bytes(&bytes),
            Err(err) => {
                glib::ffi::g_propagate_error(error.cast(), err.into_glib_ptr());
                ptr::null_mut()
            }
        }
    }
}

pub(super) unsafe fn get_twitch_hls_playlist(
    uri: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut c_char {
    unsafe {
        let uri = CStr::from_ptr(uri).to_string_lossy();
        let cancel: Option<gio::Cancellable> = from_glib_none(cancel.cast());
        let response = http::send(
            "GET",
            &uri,
            &[
                ("Client-ID", "kimne78kx3ncx6brgo4mv6wki5h1ko"),
                (
                    "Accept",
                    "application/x-mpegURL, application/vnd.apple.mpegurl, */*",
                ),
            ],
            None,
            cancel.as_ref(),
        );
        match response.and_then(|response| {
            if (200..300).contains(&response.status) {
                Ok(response.body)
            } else {
                Err(http_error(response.status, true))
            }
        }) {
            Ok(bytes) => dup_bytes(&bytes),
            Err(err) => {
                glib::ffi::g_propagate_error(error.cast(), err.into_glib_ptr());
                ptr::null_mut()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_accept_existing_authorization_prefixes() {
        for token in ["abc", "oauth:abc", "Bearer abc", "OAuth abc"] {
            assert_eq!(sanitize_oauth_token(token), "abc");
        }
    }

    #[test]
    fn unauthorized_errors_preserve_the_refresh_signal() {
        let err = http_error(401, false);
        assert_eq!(err.domain(), unsafe {
            from_glib(twitch_stream_info_error_quark())
        });
        assert_eq!(err.code(), TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED);
        assert!(http_error(503, false).message().contains("503"));
    }
}
