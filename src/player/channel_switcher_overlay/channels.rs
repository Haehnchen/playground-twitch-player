//! Shared live queries: one request serves every picker in the window.

use super::ChannelSwitcherOverlay;
use super::cache::{SharedPickerCache, WaitingPicker};
use super::ffi::*;
use super::runtime::switcher_is_closing;
use super::widgets::{clear_preview_cards, render_live_channels, show_status};
use crate::settings::{AppSettings, app_settings_free, app_settings_ref};
use crate::twitch::channel_list::{
    twitch_channel_list_fetch_async, twitch_channel_list_fetch_finish,
};
use crate::twitch::stream_info::{
    GAsyncResult, GError, TwitchStreamPreview, twitch_stream_info_fetch_live_channels_async,
    twitch_stream_info_fetch_live_channels_finish,
};
use gtk4::{glib::translate::*, prelude::*};
use std::{
    ffi::{CString, c_void},
    ptr,
};

struct FetchRequest {
    cache: SharedPickerCache,
    generation: u64,
    settings: *mut AppSettings,
}

impl Drop for FetchRequest {
    fn drop(&mut self) {
        unsafe {
            app_settings_free(self.settings);
        }
    }
}

unsafe fn apply_snapshot(switcher: *mut ChannelSwitcherOverlay) -> bool {
    unsafe {
        let cache = (*switcher).cache.clone();
        let live = cache.live.borrow();
        let Some(previews) = &live.previews else {
            return false;
        };
        if (*switcher).cache_revision != Some(live.revision) {
            clear_preview_cards(switcher);
            (*switcher).previews = Some(previews.clone());
            (*switcher).cache_revision = Some(live.revision);
            super::radial::reset(switcher);
        }
        drop(live);
        render_live_channels(switcher);
        true
    }
}

unsafe fn complete(
    request: FetchRequest,
    previews: Option<Vec<TwitchStreamPreview>>,
    error: Option<String>,
) {
    unsafe {
        let waiters = {
            let mut live = request.cache.live.borrow_mut();
            if live.generation != request.generation {
                return;
            }
            live.cancel = None;
            live.error = error;
            if let Some(previews) = previews {
                live.previews = Some(previews);
                live.cached_at_us = g_get_monotonic_time();
                live.revision = live.revision.wrapping_add(1);
            }
            std::mem::take(&mut live.waiters)
        };
        for waiter in waiters {
            let switcher = waiter.switcher;
            if switcher_is_closing(switcher) || (*switcher).generation != waiter.generation {
                continue;
            }
            if !apply_snapshot(switcher) {
                let message = request
                    .cache
                    .live
                    .borrow()
                    .error
                    .clone()
                    .unwrap_or_else(|| "Live channels could not be loaded".into());
                let message = CString::new(message)
                    .unwrap_or_else(|_| c"Live channels could not be loaded".into());
                show_status(switcher, message.as_ptr());
            }
        }
    }
}

unsafe extern "C" fn on_live_channels_fetched(
    _source: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let request = *Box::from_raw(user_data.cast::<FetchRequest>());
        let mut error: *mut GError = ptr::null_mut();
        let previews = twitch_stream_info_fetch_live_channels_finish(result, &mut error);
        let error = if error.is_null() {
            None
        } else {
            let error: gtk4::glib::Error = from_glib_full(error);
            Some(error.to_string())
        };
        complete(request, previews, error);
    }
}

unsafe extern "C" fn on_channel_list_fetched(
    _source: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let request = Box::from_raw(user_data.cast::<FetchRequest>());
        let mut error: *mut GError = ptr::null_mut();
        let channels = twitch_channel_list_fetch_finish(result, &mut error);
        if !error.is_null() {
            let error: gtk4::glib::Error = from_glib_full(error);
            complete(*request, None, Some(error.to_string()));
            return;
        }
        let cancel = {
            let live = request.cache.live.borrow();
            if live.generation != request.generation {
                return;
            }
            live.cancel.clone().unwrap()
        };
        let channels = channels.unwrap_or_default();
        if channels.is_empty() {
            complete(*request, Some(Vec::new()), None);
            return;
        }
        twitch_stream_info_fetch_live_channels_async(
            &channels,
            cancel.as_ptr().cast(),
            Some(on_live_channels_fetched),
            Box::into_raw(request).cast(),
        );
    }
}

pub(super) unsafe fn fetch_channel_list(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if switcher_is_closing(switcher) || (*switcher).settings.is_null() {
            return;
        }
        let have_snapshot = apply_snapshot(switcher);
        let cache = (*switcher).cache.clone();
        let mut live = cache.live.borrow_mut();
        let start = live.should_fetch(g_get_monotonic_time());
        if !have_snapshot {
            let message = live.error.as_deref().unwrap_or("Loading live channels");
            let message = CString::new(message)
                .unwrap_or_else(|_| c"Live channels could not be loaded".into());
            show_status(switcher, message.as_ptr());
        }
        if !start && live.cancel.is_none() {
            return;
        }
        if !live.waiters.iter().any(|waiter| {
            waiter.switcher == switcher && waiter.generation == (*switcher).generation
        }) {
            live.waiters.push(WaitingPicker::new(switcher));
        }
        if !start {
            return;
        }
        live.begin(g_get_monotonic_time());
        let cancel = live.cancel.clone().unwrap();
        let generation = live.generation;
        drop(live);
        let request = Box::new(FetchRequest {
            cache,
            generation,
            settings: app_settings_ref((*switcher).settings),
        });
        twitch_channel_list_fetch_async(
            request.settings,
            cancel.as_ptr().cast(),
            Some(on_channel_list_fetched),
            Box::into_raw(request).cast(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::channel_switcher_overlay::channel_switcher_overlay_free;

    fn picker(cache: &SharedPickerCache) -> *mut ChannelSwitcherOverlay {
        let mut picker = ChannelSwitcherOverlay::new(
            ptr::null_mut(),
            None,
            ptr::null_mut(),
            None,
            ptr::null_mut(),
        );
        picker.cache = cache.clone();
        Box::into_raw(Box::new(picker))
    }

    #[test]
    fn closing_one_tile_does_not_cancel_the_shared_result_for_another_tile() {
        unsafe {
            let cache = SharedPickerCache::default();
            let first = picker(&cache);
            let second = picker(&cache);
            cache.live.borrow_mut().begin(0);
            cache.live.borrow_mut().waiters =
                vec![WaitingPicker::new(first), WaitingPicker::new(second)];
            channel_switcher_overlay_free(first);
            assert!(!cache.live.borrow().cancel.as_ref().unwrap().is_cancelled());

            complete(
                FetchRequest {
                    cache: cache.clone(),
                    generation: 0,
                    settings: ptr::null_mut(),
                },
                Some(Vec::new()),
                None,
            );

            assert!((*second).previews.is_some());
            assert!(cache.live.borrow().waiters.is_empty());
            let third = picker(&cache);
            assert!(apply_snapshot(third));
            assert!((*third).previews.is_some());
            channel_switcher_overlay_free(second);
            channel_switcher_overlay_free(third);
        }
    }

    #[test]
    fn an_old_request_cannot_repopulate_the_cache_after_settings_change() {
        unsafe {
            let cache = SharedPickerCache::default();
            cache.live.borrow_mut().begin(0);
            cache.invalidate();
            complete(
                FetchRequest {
                    cache: cache.clone(),
                    generation: 0,
                    settings: ptr::null_mut(),
                },
                Some(Vec::new()),
                None,
            );
            assert!(cache.live.borrow().previews.is_none());
            assert!(cache.live.borrow().should_fetch(1));
        }
    }
}
