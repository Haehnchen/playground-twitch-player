use super::ffi::{
    G_IO_ERROR_FAILED, GCancellable, GError, GTask, g_free, g_io_error_quark, g_random_int_range,
    g_set_error, g_task_return_error, g_task_return_pointer, g_uri_escape_string,
};
use super::playlist::parse_stream_qualities_playlist;
use super::requests::{
    build_live_channels_request_body, build_playback_access_token_request_body,
    build_stream_title_request_body, get_twitch_hls_playlist, post_twitch_gql_request,
};
use super::responses::{
    compare_stream_previews_by_viewers, parse_current_stream_response,
    parse_live_channels_response, parse_playback_access_token_response,
    twitch_stream_info_fetch_followed_channels,
};
use super::utils::{
    drop_box, twitch_current_stream_free_destroy, twitch_stream_qualities_result_free,
};
use super::{
    FetchCurrentStreamData, FetchFollowedChannelsData, FetchLiveChannelsData,
    FetchStreamQualitiesData, TwitchCurrentStream, TwitchFollowedChannel, TwitchStreamPreview,
    TwitchStreamQualitiesResult,
};
use crate::glib_strings::dup_bytes;
use gtk4::glib::{self, translate::IntoGlibPtr};
use std::ffi::{CStr, c_char, c_uint, c_void};
use std::ptr;

pub(super) const TWITCH_GQL_MAX_LOGINS: c_uint = 100;

