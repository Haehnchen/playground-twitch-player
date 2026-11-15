mod fetch;
mod ffi;
mod playlist;
mod requests;
mod responses;
#[cfg(test)]
mod tests;
mod utils;

pub use self::ffi::{
    GAsyncResult, GBytes, GCancellable, GDateTime, GError, GPtrArray, GTask, SoupMessage,
    SoupMessageHeaders, SoupSession, TwitchPlaybackInfo, TwitchStreamQuality,
};
pub use self::responses::twitch_stream_info_fetch_followed_channels;
pub use self::utils::{
    twitch_current_stream_free, twitch_followed_channel_free, twitch_stream_preview_free,
    twitch_stream_quality_free,
};
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
    g_ascii_strdown, g_malloc0, g_object_unref, g_quark_from_static_string, g_strdup,
    g_task_is_valid, g_task_new, g_task_propagate_pointer, g_task_run_in_thread,
    g_task_set_task_data,
};
use self::utils::twitch_stream_qualities_result_free;
use crate::glib_strings::is_nonempty;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

pub struct TwitchStreamPreview {
    pub channel: *mut c_char,
    pub display_name: *mut c_char,
    pub title: *mut c_char,
    pub avatar_url: *mut c_char,
    pub preview_url: *mut c_char,
    pub started_at: *mut c_char,
    pub category_name: *mut c_char,
    pub viewer_count: c_uint,
}

pub struct TwitchCurrentStream {
    pub title: *mut c_char,
    pub started_at: *mut c_char,
    pub category_name: *mut c_char,
    pub viewer_count: c_uint,
}

struct TwitchStreamQualitiesResult {
    qualities: *mut GPtrArray,
    playback_info: TwitchPlaybackInfo,
}

pub struct TwitchFollowedChannel {
    pub channel: *mut c_char,
    pub display_name: *mut c_char,
}

pub struct FetchCurrentStreamData {
    channel: *mut c_char,
}

pub struct FetchStreamQualitiesData {
    channel: *mut c_char,
    oauth_token: *mut c_char,
}

pub struct FetchLiveChannelsData {
    channels: *mut *mut c_char,
    channel_count: c_uint,
}

pub struct FetchFollowedChannelsData {
    client_id: *mut c_char,
    oauth_token: *mut c_char,
}

pub type GAsyncReadyCallback = unsafe extern "C" fn(*mut c_void, *mut GAsyncResult, *mut c_void);

type GCompareFunc = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

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
            channel: g_ascii_strdown(channel, -1),
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
            channel: g_ascii_strdown(channel, -1),
            oauth_token: if is_nonempty(oauth_token) {
                g_strdup(oauth_token)
            } else {
                ptr::null_mut()
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
/// - Result owned; release with `g_ptr_array_unref`.
pub unsafe fn twitch_stream_info_fetch_stream_qualities_finish(
    result: *mut GAsyncResult,
    playback_info_out: *mut TwitchPlaybackInfo,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    unsafe {
        if !playback_info_out.is_null() {
            *playback_info_out = twitch_playback_info_empty();
        }
        if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
            return ptr::null_mut();
        }
        let fetched = g_task_propagate_pointer(result as *mut GTask, error)
            as *mut TwitchStreamQualitiesResult;
        if fetched.is_null() {
            return ptr::null_mut();
        }

        if !playback_info_out.is_null() {
            *playback_info_out = (*fetched).playback_info;
        }
        let qualities = (*fetched).qualities;
        (*fetched).qualities = ptr::null_mut();
        twitch_stream_qualities_result_free(fetched as *mut c_void);
        qualities
    }
}

/// # Safety
/// - `cancel`: null or live `GCancellable`.
/// - Callback: declared C ABI, no unwind; data valid through completion.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
/// - `channels`: `channel_count` readable string-pointer slots.
pub unsafe fn twitch_stream_info_fetch_live_channels_async(
    channels: *const *const c_char,
    channel_count: c_uint,
    cancel: *mut GCancellable,
    callback: Option<GAsyncReadyCallback>,
    user_data: *mut c_void,
) {
    unsafe {
        let data = Box::into_raw(Box::new(FetchLiveChannelsData {
            channel_count,
            channels: g_malloc0((channel_count as usize + 1) * std::mem::size_of::<*mut c_char>())
                as *mut *mut c_char,
        }));
        for i in 0..channel_count {
            let channel = *channels.add(i as usize);
            *(*data).channels.add(i as usize) = if channel.is_null() {
                g_strdup(c"".as_ptr())
            } else {
                g_ascii_strdown(channel, -1)
            };
        }
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
/// - Result owned; release with `g_ptr_array_unref`.
pub unsafe fn twitch_stream_info_fetch_live_channels_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    unsafe {
        if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
            return ptr::null_mut();
        }
        g_task_propagate_pointer(result as *mut GTask, error) as *mut GPtrArray
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
            client_id: g_strdup(client_id),
            oauth_token: g_strdup(oauth_token),
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
/// - Result owned; release with `g_ptr_array_unref`.
pub unsafe fn twitch_stream_info_fetch_followed_channels_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    unsafe {
        if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
            return ptr::null_mut();
        }
        g_task_propagate_pointer(result as *mut GTask, error) as *mut GPtrArray
    }
}
