use super::ffi::{
    G_IO_ERROR_FAILED, GBytes, GCancellable, GError, g_bytes_get_data, g_bytes_new_static,
    g_bytes_unref, g_free, g_io_error_quark, g_object_set, g_object_unref, g_set_error, g_strdup,
    g_strndup, soup_message_get_request_headers, soup_message_get_status,
    soup_message_headers_append, soup_message_new, soup_message_set_request_body_from_bytes,
    soup_session_new, soup_session_send_and_read,
};
use super::twitch_stream_info_error_quark;
use super::utils::{json_escape_bytes, json_string_from_c};
use crate::glib_strings::{dup_bytes, is_nonempty};
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::ptr;
pub(super) const TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED: c_int = 0;
pub(super) const TWITCH_GQL_URI: &[u8] = b"https://gql.twitch.tv/gql\0";
pub(super) const TWITCH_GQL_CLIENT_ID: &[u8] = b"kimne78kx3ncx6brgo4mv6wki5h1ko\0";
pub(super) const TWITCH_GQL_QUERY: &str =
    "query($login:String!){user(login:$login){stream{title viewersCount createdAt game{name}}}}";
pub(super) const TWITCH_GQL_LIVE_CHANNELS_QUERY: &str = "query($logins:[String!]!){users(logins:$logins){login displayName profileImageURL(width:70) stream{title viewersCount createdAt previewImageURL(width:320,height:180) game{name}}}}";
pub(super) const TWITCH_GQL_PLAYBACK_ACCESS_TOKEN_QUERY: &str = "query($login:String!){streamPlaybackAccessToken(channelName:$login,params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:\"site\"}){value signature}}";

