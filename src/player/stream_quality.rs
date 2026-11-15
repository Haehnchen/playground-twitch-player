use gtk4::{gio, glib, prelude::*};
use std::ffi::{CString, c_int, c_uint};

use crate::twitch::stream_info::TwitchStreamQuality;

const G_USEC_PER_SEC: i64 = 1_000_000;

pub struct PlayerStreamQualityState {
    pub cancel: Option<gio::Cancellable>,
    pub qualities: Option<Vec<TwitchStreamQuality>>,

    pub selected_url: Option<CString>,
    pub selected_label: Option<CString>,

    fetched_at: i64,
    pub generation: c_uint,
    pub fetch_in_progress: c_int,
}

impl PlayerStreamQualityState {
    pub const fn new() -> Self {
        Self {
            cancel: None,
            qualities: None,

            selected_url: None,
            selected_label: None,

            fetched_at: 0,
            generation: 0,
            fetch_in_progress: 0,
        }
    }
}

impl Default for PlayerStreamQualityState {
    fn default() -> Self {
        Self::new()
    }
}

/// # Safety
/// - `state`: null or initialized instance with valid owned fields; serialize access.
pub unsafe fn player_stream_quality_state_clear(state: *mut PlayerStreamQualityState) {
    unsafe {
        if state.is_null() {
            return;
        }

        player_stream_quality_state_cancel_fetch(state);
        let state = &mut *state;
        state.qualities = None;
        player_stream_quality_state_select_auto(state);
        state.fetched_at = 0;
        state.generation = state.generation.wrapping_add(1);
    }
}

/// # Safety
/// - `state`: null or initialized instance with valid owned fields; serialize access.
pub unsafe fn player_stream_quality_state_reset_selection(state: *mut PlayerStreamQualityState) {
    unsafe {
        if state.is_null() {
            return;
        }

        player_stream_quality_state_clear(state);
    }
}

/// # Safety
/// - `state`: null or initialized instance with valid owned fields; serialize access.
pub unsafe fn player_stream_quality_state_cache_is_valid(
    state: *mut PlayerStreamQualityState,
    max_age_seconds: c_uint,
) -> c_int {
    unsafe {
        if state.is_null() {
            return 0;
        }

        let state = &mut *state;
        (state.qualities.is_some()
            && state.fetched_at > 0
            && glib::monotonic_time() - state.fetched_at < max_age_seconds as i64 * G_USEC_PER_SEC)
            as c_int
    }
}

/// # Safety
/// - `state`: null or initialized instance with valid owned fields; serialize access.
/// - `quality`: null or live entry with valid C strings, not aliasing selected strings.
pub unsafe fn player_stream_quality_state_select(
    state: *mut PlayerStreamQualityState,
    quality: *const TwitchStreamQuality,
) {
    unsafe {
        if state.is_null() || quality.is_null() {
            return;
        }

        let state = &mut *state;
        let quality = &*quality;
        state.selected_url = Some(quality.url.clone());
        state.selected_label = Some(quality.label.clone());
    }
}

/// # Safety
/// - `state`: null or initialized instance with valid owned fields; serialize access.
pub unsafe fn player_stream_quality_state_select_auto(state: *mut PlayerStreamQualityState) {
    unsafe {
        if state.is_null() {
            return;
        }

        let state = &mut *state;
        state.selected_url = None;
        state.selected_label = None;
    }
}

/// # Safety
/// - `state`: null or initialized instance with valid owned fields; serialize access.
pub unsafe fn player_stream_quality_state_cancel_fetch(state: *mut PlayerStreamQualityState) {
    unsafe {
        if state.is_null() {
            return;
        }

        let state = &mut *state;
        if let Some(cancel) = state.cancel.take() {
            cancel.cancel();
        }
        state.fetch_in_progress = 0;
    }
}

/// Assign a generation so callbacks can reject superseded requests.
///
/// # Safety
/// - `state`: null or initialized instance with valid owned fields; serialize access.
pub unsafe fn player_stream_quality_state_begin_fetch(
    state: *mut PlayerStreamQualityState,
) -> c_uint {
    unsafe {
        if state.is_null() {
            return 0;
        }

        let state = &mut *state;
        state.cancel = Some(gio::Cancellable::new());
        state.fetch_in_progress = 1;
        state.generation = state.generation.wrapping_add(1);
        state.generation
    }
}

/// Take ownership of the fetched qualities and release the previous list.
///
/// # Safety
/// - `state`: null or initialized instance with valid owned fields; serialize access.
/// - `qualities`: null or owned array of live quality entries; ownership transfers.
/// - Must not alias the array already owned by `state`.
pub unsafe fn player_stream_quality_state_finish_fetch(
    state: *mut PlayerStreamQualityState,
    qualities: Option<Vec<TwitchStreamQuality>>,
) {
    unsafe {
        if state.is_null() {
            return;
        }

        let state = &mut *state;
        state.fetch_in_progress = 0;
        state.cancel = None;
        state.qualities = None;
        state.qualities = qualities;
    }
}

/// # Safety
/// - `state`: null or initialized instance with valid owned fields; serialize access.
pub unsafe fn player_stream_quality_state_mark_fetched(state: *mut PlayerStreamQualityState) {
    unsafe {
        if state.is_null() {
            return;
        }

        (*state).fetched_at = glib::monotonic_time();
    }
}
