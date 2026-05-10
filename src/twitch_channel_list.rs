#![allow(clashing_extern_declarations)]

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::settings::{
    app_settings_get_channel, app_settings_get_channel_count,
    app_settings_get_twitch_oauth_expires_at, app_settings_get_twitch_oauth_token,
    app_settings_get_twitch_refresh_token, app_settings_save, app_settings_set_twitch_auth_tokens,
    AppSettings,
};
use crate::twitch_auth::{twitch_auth_refresh_token, twitch_auth_token_free, TwitchAuthToken};
use crate::twitch_stream_info::{
    twitch_stream_info_error_quark, twitch_stream_info_fetch_followed_channels, GAsyncResult,
    GCancellable, GError, GPtrArray, TwitchFollowedChannel,
};

const G_IO_ERROR_FAILED: c_int = 0;
const G_IO_ERROR_CANCELLED: c_int = 19;
const G_USEC_PER_SEC: i64 = 1_000_000;
const FOLLOWED_CHANNELS_CACHE_SECONDS: i64 = 120;
const TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED: c_int = 0;
const TWITCH_AUTH_CLIENT_ID: &[u8] = b"8l1fzyh4jhs1cxhtqs6p4swmxuejh6\0";

#[repr(C)]
pub struct GTask {
    _private: [u8; 0],
}

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

unsafe extern "C" {
    fn g_ascii_strcasecmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn g_ascii_strdown(str: *const c_char, len: isize) -> *mut c_char;
    fn g_clear_error(error: *mut *mut GError);
    fn g_error_matches(error: *const GError, domain: c_uint, code: c_int) -> c_int;
    fn g_free(mem: *mut c_void);
    fn g_get_monotonic_time() -> i64;
    fn g_get_real_time() -> i64;
    fn g_io_error_quark() -> c_uint;
    fn g_malloc0(n_bytes: usize) -> *mut c_void;
    fn g_object_unref(object: *mut c_void);
    fn g_propagate_prefixed_error(
        dest: *mut *mut GError,
        src: *mut GError,
        format: *const c_char,
        ...
    );
    fn g_ptr_array_add(array: *mut GPtrArray, data: *mut c_void);
    fn g_ptr_array_new_with_free_func(element_free_func: Option<GDestroyNotify>) -> *mut GPtrArray;
    fn g_ptr_array_ref(array: *mut GPtrArray) -> *mut GPtrArray;
    fn g_ptr_array_unref(array: *mut GPtrArray);
    fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn g_strfreev(str_array: *mut *mut c_char);

    fn g_task_is_valid(result: *mut c_void, source_object: *mut c_void) -> c_int;
    fn g_task_new(
        source_object: *mut c_void,
        cancellable: *mut GCancellable,
        callback: GAsyncReadyCallback,
        callback_data: *mut c_void,
    ) -> *mut GTask;
    fn g_task_propagate_pointer(task: *mut GTask, error: *mut *mut GError) -> *mut c_void;
    fn g_task_return_pointer(
        task: *mut GTask,
        result: *mut c_void,
        result_destroy: Option<GDestroyNotify>,
    );
    fn g_task_run_in_thread(task: *mut GTask, task_func: Option<GTaskThreadFunc>);
    fn g_task_set_task_data(
        task: *mut GTask,
        task_data: *mut c_void,
        task_data_destroy: Option<GDestroyNotify>,
    );

}

unsafe extern "C" fn g_free_destroy(data: *mut c_void) {
    g_free(data);
}

unsafe extern "C" fn fetch_channel_list_data_free(data: *mut c_void) {
    if data.is_null() {
        return;
    }

    let data = Box::from_raw(data as *mut FetchChannelListData);
    g_strfreev(data.manual_channels);
    g_free(data.oauth_token as *mut c_void);
    g_free(data.refresh_token as *mut c_void);
}

unsafe extern "C" fn channel_list_result_free(data: *mut c_void) {
    if data.is_null() {
        return;
    }

    let mut result = Box::from_raw(data as *mut ChannelListResult);
    g_strfreev(result.channels);
    twitch_auth_token_free(result.refreshed_token);
    g_clear_error(&mut result.error);
}

