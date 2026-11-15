use super::ffi::{
    G_IO_ERROR_FAILED, GCancellable, GError, GPtrArray, GTask, g_free, g_io_error_quark,
    g_ptr_array_add, g_ptr_array_new_with_free_func, g_ptr_array_set_free_func, g_ptr_array_sort,
    g_ptr_array_unref, g_random_int_range, g_set_error, g_task_return_error, g_task_return_pointer,
    g_uri_escape_string,
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
    g_ptr_array_unref_destroy, twitch_current_stream_free_destroy,
    twitch_stream_preview_free_destroy, twitch_stream_qualities_result_free,
};
use super::{
    FetchCurrentStreamData, FetchFollowedChannelsData, FetchLiveChannelsData,
    FetchStreamQualitiesData, TwitchCurrentStream, TwitchStreamQualitiesResult,
    twitch_playback_info_empty,
};
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

        let stream = parse_current_stream_response(
            response,
            CStr::from_ptr(response).to_bytes().len(),
            error,
        );
        g_free(response as *mut c_void);
        stream
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

        let mut token: *mut c_char = ptr::null_mut();
        let mut signature: *mut c_char = ptr::null_mut();
        let mut playback_info = twitch_playback_info_empty();
        if parse_playback_access_token_response(
            response,
            CStr::from_ptr(response).to_bytes().len(),
            &mut token,
            &mut signature,
            &mut playback_info,
            error,
        ) == 0
        {
            g_free(response as *mut c_void);
            if error.is_null() || (*error).is_null() {
                g_set_error(
                    error,
                    g_io_error_quark(),
                    G_IO_ERROR_FAILED,
                    c"Twitch playback token is unavailable".as_ptr(),
                );
            }
            return ptr::null_mut();
        }
        g_free(response as *mut c_void);

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
        let qualities = parse_stream_qualities_playlist(playlist, error);
        g_free(playlist as *mut c_void);
        if qualities.is_null() {
            return ptr::null_mut();
        }

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
) -> *mut GPtrArray {
    unsafe {
        let all_previews = g_ptr_array_new_with_free_func(Some(twitch_stream_preview_free_destroy));
        let mut offset = 0;

        while offset < (*data).channel_count {
            let chunk_count = std::cmp::min(TWITCH_GQL_MAX_LOGINS, (*data).channel_count - offset);
            let body = build_live_channels_request_body(
                (*data).channels.add(offset as usize) as *const *const c_char,
                chunk_count,
            );
            let response = post_twitch_gql_request(body, ptr::null(), cancel, error);
            g_free(body as *mut c_void);
            if response.is_null() {
                g_ptr_array_unref(all_previews);
                return ptr::null_mut();
            }

            let chunk_previews = parse_live_channels_response(
                response,
                CStr::from_ptr(response).to_bytes().len(),
                error,
            );
            g_free(response as *mut c_void);
            if chunk_previews.is_null() {
                g_ptr_array_unref(all_previews);
                return ptr::null_mut();
            }

            for i in 0..(*chunk_previews).len {
                g_ptr_array_add(all_previews, *(*chunk_previews).pdata.add(i as usize));
            }
            g_ptr_array_set_free_func(chunk_previews, None);
            g_ptr_array_unref(chunk_previews);
            offset += TWITCH_GQL_MAX_LOGINS;
        }

        g_ptr_array_sort(all_previews, Some(compare_stream_previews_by_viewers));
        all_previews
    }
}

pub(super) unsafe extern "C" fn fetch_current_stream_data_free(data: *mut c_void) {
    unsafe {
        let data = data as *mut FetchCurrentStreamData;
        if data.is_null() {
            return;
        }
        g_free((*data).channel as *mut c_void);
        drop(Box::from_raw(data));
    }
}

pub(super) unsafe extern "C" fn fetch_stream_qualities_data_free(data: *mut c_void) {
    unsafe {
        let data = data as *mut FetchStreamQualitiesData;
        if data.is_null() {
            return;
        }
        g_free((*data).channel as *mut c_void);
        g_free((*data).oauth_token as *mut c_void);
        drop(Box::from_raw(data));
    }
}

pub(super) unsafe extern "C" fn fetch_live_channels_data_free(data: *mut c_void) {
    unsafe {
        let data = data as *mut FetchLiveChannelsData;
        if data.is_null() {
            return;
        }
        if !(*data).channels.is_null() {
            for i in 0..(*data).channel_count {
                g_free(*(*data).channels.add(i as usize) as *mut c_void);
            }
            g_free((*data).channels as *mut c_void);
        }
        drop(Box::from_raw(data));
    }
}

pub(super) unsafe extern "C" fn fetch_followed_channels_data_free(data: *mut c_void) {
    unsafe {
        let data = data as *mut FetchFollowedChannelsData;
        if data.is_null() {
            return;
        }
        g_free((*data).client_id as *mut c_void);
        g_free((*data).oauth_token as *mut c_void);
        drop(Box::from_raw(data));
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
        let stream = fetch_current_stream((*data).channel, cancel, &mut error);
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
        let result =
            fetch_stream_qualities((*data).channel, (*data).oauth_token, cancel, &mut error);
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
            previews as *mut c_void,
            Some(g_ptr_array_unref_destroy),
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
            (*data).client_id,
            (*data).oauth_token,
            cancel,
            &mut error,
        );
        if !error.is_null() {
            g_task_return_error(task, error);
            return;
        }
        g_task_return_pointer(
            task,
            channels as *mut c_void,
            Some(g_ptr_array_unref_destroy),
        );
    }
}
