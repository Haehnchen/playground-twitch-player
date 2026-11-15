use super::ffi::{
    G_IO_ERROR_CANCELLED, G_IO_ERROR_FAILED, G_USEC_PER_SEC, GTask, g_clear_error, g_error_matches,
    g_free, g_get_real_time, g_io_error_quark, g_malloc0, g_ptr_array_add,
    g_ptr_array_new_with_free_func, g_ptr_array_unref, g_set_error, g_strdup,
    g_task_return_pointer,
};
use super::{ChannelListResult, FetchChannelListData, FollowedCache, channel_list_result_free};
use crate::glib_strings::dup_bytes;
use crate::settings::{
    AppSettings, app_settings_get_channel, app_settings_get_channel_count, app_settings_save,
    app_settings_set_twitch_auth_tokens,
};
use crate::twitch::auth::twitch_auth_refresh_token;
use crate::twitch::stream_info::{
    GCancellable, GError, GPtrArray, TwitchFollowedChannel, twitch_stream_info_error_quark,
    twitch_stream_info_fetch_followed_channels,
};
use std::collections::HashSet;
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

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

// Keep insertion order while removing duplicate channel logins.
fn unique_channels(channels: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    channels
        .into_iter()
        .map(|channel| channel.to_ascii_lowercase())
        .filter(|channel| !channel.is_empty() && seen.insert(channel.clone()))
        .collect()
}

fn channels_array(channels: &[String]) -> *mut *mut c_char {
    unsafe {
        let result = g_malloc0((channels.len() + 1) * std::mem::size_of::<*mut c_char>())
            .cast::<*mut c_char>();
        for (index, channel) in channels.iter().enumerate() {
            *result.add(index) = dup_bytes(channel.as_bytes());
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
        let mut channels = Vec::new();
        for i in 0..manual_channel_count {
            let channel = *manual_channels.add(i as usize);
            if !channel.is_null() {
                channels.push(CStr::from_ptr(channel).to_string_lossy().into_owned());
            }
        }
        if !followed_channels.is_null() {
            for i in 0..(*followed_channels).len {
                let followed =
                    *(*followed_channels).pdata.add(i as usize) as *mut TwitchFollowedChannel;
                if !followed.is_null() && !(*followed).channel.is_null() {
                    channels.push(
                        CStr::from_ptr((*followed).channel)
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        let channels = unique_channels(channels);

        let result = Box::new(ChannelListResult {
            settings: ptr::null_mut(),
            channels: channels_array(&channels),
            channel_count: channels.len() as c_uint,
            refreshed_token: ptr::null_mut(),
            error: ptr::null_mut(),
        });
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
            channels: Vec::new(),
            cached_at: None,
        })
    })
}

unsafe extern "C" fn free_followed_channel(data: *mut c_void) {
    unsafe {
        crate::twitch::stream_info::twitch_followed_channel_free(data.cast());
    }
}

pub(super) unsafe fn dup_fresh_followed_cache() -> *mut GPtrArray {
    unsafe {
        let cache = followed_cache()
            .lock()
            .expect("followed channel cache poisoned");
        if !cache.cached_at.is_some_and(|time| {
            time.elapsed() < Duration::from_secs(FOLLOWED_CHANNELS_CACHE_SECONDS as u64)
        }) {
            return ptr::null_mut();
        }
        let channels = g_ptr_array_new_with_free_func(Some(free_followed_channel));
        for (channel, display_name) in &cache.channels {
            let followed = Box::new(TwitchFollowedChannel {
                channel: dup_bytes(channel.as_bytes()),
                display_name: dup_bytes(display_name.as_bytes()),
            });
            g_ptr_array_add(channels, Box::into_raw(followed).cast());
        }
        channels
    }
}

pub(super) unsafe fn store_followed_cache(channels: *mut GPtrArray) {
    unsafe {
        let mut cache = followed_cache()
            .lock()
            .expect("followed channel cache poisoned");
        cache.channels.clear();
        cache.cached_at = if channels.is_null() {
            None
        } else {
            Some(Instant::now())
        };
        if !channels.is_null() {
            for index in 0..(*channels).len {
                let channel = *(*channels).pdata.add(index as usize) as *mut TwitchFollowedChannel;
                cache.channels.push((
                    CStr::from_ptr((*channel).channel)
                        .to_string_lossy()
                        .into_owned(),
                    CStr::from_ptr((*channel).display_name)
                        .to_string_lossy()
                        .into_owned(),
                ));
            }
        }
    }
}

pub fn twitch_channel_list_invalidate_followed_cache() {
    let mut cache = followed_cache()
        .lock()
        .expect("followed channel cache poisoned");
    cache.channels.clear();
    cache.cached_at = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_merge_keeps_order_and_ignores_case_and_empty_names() {
        assert_eq!(
            unique_channels(["Foo", "", "BAR", "foo", "Baz", "bar"].map(String::from)),
            ["foo", "bar", "baz"]
        );
    }

    #[test]
    fn followed_cache_snapshot_survives_invalidation_and_expires() {
        unsafe {
            let channels = g_ptr_array_new_with_free_func(Some(free_followed_channel));
            let channel = Box::new(TwitchFollowedChannel {
                channel: dup_bytes(b"example"),
                display_name: dup_bytes(b"Example"),
            });
            g_ptr_array_add(channels, Box::into_raw(channel).cast());
            store_followed_cache(channels);
            g_ptr_array_unref(channels);
            let snapshot = dup_fresh_followed_cache();
            assert!(!snapshot.is_null());
            twitch_channel_list_invalidate_followed_cache();
            assert!(dup_fresh_followed_cache().is_null());
            assert_eq!((*snapshot).len, 1);
            let channel = *(*snapshot).pdata as *mut TwitchFollowedChannel;
            assert_eq!(CStr::from_ptr((*channel).channel), c"example");
            store_followed_cache(snapshot);
            g_ptr_array_unref(snapshot);
            followed_cache().lock().unwrap().cached_at = Some(
                Instant::now() - Duration::from_secs(FOLLOWED_CHANNELS_CACHE_SECONDS as u64 + 1),
            );
            assert!(dup_fresh_followed_cache().is_null());
            twitch_channel_list_invalidate_followed_cache();
        }
    }
}
