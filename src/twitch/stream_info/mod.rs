mod fetch;
mod ffi;
mod playlist;
mod requests;
mod responses;
#[cfg(test)]
mod tests;
mod utils;

pub use self::ffi::{GAsyncResult, GCancellable, GDateTime, GError, GTask};
pub use self::responses::twitch_stream_info_fetch_followed_channels;
pub use self::utils::twitch_current_stream_free;
pub use self::utils::{
    twitch_stream_info_format_current_stream_metadata,
    twitch_stream_info_format_current_stream_title, twitch_stream_info_format_live_duration,
    twitch_stream_info_format_viewer_count,
};

use self::fetch::{
    fetch_current_stream_data_free, fetch_current_stream_worker, fetch_followed_channels_data_free,
    fetch_followed_channels_worker, fetch_live_channels_data_free, fetch_live_channels_worker,
    fetch_stream_qualities_data_free, fetch_stream_qualities_worker,
};
use self::ffi::{
    g_object_unref, g_quark_from_static_string, g_task_is_valid, g_task_new,
    g_task_propagate_pointer, g_task_run_in_thread, g_task_set_task_data,
};
use crate::glib_strings::is_nonempty;
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_uint, c_void};
use std::ptr;

#[derive(Clone, Debug)]
pub struct TwitchStreamQuality {
    pub label: CString,
    pub url: CString,
    pub width: c_uint,
    pub height: c_uint,
    pub bandwidth: c_uint,
    pub frame_rate: c_double,
}

#[derive(Clone, Copy)]
pub struct TwitchPlaybackInfo {
    pub available: c_int,
    pub authenticated: c_int,
    pub user_id: u64,
    pub subscriber: c_int,
    pub turbo: c_int,
    pub show_ads: c_int,
    pub hide_ads: c_int,
    pub server_ads: c_int,
}

#[derive(Clone, Debug)]
pub struct TwitchStreamPreview {
    pub channel: CString,
    pub display_name: CString,
    pub title: CString,
    pub avatar_url: CString,
    pub preview_url: CString,
    pub started_at: CString,
    pub category_name: CString,
    pub viewer_count: c_uint,
}

#[derive(Clone, Debug)]
pub struct TwitchCurrentStream {
    pub title: CString,
    pub started_at: CString,
    pub category_name: CString,
    pub viewer_count: c_uint,
}

struct TwitchStreamQualitiesResult {
    qualities: Vec<TwitchStreamQuality>,
    playback_info: TwitchPlaybackInfo,
}

#[derive(Clone, Debug)]
pub struct TwitchFollowedChannel {
    pub channel: CString,
    pub display_name: CString,
}

pub struct FetchCurrentStreamData {
    channel: CString,
}

pub struct FetchStreamQualitiesData {
    channel: CString,
    oauth_token: CString,
}

pub struct FetchLiveChannelsData {
    channels: Vec<CString>,
}

pub struct FetchFollowedChannelsData {
    client_id: CString,
    oauth_token: CString,
}

pub type GAsyncReadyCallback = unsafe extern "C" fn(*mut c_void, *mut GAsyncResult, *mut c_void);

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

type GTaskThreadFunc =
    unsafe extern "C" fn(*mut GTask, *mut c_void, *mut c_void, *mut GCancellable);

pub fn twitch_playback_info_empty() -> TwitchPlaybackInfo {
    TwitchPlaybackInfo {
        available: 0,
        authenticated: 0,
        user_id: 0,
        subscriber: 0,
        turbo: 0,
        show_ads: 0,
        hide_ads: 0,
        server_ads: 0,
    }
}

pub fn twitch_stream_info_error_quark() -> c_uint {
    unsafe { g_quark_from_static_string(c"twitch-stream-info-error".as_ptr()) }
}

/// # Safety
/// - `cancel`: null or live `GCancellable`.
/// - Callback: declared C ABI, no unwind; data valid through completion.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn twitch_stream_info_fetch_current_stream_async(
    channel: *const c_char,
    cancel: *mut GCancellable,
    callback: Option<GAsyncReadyCallback>,
    user_data: *mut c_void,
) {
    unsafe {
        if !is_nonempty(channel) {
            return;
        }
        let data = Box::into_raw(Box::new(FetchCurrentStreamData {
            channel: CString::new(CStr::from_ptr(channel).to_bytes().to_ascii_lowercase()).unwrap(),
        }));
        let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
        g_task_set_task_data(
            task,
            data as *mut c_void,
            Some(fetch_current_stream_data_free),
        );
        g_task_run_in_thread(task, Some(fetch_current_stream_worker));
        g_object_unref(task as *mut c_void);
    }
}

/// # Safety
/// - `result`: matching async result, consumed once on originating main context.
/// - `error`: null or writable, initially null `GError*` slot.
/// - Result owned; release with matching module free function.
pub unsafe fn twitch_stream_info_fetch_current_stream_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut TwitchCurrentStream {
    unsafe {
        if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
            return ptr::null_mut();
        }
        g_task_propagate_pointer(result as *mut GTask, error) as *mut TwitchCurrentStream
    }
}