fn followed_cache() -> &'static Mutex<FollowedCache> {
    static CACHE: OnceLock<Mutex<FollowedCache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(FollowedCache {
            channels: ptr::null_mut(),
            cached_at_us: 0,
        })
    })
}

unsafe fn token_expires_at_from_expires_in(expires_in: c_uint) -> i64 {
    if expires_in > 0 {
        g_get_real_time() / G_USEC_PER_SEC + expires_in as i64
    } else {
        0
    }
}

unsafe fn token_needs_refresh(expires_at: i64) -> bool {
    expires_at > 0 && g_get_real_time() / G_USEC_PER_SEC + 60 >= expires_at
}

unsafe fn add_unique_channel(channels: *mut GPtrArray, channel: *const c_char) {
    if channel.is_null() || *channel == 0 {
        return;
    }

    for i in 0..(*channels).len {
        let existing = *(*channels).pdata.add(i as usize) as *const c_char;
        if g_ascii_strcasecmp(existing, channel) == 0 {
            return;
        }
    }

    g_ptr_array_add(channels, g_ascii_strdown(channel, -1) as *mut c_void);
}

unsafe fn channels_array_from_ptr_array(channels: *mut GPtrArray) -> *mut *mut c_char {
    let count = (*channels).len as usize;
    let result = g_malloc0((count + 1) * std::mem::size_of::<*mut c_char>()) as *mut *mut c_char;

    for i in 0..count {
        let channel = *(*channels).pdata.add(i) as *const c_char;
        *result.add(i) = g_strdup(channel);
    }

    result
}

unsafe fn collect_settings_channels(
    settings: *const AppSettings,
    count_out: *mut c_uint,
) -> *mut *mut c_char {
    let settings_channel_count = app_settings_get_channel_count(settings);
    let result =
        g_malloc0((settings_channel_count as usize + 1) * std::mem::size_of::<*mut c_char>())
            as *mut *mut c_char;
    let mut out = 0;

    for i in 0..settings_channel_count {
        let channel = app_settings_get_channel(settings, i);
        if channel.is_null() || (*channel).channel.is_null() || *(*channel).channel == 0 {
            continue;
        }

        *result.add(out as usize) = g_strdup((*channel).channel);
        out += 1;
    }

    if !count_out.is_null() {
        *count_out = out;
    }
    result
}

unsafe fn dup_fresh_followed_cache() -> *mut GPtrArray {
    let now_us = g_get_monotonic_time();
    let cache = followed_cache()
        .lock()
        .expect("followed channel cache poisoned");
    if !cache.channels.is_null()
        && now_us - cache.cached_at_us < FOLLOWED_CHANNELS_CACHE_SECONDS * G_USEC_PER_SEC
    {
        g_ptr_array_ref(cache.channels)
    } else {
        ptr::null_mut()
    }
}

unsafe fn store_followed_cache(channels: *mut GPtrArray) {
    let mut cache = followed_cache()
        .lock()
        .expect("followed channel cache poisoned");
    if !cache.channels.is_null() {
        g_ptr_array_unref(cache.channels);
    }
    cache.channels = if channels.is_null() {
        ptr::null_mut()
    } else {
        g_ptr_array_ref(channels)
    };
    cache.cached_at_us = g_get_monotonic_time();
}

pub unsafe fn twitch_channel_list_invalidate_followed_cache() {
    let mut cache = followed_cache()
        .lock()
        .expect("followed channel cache poisoned");
    if !cache.channels.is_null() {
        g_ptr_array_unref(cache.channels);
        cache.channels = ptr::null_mut();
    }
    cache.cached_at_us = 0;
}