pub(super) unsafe fn build_gql_login_body(query: &str, channel: *const c_char) -> *mut c_char {
    unsafe {
        let login = json_string_from_c(channel);
        let mut body = Vec::new();
        body.extend_from_slice(br#"{"query":""#);
        body.extend_from_slice(json_escape_bytes(query.as_bytes()).as_slice());
        body.extend_from_slice(br#"","variables":{"login":""#);
        body.extend_from_slice(&login);
        body.extend_from_slice(br#""}}"#);
        dup_bytes(&body)
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
        let mut body = Vec::new();
        body.extend_from_slice(br#"{"query":""#);
        body.extend_from_slice(
            json_escape_bytes(TWITCH_GQL_LIVE_CHANNELS_QUERY.as_bytes()).as_slice(),
        );
        body.extend_from_slice(br#"","variables":{"logins":["#);
        let mut first = true;

        for i in 0..channel_count {
            let channel = *channels.add(i as usize);
            if !is_nonempty(channel) {
                continue;
            }

            if !first {
                body.push(b',');
            }
            first = false;
            body.push(b'"');
            body.extend_from_slice(&json_string_from_c(channel));
            body.push(b'"');
        }

        body.extend_from_slice(br#"]}}"#);
        dup_bytes(&body)
    }
}

pub(super) unsafe fn build_playback_access_token_request_body(
    channel: *const c_char,
) -> *mut c_char {
    unsafe { build_gql_login_body(TWITCH_GQL_PLAYBACK_ACCESS_TOKEN_QUERY, channel) }
}

pub(super) unsafe fn response_bytes_to_string(response: *mut GBytes) -> *mut c_char {
    unsafe {
        let mut response_size = 0usize;
        let response_data = g_bytes_get_data(response, &mut response_size);
        g_strndup(response_data as *const c_char, response_size)
    }
}

pub(super) unsafe fn set_twitch_http_error(error: *mut *mut GError, status: c_uint) {
    unsafe {
        if status == 401 {
            g_set_error(
                error,
                twitch_stream_info_error_quark(),
                TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED,
                c"Twitch access token is expired or invalid".as_ptr(),
            );
        } else {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"Twitch returned HTTP %u".as_ptr(),
                status,
            );
        }
    }
}

pub(super) unsafe fn post_twitch_gql_request(
    body: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut c_char {
    unsafe {
        let session = soup_session_new();
        let message = soup_message_new(c"POST".as_ptr(), TWITCH_GQL_URI.as_ptr() as *const c_char);
        let body_len = CStr::from_ptr(body).to_bytes().len();
        let body_bytes = g_bytes_new_static(body as *const c_void, body_len);

        g_object_set(
            session as *mut c_void,
            c"timeout".as_ptr(),
            15,
            ptr::null::<c_char>(),
        );
        let request_headers = soup_message_get_request_headers(message);
        soup_message_headers_append(
            request_headers,
            c"Client-ID".as_ptr(),
            TWITCH_GQL_CLIENT_ID.as_ptr() as *const c_char,
        );
        soup_message_headers_append(
            request_headers,
            c"Accept".as_ptr(),
            c"application/json".as_ptr(),
        );
        if is_nonempty(oauth_token) {
            let token = sanitize_oauth_token(oauth_token);
            let mut auth = Vec::new();
            auth.extend_from_slice(b"OAuth ");
            auth.extend_from_slice(CStr::from_ptr(token).to_bytes());
            auth.push(0);
            soup_message_headers_append(
                request_headers,
                c"Authorization".as_ptr(),
                auth.as_ptr() as *const c_char,
            );
            g_free(token as *mut c_void);
        }
        soup_message_set_request_body_from_bytes(message, c"application/json".as_ptr(), body_bytes);
        g_bytes_unref(body_bytes);

        let response = soup_session_send_and_read(session, message, cancel, error);
        if response.is_null() {
            g_object_unref(message as *mut c_void);
            g_object_unref(session as *mut c_void);
            return ptr::null_mut();
        }

        let status = soup_message_get_status(message);
        if !(200..300).contains(&status) {
            set_twitch_http_error(error, status);
            g_bytes_unref(response);
            g_object_unref(message as *mut c_void);
            g_object_unref(session as *mut c_void);
            return ptr::null_mut();
        }

        let result = response_bytes_to_string(response);
        g_bytes_unref(response);
        g_object_unref(message as *mut c_void);
        g_object_unref(session as *mut c_void);
        result
    }
}

pub(super) unsafe fn sanitize_oauth_token(oauth_token: *const c_char) -> *mut c_char {
    unsafe {
        if oauth_token.is_null() {
            return g_strdup(c"".as_ptr());
        }

        let bytes = CStr::from_ptr(oauth_token).to_bytes();
        if bytes.starts_with(b"oauth:") {
            return dup_bytes(&bytes[6..]);
        }
        if bytes.starts_with(b"Bearer ") {
            return dup_bytes(&bytes[7..]);
        }
        if bytes.starts_with(b"OAuth ") {
            return dup_bytes(&bytes[6..]);
        }

        g_strdup(oauth_token)
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
        let session = soup_session_new();
        let message = soup_message_new(c"GET".as_ptr(), uri);
        let token = sanitize_oauth_token(oauth_token);
        let mut auth = Vec::new();
        auth.extend_from_slice(b"Bearer ");
        auth.extend_from_slice(CStr::from_ptr(token).to_bytes());
        auth.push(0);

        g_object_set(
            session as *mut c_void,
            c"timeout".as_ptr(),
            15,
            ptr::null::<c_char>(),
        );
        let request_headers = soup_message_get_request_headers(message);
        soup_message_headers_append(request_headers, c"Client-ID".as_ptr(), client_id);
        soup_message_headers_append(
            request_headers,
            c"Authorization".as_ptr(),
            auth.as_ptr() as *const c_char,
        );
        soup_message_headers_append(
            request_headers,
            c"Accept".as_ptr(),
            c"application/json".as_ptr(),
        );

        let response = soup_session_send_and_read(session, message, cancel, error);
        g_free(token as *mut c_void);
        if response.is_null() {
            g_object_unref(message as *mut c_void);
            g_object_unref(session as *mut c_void);
            return ptr::null_mut();
        }

        let status = soup_message_get_status(message);
        if !(200..300).contains(&status) {
            set_twitch_http_error(error, status);
            g_bytes_unref(response);
            g_object_unref(message as *mut c_void);
            g_object_unref(session as *mut c_void);
            return ptr::null_mut();
        }

        let result = response_bytes_to_string(response);
        g_bytes_unref(response);
        g_object_unref(message as *mut c_void);
        g_object_unref(session as *mut c_void);
        result
    }
}

pub(super) unsafe fn get_twitch_hls_playlist(
    uri: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut c_char {
    unsafe {
        let session = soup_session_new();
        let message = soup_message_new(c"GET".as_ptr(), uri);

        g_object_set(
            session as *mut c_void,
            c"timeout".as_ptr(),
            15,
            ptr::null::<c_char>(),
        );
        let request_headers = soup_message_get_request_headers(message);
        soup_message_headers_append(
            request_headers,
            c"Client-ID".as_ptr(),
            TWITCH_GQL_CLIENT_ID.as_ptr() as *const c_char,
        );
        soup_message_headers_append(
            request_headers,
            c"Accept".as_ptr(),
            c"application/x-mpegURL, application/vnd.apple.mpegurl, */*".as_ptr(),
        );

        let response = soup_session_send_and_read(session, message, cancel, error);
        if response.is_null() {
            g_object_unref(message as *mut c_void);
            g_object_unref(session as *mut c_void);
            return ptr::null_mut();
        }

        let status = soup_message_get_status(message);
        if !(200..300).contains(&status) {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"Twitch HLS returned HTTP %u".as_ptr(),
                status,
            );
            g_bytes_unref(response);
            g_object_unref(message as *mut c_void);
            g_object_unref(session as *mut c_void);
            return ptr::null_mut();
        }

        let result = response_bytes_to_string(response);
        g_bytes_unref(response);
        g_object_unref(message as *mut c_void);
        g_object_unref(session as *mut c_void);
        result
    }
}