/// # Safety
/// - `cancel`: null or live `GCancellable`.
/// - Callback: declared C ABI, no unwind; data valid through completion.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn twitch_stream_info_fetch_stream_qualities_async(
    channel: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    callback: Option<GAsyncReadyCallback>,
    user_data: *mut c_void,
) {
    unsafe {
        if !is_nonempty(channel) {
            return;
        }
        let data = Box::into_raw(Box::new(FetchStreamQualitiesData {
            channel: CString::new(CStr::from_ptr(channel).to_bytes().to_ascii_lowercase()).unwrap(),
            oauth_token: if is_nonempty(oauth_token) {
                CStr::from_ptr(oauth_token).to_owned()
            } else {
                CString::default()
            },
        }));
        let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
        g_task_set_task_data(
            task,
            data as *mut c_void,
            Some(fetch_stream_qualities_data_free),
        );
        g_task_run_in_thread(task, Some(fetch_stream_qualities_worker));
        g_object_unref(task as *mut c_void);
    }
}

/// # Safety
/// - `result`: matching async result, consumed once on originating main context.
/// - `error`: null or writable, initially null `GError*` slot.
/// - `playback_info_out`: null or writable `TwitchPlaybackInfo` slot.
/// - Returns the owned list on success.
pub unsafe fn twitch_stream_info_fetch_stream_qualities_finish(
    result: *mut GAsyncResult,
    playback_info_out: *mut TwitchPlaybackInfo,
    error: *mut *mut GError,
) -> Option<Vec<TwitchStreamQuality>> {
    unsafe {
        if !playback_info_out.is_null() {
            *playback_info_out = twitch_playback_info_empty();
        }
        if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
            return None;
        }
        let fetched = g_task_propagate_pointer(result as *mut GTask, error)
            as *mut TwitchStreamQualitiesResult;
        if fetched.is_null() {
            return None;
        }

        if !playback_info_out.is_null() {
            *playback_info_out = (*fetched).playback_info;
        }
        let fetched = Box::from_raw(fetched);
        Some(fetched.qualities)
    }
}

/// # Safety
/// - `cancel`: null or live `GCancellable`.
/// - Callback: declared C ABI, no unwind; data valid through completion.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn twitch_stream_info_fetch_live_channels_async(
    channels: &[String],
    cancel: *mut GCancellable,
    callback: Option<GAsyncReadyCallback>,
    user_data: *mut c_void,
) {
    unsafe {
        let data = Box::into_raw(Box::new(FetchLiveChannelsData {
            channels: channels
                .iter()
                .map(|channel| CString::new(channel.as_str()).expect("channel login"))
                .collect(),
        }));
        let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
        g_task_set_task_data(
            task,
            data as *mut c_void,
            Some(fetch_live_channels_data_free),
        );
        g_task_run_in_thread(task, Some(fetch_live_channels_worker));
        g_object_unref(task as *mut c_void);
    }
}

/// # Safety
/// - `result`: matching async result, consumed once on originating main context.
/// - `error`: null or writable, initially null `GError*` slot.
/// - Returns the owned list on success.
pub unsafe fn twitch_stream_info_fetch_live_channels_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> Option<Vec<TwitchStreamPreview>> {
    unsafe {
        if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
            return None;
        }
        let values =
            g_task_propagate_pointer(result as *mut GTask, error) as *mut Vec<TwitchStreamPreview>;
        (!values.is_null()).then(|| *Box::from_raw(values))
    }
}

/// # Safety
/// - `cancel`: null or live `GCancellable`.
/// - Callback: declared C ABI, no unwind; data valid through completion.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn twitch_stream_info_fetch_followed_channels_async(
    client_id: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    callback: Option<GAsyncReadyCallback>,
    user_data: *mut c_void,
) {
    unsafe {
        if !is_nonempty(client_id) || !is_nonempty(oauth_token) {
            return;
        }
        let data = Box::into_raw(Box::new(FetchFollowedChannelsData {
            client_id: CStr::from_ptr(client_id).to_owned(),
            oauth_token: CStr::from_ptr(oauth_token).to_owned(),
        }));
        let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
        g_task_set_task_data(
            task,
            data as *mut c_void,
            Some(fetch_followed_channels_data_free),
        );
        g_task_run_in_thread(task, Some(fetch_followed_channels_worker));
        g_object_unref(task as *mut c_void);
    }
}

/// # Safety
/// - `result`: matching async result, consumed once on originating main context.
/// - `error`: null or writable, initially null `GError*` slot.
/// - Returns the owned list on success.
pub unsafe fn twitch_stream_info_fetch_followed_channels_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> Option<Vec<TwitchFollowedChannel>> {
    unsafe {
        if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
            return None;
        }
        let values = g_task_propagate_pointer(result as *mut GTask, error)
            as *mut Vec<TwitchFollowedChannel>;
        (!values.is_null()).then(|| *Box::from_raw(values))
    }
}