unsafe fn build_channel_list_result(
    manual_channels: *mut *mut c_char,
    manual_channel_count: c_uint,
    followed_channels: *mut GPtrArray,
) -> *mut ChannelListResult {
    let combined = g_ptr_array_new_with_free_func(Some(g_free_destroy));

    for i in 0..manual_channel_count {
        add_unique_channel(combined, *manual_channels.add(i as usize));
    }

    if !followed_channels.is_null() {
        for i in 0..(*followed_channels).len {
            let followed =
                *(*followed_channels).pdata.add(i as usize) as *mut TwitchFollowedChannel;
            add_unique_channel(
                combined,
                if followed.is_null() {
                    ptr::null()
                } else {
                    (*followed).channel
                },
            );
        }
    }

    let result = Box::new(ChannelListResult {
        settings: ptr::null_mut(),
        channels: channels_array_from_ptr_array(combined),
        channel_count: (*combined).len,
        refreshed_token: ptr::null_mut(),
        error: ptr::null_mut(),
    });
    g_ptr_array_unref(combined);
    Box::into_raw(result)
}

unsafe fn refresh_access_token(
    data: *mut FetchChannelListData,
    result: *mut ChannelListResult,
    oauth_token: *mut *mut c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> bool {
    if (*data).refresh_token.is_null() || *(*data).refresh_token == 0 {
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_FAILED,
            b"Connect Twitch again to refresh followed channels\0".as_ptr() as *const c_char,
        );
        return false;
    }

    let token = twitch_auth_refresh_token(
        TWITCH_AUTH_CLIENT_ID.as_ptr() as *const c_char,
        (*data).refresh_token,
        cancel,
        error,
    );
    if token.is_null() {
        return false;
    }

    g_free(*oauth_token as *mut c_void);
    *oauth_token = g_strdup((*token).access_token);
    (*result).refreshed_token = token;
    true
}

unsafe fn save_refreshed_token(result: *mut ChannelListResult, error: *mut *mut GError) -> bool {
    if (*result).settings.is_null() || (*result).refreshed_token.is_null() {
        return true;
    }

    app_settings_set_twitch_auth_tokens(
        (*result).settings,
        (*(*result).refreshed_token).access_token,
        (*(*result).refreshed_token).refresh_token,
        token_expires_at_from_expires_in((*(*result).refreshed_token).expires_in),
    );

    app_settings_save((*result).settings, error) != 0
}

unsafe fn error_is_cancelled(error: *mut *mut GError) -> bool {
    !error.is_null()
        && !(*error).is_null()
        && g_error_matches(*error, g_io_error_quark(), G_IO_ERROR_CANCELLED) != 0
}

