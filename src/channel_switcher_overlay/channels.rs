//! Live channel queries and caching.

use super::cards::{clear_preview_cards, render_live_channels, show_status};
use super::ffi::*;
use super::images::clear_image_cache;
use super::runtime::{G_LOG_DOMAIN, clear_object, log_debug};
use super::{ChannelSwitcherOverlay, channel_switcher_overlay_ref, channel_switcher_overlay_unref};
use crate::glib_strings::dup_bytes;
use crate::twitch_channel_list::{
    twitch_channel_list_fetch_async, twitch_channel_list_fetch_finish,
};
use crate::twitch_stream_info::{
    GAsyncResult, GError, twitch_stream_info_fetch_live_channels_async,
    twitch_stream_info_fetch_live_channels_finish,
};
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

const LIVE_CHANNELS_CACHE_SECONDS: i64 = 10;

struct LiveFetchCallbackData {
    switcher: *mut ChannelSwitcherOverlay,
    generation: c_uint,
}

struct ChannelListFetchCallbackData {
    switcher: *mut ChannelSwitcherOverlay,
    generation: c_uint,
}

unsafe fn live_fetch_callback_data_free(data: *mut LiveFetchCallbackData) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data);
        channel_switcher_overlay_unref(data.switcher);
    }
}

unsafe fn channel_list_fetch_callback_data_free(data: *mut ChannelListFetchCallbackData) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data);
        channel_switcher_overlay_unref(data.switcher);
    }
}

// Discard results if the picker closed or a newer request started.
unsafe extern "C" fn on_live_channels_fetched(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let data = user_data as *mut LiveFetchCallbackData;
        let switcher = (*data).switcher;
        let mut error: *mut GError = ptr::null_mut();
        let previews = twitch_stream_info_fetch_live_channels_finish(result, &mut error);

        if (*switcher).closing.load(Ordering::Acquire)
            || (*data).generation != (*switcher).generation
            || (*switcher).panel.is_null()
        {
            g_clear_error(&mut error);
            if !previews.is_null() {
                g_ptr_array_unref(previews);
            }
            live_fetch_callback_data_free(data);
            return;
        }

        clear_object(&mut (*switcher).cancel);

        if !error.is_null() {
            if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0 {
                g_log(
                    G_LOG_DOMAIN.as_ptr() as *const c_char,
                    G_LOG_LEVEL_DEBUG,
                    c"live channel fetch failed: %s".as_ptr(),
                    (*error).message,
                );
                show_status(switcher, c"Live channels could not be loaded".as_ptr());
            }
            g_clear_error(&mut error);
            if !previews.is_null() {
                g_ptr_array_unref(previews);
            }
            live_fetch_callback_data_free(data);
            return;
        }

        if !(*switcher).previews.is_null() {
            g_ptr_array_unref((*switcher).previews);
        }
        clear_preview_cards(switcher);
        (*switcher).previews = if previews.is_null() {
            ptr::null_mut()
        } else {
            g_ptr_array_ref(previews)
        };
        if !previews.is_null() {
            g_ptr_array_unref(previews);
        }
        (*switcher).cached_at_us = g_get_monotonic_time();
        render_live_channels(switcher);

        live_fetch_callback_data_free(data);
    }
}

unsafe fn build_channels_cache_key(
    channels: *mut *mut c_char,
    channel_count: c_uint,
) -> *mut c_char {
    unsafe {
        let mut key = Vec::new();

        for i in 0..channel_count {
            if i > 0 {
                key.push(b'\n');
            }
            let channel = *channels.add(i as usize);
            if !channel.is_null() {
                key.extend_from_slice(CStr::from_ptr(channel).to_bytes());
            }
        }

        dup_bytes(&key)
    }
}

unsafe fn has_fresh_cache(
    switcher: *mut ChannelSwitcherOverlay,
    channels_key: *const c_char,
) -> bool {
    unsafe {
        let now_us = g_get_monotonic_time();

        !(*switcher).previews.is_null()
            && !(*switcher).cached_channels_key.is_null()
            && !channels_key.is_null()
            && g_strcmp0((*switcher).cached_channels_key, channels_key) == 0
            && now_us - (*switcher).cached_at_us < LIVE_CHANNELS_CACHE_SECONDS * G_USEC_PER_SEC
    }
}

