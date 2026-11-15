#![allow(clashing_extern_declarations)]

mod fetch;
mod ffi;

pub use self::fetch::twitch_channel_list_invalidate_followed_cache;
pub use self::ffi::GTask;

use self::fetch::{collect_settings_channels, fetch_channel_list_worker, save_refreshed_token};
use self::ffi::{
    g_clear_error, g_free, g_object_unref, g_propagate_prefixed_error, g_strdup, g_strfreev,
    g_task_is_valid, g_task_new, g_task_propagate_pointer, g_task_run_in_thread,
    g_task_set_task_data,
};
use crate::settings::{
    AppSettings, app_settings_free, app_settings_get_twitch_oauth_expires_at,
    app_settings_get_twitch_oauth_token, app_settings_get_twitch_refresh_token, app_settings_ref,
};
use crate::twitch::auth::{TwitchAuthToken, twitch_auth_token_free};
use crate::twitch::stream_info::{GAsyncResult, GCancellable, GError, GPtrArray};
use std::ffi::{c_char, c_uint, c_void};
use std::ptr;

struct FetchChannelListData {
    settings: *mut AppSettings,
    manual_channels: *mut *mut c_char,
    manual_channel_count: c_uint,

    oauth_token: *mut c_char,
    refresh_token: *mut c_char,
    oauth_expires_at: i64,
}

struct ChannelListResult {
    settings: *mut AppSettings,
    channels: *mut *mut c_char,
    channel_count: c_uint,
    refreshed_token: *mut TwitchAuthToken,
    error: *mut GError,
}

struct FollowedCache {
    channels: *mut GPtrArray,
    cached_at_us: i64,
}

unsafe impl Send for FollowedCache {}

type GAsyncReadyCallback =
    Option<unsafe extern "C" fn(*mut c_void, *mut GAsyncResult, *mut c_void)>;

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

type GTaskThreadFunc =
    unsafe extern "C" fn(*mut GTask, *mut c_void, *mut c_void, *mut GCancellable);

unsafe extern "C" fn g_free_destroy(data: *mut c_void) {
    unsafe {
        g_free(data);
    }
}

unsafe extern "C" fn fetch_channel_list_data_free(data: *mut c_void) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data as *mut FetchChannelListData);
        g_strfreev(data.manual_channels);
        g_free(data.oauth_token as *mut c_void);
        g_free(data.refresh_token as *mut c_void);
        app_settings_free(data.settings);
    }
}

unsafe extern "C" fn channel_list_result_free(data: *mut c_void) {
    unsafe {
        if data.is_null() {
            return;
        }

        let mut result = Box::from_raw(data as *mut ChannelListResult);
        g_strfreev(result.channels);
        twitch_auth_token_free(result.refreshed_token);
        g_clear_error(&mut result.error);
    }
}

/// # Safety
/// - `cancel`: null or live `GCancellable`.
/// - Callback: declared C ABI, no unwind; data valid through completion.
/// - `settings`: live; serialize access and finish call on its owning main context.
pub unsafe fn twitch_channel_list_fetch_async(
    settings: *mut AppSettings,
    cancel: *mut GCancellable,
    callback: GAsyncReadyCallback,
    user_data: *mut c_void,
) {
    unsafe {
        let mut manual_channel_count = 0;
        let settings_ref = app_settings_ref(settings);
        let data = Box::new(FetchChannelListData {
            settings: settings_ref,
            manual_channels: collect_settings_channels(settings, &mut manual_channel_count),
            manual_channel_count,

            oauth_token: g_strdup(app_settings_get_twitch_oauth_token(settings)),
            refresh_token: g_strdup(app_settings_get_twitch_refresh_token(settings)),
            oauth_expires_at: app_settings_get_twitch_oauth_expires_at(settings),
        });

        let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
        g_task_set_task_data(
            task,
            Box::into_raw(data) as *mut c_void,
            Some(fetch_channel_list_data_free),
        );
        g_task_run_in_thread(task, Some(fetch_channel_list_worker));
        g_object_unref(task as *mut c_void);
    }
}

/// # Safety
/// - `result`: matching async result, consumed once on originating main context.
/// - `error`: null or writable, initially null `GError*` slot.
/// - `channel_count_out`: null or writable `c_uint` slot.
/// - Result owned; release with `g_strfreev`.
pub unsafe fn twitch_channel_list_fetch_finish(
    result: *mut GAsyncResult,
    channel_count_out: *mut c_uint,
    error: *mut *mut GError,
) -> *mut *mut c_char {
    unsafe {
        if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
            return ptr::null_mut();
        }

        let list = g_task_propagate_pointer(result as *mut GTask, error) as *mut ChannelListResult;
        if list.is_null() {
            if !channel_count_out.is_null() {
                *channel_count_out = 0;
            }
            return ptr::null_mut();
        }

        let mut save_error: *mut GError = ptr::null_mut();
        if !save_refreshed_token(list, &mut save_error) {
            if !error.is_null() {
                g_propagate_prefixed_error(
                    error,
                    save_error,
                    c"Twitch token refreshed, but saving failed: ".as_ptr(),
                );
                save_error = ptr::null_mut();
            }
            g_clear_error(&mut save_error);
            channel_list_result_free(list as *mut c_void);
            if !channel_count_out.is_null() {
                *channel_count_out = 0;
            }
            return ptr::null_mut();
        }

        if !(*list).error.is_null() {
            if !error.is_null() {
                *error = (*list).error;
                (*list).error = ptr::null_mut();
            }
            channel_list_result_free(list as *mut c_void);
            if !channel_count_out.is_null() {
                *channel_count_out = 0;
            }
            return ptr::null_mut();
        }

        let channels = (*list).channels;
        (*list).channels = ptr::null_mut();
        if !channel_count_out.is_null() {
            *channel_count_out = (*list).channel_count;
        }
        channel_list_result_free(list as *mut c_void);
        channels
    }
}