unsafe fn fetch_channel_list(
    data: *mut FetchChannelListData,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut ChannelListResult {
    let mut oauth_token = g_strdup((*data).oauth_token);

    if (oauth_token.is_null() || *oauth_token == 0)
        && ((*data).refresh_token.is_null() || *(*data).refresh_token == 0)
    {
        let result = build_channel_list_result(
            (*data).manual_channels,
            (*data).manual_channel_count,
            ptr::null_mut(),
        );
        (*result).settings = (*data).settings;
        g_free(oauth_token as *mut c_void);
        return result;
    }

    let mut followed_channels = dup_fresh_followed_cache();
    if !followed_channels.is_null() {
        let result = build_channel_list_result(
            (*data).manual_channels,
            (*data).manual_channel_count,
            followed_channels,
        );
        (*result).settings = (*data).settings;
        g_ptr_array_unref(followed_channels);
        g_free(oauth_token as *mut c_void);
        return result;
    }

    if (*data).manual_channel_count > 0 && TWITCH_AUTH_CLIENT_ID[0] == 0 {
        let result = build_channel_list_result(
            (*data).manual_channels,
            (*data).manual_channel_count,
            ptr::null_mut(),
        );
        (*result).settings = (*data).settings;
        g_free(oauth_token as *mut c_void);
        return result;
    }

    let result = Box::into_raw(Box::new(ChannelListResult {
        settings: (*data).settings,
        channels: ptr::null_mut(),
        channel_count: 0,
        refreshed_token: ptr::null_mut(),
        error: ptr::null_mut(),
    }));

    if (oauth_token.is_null() || *oauth_token == 0 || token_needs_refresh((*data).oauth_expires_at))
        && !refresh_access_token(data, result, &mut oauth_token, cancel, error)
    {
        if (*data).manual_channel_count > 0 && !error_is_cancelled(error) {
            if !error.is_null() && !(*error).is_null() {
                g_clear_error(error);
            }
            let manual_result = build_channel_list_result(
                (*data).manual_channels,
                (*data).manual_channel_count,
                ptr::null_mut(),
            );
            (*manual_result).settings = (*data).settings;
            channel_list_result_free(result as *mut c_void);
            g_free(oauth_token as *mut c_void);
            return manual_result;
        }
        channel_list_result_free(result as *mut c_void);
        g_free(oauth_token as *mut c_void);
        return ptr::null_mut();
    }

    followed_channels = twitch_stream_info_fetch_followed_channels(
        TWITCH_AUTH_CLIENT_ID.as_ptr() as *const c_char,
        oauth_token,
        cancel,
        error,
    );
    if followed_channels.is_null()
        && !error.is_null()
        && !(*error).is_null()
        && g_error_matches(
            *error,
            twitch_stream_info_error_quark(),
            TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED,
        ) != 0
        && (*result).refreshed_token.is_null()
    {
        g_clear_error(error);
        if refresh_access_token(data, result, &mut oauth_token, cancel, error) {
            followed_channels = twitch_stream_info_fetch_followed_channels(
                TWITCH_AUTH_CLIENT_ID.as_ptr() as *const c_char,
                oauth_token,
                cancel,
                error,
            );
        }
    }

    if followed_channels.is_null() {
        if (*data).manual_channel_count > 0 && !error_is_cancelled(error) {
            if !error.is_null() && !(*error).is_null() {
                g_clear_error(error);
            }
            let manual_result = build_channel_list_result(
                (*data).manual_channels,
                (*data).manual_channel_count,
                ptr::null_mut(),
            );
            (*manual_result).settings = (*data).settings;
            (*manual_result).refreshed_token = (*result).refreshed_token;
            (*result).refreshed_token = ptr::null_mut();
            channel_list_result_free(result as *mut c_void);
            g_free(oauth_token as *mut c_void);
            return manual_result;
        }
        channel_list_result_free(result as *mut c_void);
        g_free(oauth_token as *mut c_void);
        return ptr::null_mut();
    }

    store_followed_cache(followed_channels);
    let channels_result = build_channel_list_result(
        (*data).manual_channels,
        (*data).manual_channel_count,
        followed_channels,
    );
    (*channels_result).settings = (*data).settings;
    (*channels_result).refreshed_token = (*result).refreshed_token;
    (*result).refreshed_token = ptr::null_mut();
    channel_list_result_free(result as *mut c_void);
    g_ptr_array_unref(followed_channels);
    g_free(oauth_token as *mut c_void);
    channels_result
}

unsafe extern "C" fn fetch_channel_list_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    let data = task_data as *mut FetchChannelListData;
    let mut error: *mut GError = ptr::null_mut();
    let mut result = fetch_channel_list(data, cancel, &mut error);

    if result.is_null() {
        result = Box::into_raw(Box::new(ChannelListResult {
            settings: (*data).settings,
            channels: ptr::null_mut(),
            channel_count: 0,
            refreshed_token: ptr::null_mut(),
            error,
        }));
        error = ptr::null_mut();
    } else if !error.is_null() {
        (*result).error = error;
        error = ptr::null_mut();
    }
    g_clear_error(&mut error);

    g_task_return_pointer(task, result as *mut c_void, Some(channel_list_result_free));
}

pub unsafe fn twitch_channel_list_fetch_async(
    settings: *mut AppSettings,
    cancel: *mut GCancellable,
    callback: GAsyncReadyCallback,
    user_data: *mut c_void,
) {
    let mut manual_channel_count = 0;
    let data = Box::new(FetchChannelListData {
        settings,
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

pub unsafe fn twitch_channel_list_fetch_finish(
    result: *mut GAsyncResult,
    channel_count_out: *mut c_uint,
    error: *mut *mut GError,
) -> *mut *mut c_char {
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
                b"Twitch token refreshed, but saving failed: \0".as_ptr() as *const c_char,
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
