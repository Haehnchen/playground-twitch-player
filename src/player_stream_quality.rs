use crate::glib_strings::clear_string;
use std::ffi::{c_char, c_int, c_uint, c_void};

use std::ptr;

use crate::twitch_stream_info::{GCancellable, GPtrArray, TwitchStreamQuality};

const G_USEC_PER_SEC: i64 = 1_000_000;

pub struct PlayerStreamQualityState {
    pub cancel: *mut GCancellable,
    pub qualities: *mut GPtrArray,
    pub selected_url: *mut c_char,
    pub selected_label: *mut c_char,
    fetched_at: i64,
    pub generation: c_uint,
    pub fetch_in_progress: c_int,
}

impl PlayerStreamQualityState {
    pub const fn new() -> Self {
        Self {
            cancel: ptr::null_mut(),
            qualities: ptr::null_mut(),
            selected_url: ptr::null_mut(),
            selected_label: ptr::null_mut(),
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

unsafe extern "C" {
    fn g_cancellable_new() -> *mut GCancellable;
    fn g_cancellable_cancel(cancellable: *mut GCancellable);
    fn g_free(mem: *mut c_void);
    fn g_get_monotonic_time() -> i64;
    fn g_object_unref(object: *mut c_void);
    fn g_ptr_array_unref(array: *mut GPtrArray);
    fn g_strdup(str: *const c_char) -> *mut c_char;
}

unsafe fn clear_object(object: &mut *mut GCancellable) {
    unsafe {
        if !object.is_null() {
            let old_object = *object;
            *object = ptr::null_mut();
            g_object_unref(old_object as *mut c_void);
        }
    }
}

unsafe fn clear_ptr_array(array: &mut *mut GPtrArray) {
    unsafe {
        if !array.is_null() {
            let old_array = *array;
            *array = ptr::null_mut();
            g_ptr_array_unref(old_array);
        }
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
        clear_ptr_array(&mut state.qualities);
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
        (!state.qualities.is_null()
            && state.fetched_at > 0
            && g_get_monotonic_time() - state.fetched_at < max_age_seconds as i64 * G_USEC_PER_SEC)
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
        g_free(state.selected_url as *mut c_void);
        g_free(state.selected_label as *mut c_void);
        state.selected_url = g_strdup(quality.url);
        state.selected_label = g_strdup(quality.label);
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
        clear_string(&mut state.selected_url);
        clear_string(&mut state.selected_label);
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
        if !state.cancel.is_null() {
            g_cancellable_cancel(state.cancel);
            clear_object(&mut state.cancel);
        }
        state.fetch_in_progress = 0;
    }
}

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
        state.cancel = g_cancellable_new();
        state.fetch_in_progress = 1;
        state.generation = state.generation.wrapping_add(1);
        state.generation
    }
}

/// # Safety
/// - `state`: null or initialized instance with valid owned fields; serialize access.
/// - `qualities`: null or owned array of live quality entries; ownership transfers.
/// - Must not alias the array already owned by `state`.
pub unsafe fn player_stream_quality_state_finish_fetch(
    state: *mut PlayerStreamQualityState,
    qualities: *mut GPtrArray,
) {
    unsafe {
        if state.is_null() {
            if !qualities.is_null() {
                g_ptr_array_unref(qualities);
            }
            return;
        }

        let state = &mut *state;
        state.fetch_in_progress = 0;
        clear_object(&mut state.cancel);
        clear_ptr_array(&mut state.qualities);
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

        (*state).fetched_at = g_get_monotonic_time();
    }
}