unsafe fn start_live_channel_fetch(
    switcher: *mut ChannelSwitcherOverlay,
    channels: *mut *mut c_char,
    channel_count: c_uint,
    allow_cache: c_int,
) {
    unsafe {
        let channels_key = build_channels_cache_key(channels, channel_count);

        if channel_count == 0 {
            if !(*switcher).previews.is_null() {
                g_ptr_array_unref((*switcher).previews);
                (*switcher).previews = ptr::null_mut();
            }
            clear_preview_cards(switcher);
            clear_image_cache(switcher);
            g_free((*switcher).cached_channels_key as *mut c_void);
            (*switcher).cached_channels_key = ptr::null_mut();
            (*switcher).cached_at_us = 0;
            show_status(switcher, c"No channels configured".as_ptr());
            g_free(channels_key as *mut c_void);
            return;
        }

        if allow_cache != 0 && has_fresh_cache(switcher, channels_key) {
            render_live_channels(switcher);
            g_free(channels_key as *mut c_void);
            return;
        }

        if !(*switcher).previews.is_null() {
            g_ptr_array_unref((*switcher).previews);
            (*switcher).previews = ptr::null_mut();
        }
        clear_preview_cards(switcher);
        clear_image_cache(switcher);
        g_free((*switcher).cached_channels_key as *mut c_void);
        (*switcher).cached_channels_key = g_strdup(channels_key);
        (*switcher).cached_at_us = 0;

        (*switcher).cancel = g_cancellable_new();
        let data = Box::into_raw(Box::new(LiveFetchCallbackData {
            switcher: channel_switcher_overlay_ref(switcher),
            generation: (*switcher).generation,
        }));
        twitch_stream_info_fetch_live_channels_async(
            channels as *const *const c_char,
            channel_count,
            (*switcher).cancel,
            Some(on_live_channels_fetched),
            data as *mut c_void,
        );
        g_free(channels_key as *mut c_void);
    }
}

unsafe extern "C" fn on_channel_list_fetched(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let data = user_data as *mut ChannelListFetchCallbackData;
        let switcher = (*data).switcher;
        let mut error: *mut GError = ptr::null_mut();
        let mut channel_count = 0;
        let channels = twitch_channel_list_fetch_finish(result, &mut channel_count, &mut error);

        if (*switcher).closing.load(Ordering::Acquire)
            || (*data).generation != (*switcher).generation
            || (*switcher).panel.is_null()
        {
            g_clear_error(&mut error);
            g_strfreev(channels);
            channel_list_fetch_callback_data_free(data);
            return;
        }

        clear_object(&mut (*switcher).cancel);

        if !error.is_null() {
            if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0 {
                log_debug(
                    c"channel list fetch failed: %s%s".as_ptr(),
                    (*error).message,
                    c"".as_ptr(),
                );
                show_status(switcher, (*error).message);
            }
            g_clear_error(&mut error);
            g_strfreev(channels);
            channel_list_fetch_callback_data_free(data);
            return;
        }

        start_live_channel_fetch(switcher, channels, channel_count, TRUE);

        g_strfreev(channels);
        channel_list_fetch_callback_data_free(data);
    }
}

// Fetch configured and followed channels without blocking GTK.
pub(super) unsafe fn fetch_channel_list(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        clear_object(&mut (*switcher).cancel);
        (*switcher).cancel = g_cancellable_new();
        let data = Box::into_raw(Box::new(ChannelListFetchCallbackData {
            switcher: channel_switcher_overlay_ref(switcher),
            generation: (*switcher).generation,
        }));
        twitch_channel_list_fetch_async(
            (*switcher).settings,
            (*switcher).cancel,
            Some(on_channel_list_fetched),
            data as *mut c_void,
        );
    }
}