pub(super) unsafe fn fetch_current_stream(
    channel: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut TwitchCurrentStream {
    unsafe {
        let body = build_stream_title_request_body(channel);
        let response = post_twitch_gql_request(body, ptr::null(), cancel, error);
        g_free(body as *mut c_void);
        if response.is_null() {
            return ptr::null_mut();
        }

        let stream = parse_current_stream_response(CStr::from_ptr(response).to_bytes());
        g_free(response.cast());
        match stream {
            Ok(stream) => stream,
            Err(err) => {
                glib::ffi::g_propagate_error(error.cast(), err.into_glib_ptr());
                ptr::null_mut()
            }
        }
    }
}

pub(super) unsafe fn fetch_stream_qualities(
    channel: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut TwitchStreamQualitiesResult {
    unsafe {
        let body = build_playback_access_token_request_body(channel);
        let response = post_twitch_gql_request(body, oauth_token, cancel, error);
        g_free(body as *mut c_void);
        if response.is_null() {
            return ptr::null_mut();
        }

        let token = parse_playback_access_token_response(CStr::from_ptr(response).to_bytes());
        g_free(response.cast());
        let token = match token {
            Ok(Some(token)) => token,
            Ok(None) => {
                g_set_error(
                    error,
                    g_io_error_quark(),
                    G_IO_ERROR_FAILED,
                    c"Twitch playback token is unavailable".as_ptr(),
                );
                return ptr::null_mut();
            }
            Err(err) => {
                glib::ffi::g_propagate_error(error.cast(), err.into_glib_ptr());
                return ptr::null_mut();
            }
        };
        let playback_info = token.info;
        let signature = dup_bytes(token.signature.as_bytes());
        let token = dup_bytes(token.value.as_bytes());

        let escaped_channel = g_uri_escape_string(channel, ptr::null(), 0);
        let escaped_token = g_uri_escape_string(token, ptr::null(), 0);
        let escaped_signature = g_uri_escape_string(signature, ptr::null(), 0);
        let mut uri = Vec::new();
        uri.extend_from_slice(b"https://usher.ttvnw.net/api/channel/hls/");
        uri.extend_from_slice(CStr::from_ptr(escaped_channel).to_bytes());
        uri.extend_from_slice(b".m3u8?allow_audio_only=true&allow_source=true&fast_bread=true&p=");
        uri.extend_from_slice(format!("{}", g_random_int_range(100000, 999999)).as_bytes());
        uri.extend_from_slice(b"&player=twitchweb&sig=");
        uri.extend_from_slice(CStr::from_ptr(escaped_signature).to_bytes());
        uri.extend_from_slice(b"&token=");
        uri.extend_from_slice(CStr::from_ptr(escaped_token).to_bytes());
        uri.extend_from_slice(b"&type=any\0");

        g_free(token as *mut c_void);
        g_free(signature as *mut c_void);
        g_free(escaped_channel as *mut c_void);
        g_free(escaped_token as *mut c_void);
        g_free(escaped_signature as *mut c_void);

        let playlist = get_twitch_hls_playlist(uri.as_ptr() as *const c_char, cancel, error);
        if playlist.is_null() {
            return ptr::null_mut();
        }
        let qualities = parse_stream_qualities_playlist(CStr::from_ptr(playlist).to_bytes());
        g_free(playlist as *mut c_void);
        let qualities = match qualities {
            Ok(qualities) => qualities,
            Err(err) => {
                glib::ffi::g_propagate_error(error, err.into_glib_ptr());
                return ptr::null_mut();
            }
        };

        Box::into_raw(Box::new(TwitchStreamQualitiesResult {
            qualities,
            playback_info,
        }))
    }
}

// Fetch channel previews in batches within the Twitch query limit.
pub(super) unsafe fn fetch_live_channels(
    data: *mut FetchLiveChannelsData,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> Option<Vec<TwitchStreamPreview>> {
    unsafe {
        let mut all_previews = Vec::new();
        for channels in (*data).channels.chunks(TWITCH_GQL_MAX_LOGINS as usize) {
            let pointers: Vec<_> = channels.iter().map(|channel| channel.as_ptr()).collect();
            let body =
                build_live_channels_request_body(pointers.as_ptr(), pointers.len() as c_uint);
            let response = post_twitch_gql_request(body, ptr::null(), cancel, error);
            g_free(body as *mut c_void);
            if response.is_null() {
                return None;
            }

            let chunk_previews = parse_live_channels_response(CStr::from_ptr(response).to_bytes());
            g_free(response.cast());
            let chunk_previews = match chunk_previews {
                Ok(previews) => previews,
                Err(err) => {
                    glib::ffi::g_propagate_error(error.cast(), err.into_glib_ptr());
                    return None;
                }
            };

            all_previews.extend(chunk_previews);
        }

        all_previews.sort_by(compare_stream_previews_by_viewers);
        Some(all_previews)
    }
}

pub(super) unsafe extern "C" fn fetch_current_stream_data_free(data: *mut c_void) {
    unsafe {
        drop_box::<FetchCurrentStreamData>(data);
    }
}

pub(super) unsafe extern "C" fn fetch_stream_qualities_data_free(data: *mut c_void) {
    unsafe {
        drop_box::<FetchStreamQualitiesData>(data);
    }
}

pub(super) unsafe extern "C" fn fetch_live_channels_data_free(data: *mut c_void) {
    unsafe {
        drop_box::<FetchLiveChannelsData>(data);
    }
}

pub(super) unsafe extern "C" fn fetch_followed_channels_data_free(data: *mut c_void) {
    unsafe {
        drop_box::<FetchFollowedChannelsData>(data);
    }
}

pub(super) unsafe extern "C" fn fetch_current_stream_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    unsafe {
        let data = task_data as *mut FetchCurrentStreamData;
        let mut error: *mut GError = ptr::null_mut();
        let stream = fetch_current_stream((*data).channel.as_ptr(), cancel, &mut error);
        if !error.is_null() {
            g_task_return_error(task, error);
            return;
        }
        g_task_return_pointer(
            task,
            stream as *mut c_void,
            Some(twitch_current_stream_free_destroy),
        );
    }
}

pub(super) unsafe extern "C" fn fetch_stream_qualities_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    unsafe {
        let data = task_data as *mut FetchStreamQualitiesData;
        let mut error: *mut GError = ptr::null_mut();
        let result = fetch_stream_qualities(
            (*data).channel.as_ptr(),
            (*data).oauth_token.as_ptr(),
            cancel,
            &mut error,
        );
        if !error.is_null() {
            if !result.is_null() {
                twitch_stream_qualities_result_free(result as *mut c_void);
            }
            g_task_return_error(task, error);
            return;
        }
        g_task_return_pointer(
            task,
            result as *mut c_void,
            Some(twitch_stream_qualities_result_free),
        );
    }
}

pub(super) unsafe extern "C" fn fetch_live_channels_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    unsafe {
        let data = task_data as *mut FetchLiveChannelsData;
        let mut error: *mut GError = ptr::null_mut();
        let previews = fetch_live_channels(data, cancel, &mut error);
        if !error.is_null() {
            g_task_return_error(task, error);
            return;
        }
        g_task_return_pointer(
            task,
            Box::into_raw(Box::new(previews.unwrap_or_default())).cast(),
            Some(drop_box::<Vec<TwitchStreamPreview>>),
        );
    }
}

pub(super) unsafe extern "C" fn fetch_followed_channels_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    unsafe {
        let data = task_data as *mut FetchFollowedChannelsData;
        let mut error: *mut GError = ptr::null_mut();
        let channels = twitch_stream_info_fetch_followed_channels(
            (*data).client_id.as_ptr(),
            (*data).oauth_token.as_ptr(),
            cancel,
            &mut error,
        );
        if !error.is_null() {
            g_task_return_error(task, error);
            return;
        }
        g_task_return_pointer(
            task,
            Box::into_raw(Box::new(channels.unwrap_or_default())).cast(),
            Some(drop_box::<Vec<TwitchFollowedChannel>>),
        );
    }
}
