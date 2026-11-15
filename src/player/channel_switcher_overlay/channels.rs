//! Live channel queries and caching.

use super::ffi::*;
use super::images::clear_image_cache;
use super::runtime::{G_LOG_DOMAIN, clear_object, log_debug};
use super::widgets::{clear_preview_cards, render_live_channels, show_status};
use super::{ChannelSwitcherOverlay, channel_switcher_overlay_ref, channel_switcher_overlay_unref};
use crate::twitch::channel_list::{
    twitch_channel_list_fetch_async, twitch_channel_list_fetch_finish,
};
use crate::twitch::stream_info::{
    GAsyncResult, GError, twitch_stream_info_fetch_live_channels_async,
    twitch_stream_info_fetch_live_channels_finish,
};
use std::ffi::{c_char, c_int, c_uint, c_void};
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
            live_fetch_callback_data_free(data);
            return;
        }

        clear_preview_cards(switcher);
        (*switcher).previews = previews;
        (*switcher).cached_at_us = g_get_monotonic_time();
        render_live_channels(switcher);

        live_fetch_callback_data_free(data);
    }
}

unsafe fn has_fresh_cache(switcher: *mut ChannelSwitcherOverlay, channels_key: &str) -> bool {
    unsafe {
        let now_us = g_get_monotonic_time();

        (*switcher).previews.is_some()
            && (*switcher).cached_channels_key.as_deref() == Some(channels_key)
            && now_us - (*switcher).cached_at_us < LIVE_CHANNELS_CACHE_SECONDS * G_USEC_PER_SEC
    }
}

unsafe fn start_live_channel_fetch(
    switcher: *mut ChannelSwitcherOverlay,
    channels: &[String],
    allow_cache: c_int,
) {
    unsafe {
        let channels_key = channels.join("\n");

        if channels.is_empty() {
            (*switcher).previews = None;
            clear_preview_cards(switcher);
            clear_image_cache(switcher);
            (*switcher).cached_channels_key = None;
            (*switcher).cached_at_us = 0;
            show_status(switcher, c"No channels configured".as_ptr());
            return;
        }

        if allow_cache != 0 && has_fresh_cache(switcher, &channels_key) {
            render_live_channels(switcher);
            return;
        }

        (*switcher).previews = None;
        clear_preview_cards(switcher);
        clear_image_cache(switcher);
        (*switcher).cached_channels_key = Some(channels_key);
        (*switcher).cached_at_us = 0;

        (*switcher).cancel = g_cancellable_new();
        let data = Box::into_raw(Box::new(LiveFetchCallbackData {
            switcher: channel_switcher_overlay_ref(switcher),
            generation: (*switcher).generation,
        }));
        twitch_stream_info_fetch_live_channels_async(
            channels,
            (*switcher).cancel,
            Some(on_live_channels_fetched),
            data as *mut c_void,
        );
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
        let channels = twitch_channel_list_fetch_finish(result, &mut error);

        if (*switcher).closing.load(Ordering::Acquire)
            || (*data).generation != (*switcher).generation
            || (*switcher).panel.is_null()
        {
            g_clear_error(&mut error);
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
            channel_list_fetch_callback_data_free(data);
            return;
        }

        start_live_channel_fetch(switcher, channels.as_deref().unwrap_or_default(), TRUE);
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
