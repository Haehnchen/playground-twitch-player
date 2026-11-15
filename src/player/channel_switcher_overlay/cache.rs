//! One live snapshot and avatar store for every picker in a player window.

use super::images::AvatarCache;
use super::{ChannelSwitcherOverlay, channel_switcher_overlay_ref, channel_switcher_overlay_unref};
use crate::twitch::stream_info::TwitchStreamPreview;
use gtk4::{gio, prelude::*};
use std::{cell::RefCell, rc::Rc};

const LIVE_CACHE_US: i64 = 60_000_000;
const RETRY_US: i64 = 10_000_000;

#[derive(Clone, Default)]
pub(crate) struct SharedPickerCache {
    pub(super) live: Rc<RefCell<LiveCache>>,
    pub(super) avatars: Rc<RefCell<AvatarCache>>,
}

#[derive(Default)]
pub(super) struct LiveCache {
    pub(super) previews: Option<Vec<TwitchStreamPreview>>,
    pub(super) cached_at_us: i64,
    pub(super) revision: u64,
    pub(super) generation: u64,
    pub(super) cancel: Option<gio::Cancellable>,
    pub(super) waiters: Vec<WaitingPicker>,
    pub(super) error: Option<String>,
    last_attempt_us: Option<i64>,
}

impl LiveCache {
    pub(super) fn should_fetch(&self, now: i64) -> bool {
        self.cancel.is_none()
            && !self
                .previews
                .as_ref()
                .is_some_and(|_| now - self.cached_at_us < LIVE_CACHE_US)
            && self
                .last_attempt_us
                .is_none_or(|time| now - time >= RETRY_US)
    }

    pub(super) fn begin(&mut self, now: i64) {
        self.last_attempt_us = Some(now);
        self.cancel = Some(gio::Cancellable::new());
        self.error = None;
    }
}

impl SharedPickerCache {
    pub(super) fn invalidate(&self) {
        let mut live = self.live.borrow_mut();
        if let Some(cancel) = live.cancel.take() {
            cancel.cancel();
        }
        live.generation = live.generation.wrapping_add(1);
        live.previews = None;
        live.error = None;
        live.last_attempt_us = None;
        live.waiters.clear();
    }
}

pub(super) struct WaitingPicker {
    pub(super) switcher: *mut ChannelSwitcherOverlay,
    pub(super) generation: u32,
}

impl WaitingPicker {
    pub(super) unsafe fn new(switcher: *mut ChannelSwitcherOverlay) -> Self {
        unsafe {
            Self {
                switcher: channel_switcher_overlay_ref(switcher),
                generation: (*switcher).generation,
            }
        }
    }
}

impl Drop for WaitingPicker {
    fn drop(&mut self) {
        unsafe {
            channel_switcher_overlay_unref(self.switcher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tiles_share_one_snapshot_and_in_flight_request() {
        let first = SharedPickerCache::default();
        let second = first.clone();
        assert!(first.live.borrow().should_fetch(0));
        first.live.borrow_mut().begin(0);
        assert!(!second.live.borrow().should_fetch(1));
        {
            let mut live = first.live.borrow_mut();
            live.cancel = None;
            live.previews = Some(Vec::new()); // Offline is also a cacheable result.
            live.cached_at_us = 1;
        }
        assert!(!second.live.borrow().should_fetch(LIVE_CACHE_US));
        assert!(second.live.borrow().should_fetch(LIVE_CACHE_US + 1));
    }

    #[test]
    fn settings_changes_cancel_old_work_and_allow_an_immediate_refresh() {
        let cache = SharedPickerCache::default();
        cache.live.borrow_mut().begin(0);
        let cancel = cache.live.borrow().cancel.clone().unwrap();
        cache.invalidate();
        assert!(cancel.is_cancelled());
        assert_eq!(cache.live.borrow().generation, 1);
        assert!(cache.live.borrow().should_fetch(1));
    }

    #[test]
    fn failures_back_off_without_discarding_the_previous_snapshot() {
        let mut live = LiveCache::default();
        live.previews = Some(Vec::new());
        live.begin(LIVE_CACHE_US);
        live.cancel = None;
        assert!(!live.should_fetch(LIVE_CACHE_US + 1));
        assert!(live.previews.is_some());
        assert!(live.should_fetch(LIVE_CACHE_US + RETRY_US));
    }
}
