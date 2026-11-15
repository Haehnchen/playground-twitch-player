use super::ffi::{
    G_IO_ERROR_CANCELLED, G_IO_ERROR_FAILED, G_USEC_PER_SEC, GTask, g_ascii_strcasecmp,
    g_ascii_strdown, g_clear_error, g_error_matches, g_free, g_get_monotonic_time, g_get_real_time,
    g_io_error_quark, g_malloc0, g_ptr_array_add, g_ptr_array_new_with_free_func, g_ptr_array_ref,
    g_ptr_array_unref, g_set_error, g_strdup, g_task_return_pointer,
};
use super::{
    ChannelListResult, FetchChannelListData, FollowedCache, channel_list_result_free,
    g_free_destroy,
};
use crate::settings::{
    AppSettings, app_settings_get_channel, app_settings_get_channel_count, app_settings_save,
    app_settings_set_twitch_auth_tokens,
};
use crate::twitch::auth::twitch_auth_refresh_token;
use crate::twitch::stream_info::{
    GCancellable, GError, GPtrArray, TwitchFollowedChannel, twitch_stream_info_error_quark,
    twitch_stream_info_fetch_followed_channels,
};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::{Mutex, OnceLock};

pub(super) const TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED: c_int = 0;

pub(super) const TWITCH_AUTH_CLIENT_ID: &[u8] = b"8l1fzyh4jhs1cxhtqs6p4swmxuejh6\0";

pub(super) unsafe fn token_expires_at_from_expires_in(expires_in: c_uint) -> i64 {
    unsafe {
        if expires_in > 0 {
            g_get_real_time() / G_USEC_PER_SEC + expires_in as i64
        } else {
            0
        }
    }
}

pub(super) unsafe fn token_needs_refresh(expires_at: i64) -> bool {
    unsafe { expires_at > 0 && g_get_real_time() / G_USEC_PER_SEC + 60 >= expires_at }
}

pub(super) unsafe fn add_unique_channel(channels: *mut GPtrArray, channel: *const c_char) {
    unsafe {
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
}

pub(super) unsafe fn channels_array_from_ptr_array(channels: *mut GPtrArray) -> *mut *mut c_char {
    unsafe {
        let count = (*channels).len as usize;
        let result =
            g_malloc0((count + 1) * std::mem::size_of::<*mut c_char>()) as *mut *mut c_char;

        for i in 0..count {
            let channel = *(*channels).pdata.add(i) as *const c_char;
            *result.add(i) = g_strdup(channel);
        }

        result
    }
}

pub(super) unsafe fn collect_settings_channels(
    settings: *const AppSettings,
    count_out: *mut c_uint,
) -> *mut *mut c_char {
    unsafe {
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
}

pub(super) unsafe fn build_channel_list_result(
    manual_channels: *mut *mut c_char,
    manual_channel_count: c_uint,
    followed_channels: *mut GPtrArray,
) -> *mut ChannelListResult {
    unsafe {
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
}

pub(super) unsafe fn refresh_access_token(
    data: *mut FetchChannelListData,
    result: *mut ChannelListResult,
    oauth_token: *mut *mut c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> bool {
    unsafe {
        if (*data).refresh_token.is_null() || *(*data).refresh_token == 0 {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"Connect Twitch again to refresh followed channels".as_ptr(),
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
}

// Persist refreshed credentials on the settings-owning thread.
pub(super) unsafe fn save_refreshed_token(
    result: *mut ChannelListResult,
    error: *mut *mut GError,
) -> bool {
    unsafe {
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
}

pub(super) unsafe fn error_is_cancelled(error: *mut *mut GError) -> bool {
    unsafe {
        !error.is_null()
            && !(*error).is_null()
            && g_error_matches(*error, g_io_error_quark(), G_IO_ERROR_CANCELLED) != 0
    }
}

// Merge manual and followed channels, refreshing expired tokens as needed.
pub(super) unsafe fn fetch_channel_list(
    data: *mut FetchChannelListData,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut ChannelListResult {
    unsafe {
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

        if (oauth_token.is_null()
            || *oauth_token == 0
            || token_needs_refresh((*data).oauth_expires_at))
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
}

pub(super) unsafe extern "C" fn fetch_channel_list_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    unsafe {
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
}

pub(super) const FOLLOWED_CHANNELS_CACHE_SECONDS: i64 = 120;

pub(super) fn followed_cache() -> &'static Mutex<FollowedCache> {
    static CACHE: OnceLock<Mutex<FollowedCache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(FollowedCache {
            channels: ptr::null_mut(),
            cached_at_us: 0,
        })
    })
}

pub(super) unsafe fn dup_fresh_followed_cache() -> *mut GPtrArray {
    unsafe {
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
}

pub(super) unsafe fn store_followed_cache(channels: *mut GPtrArray) {
    unsafe {
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
}

pub fn twitch_channel_list_invalidate_followed_cache() {
    unsafe {
        let mut cache = followed_cache()
            .lock()
            .expect("followed channel cache poisoned");
        if !cache.channels.is_null() {
            g_ptr_array_unref(cache.channels);
            cache.channels = ptr::null_mut();
        }
        cache.cached_at_us = 0;
    }
}
