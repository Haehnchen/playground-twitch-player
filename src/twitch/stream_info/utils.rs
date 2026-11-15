use super::ffi::{
    GPtrArray, TwitchStreamQuality, g_date_time_difference, g_date_time_new_from_iso8601,
    g_date_time_new_now_utc, g_date_time_unref, g_free, g_ptr_array_unref, g_strdup,
};
use super::{
    TwitchCurrentStream, TwitchFollowedChannel, TwitchStreamPreview, TwitchStreamQualitiesResult,
};
use crate::glib_strings::{dup_bytes, is_nonempty};
use std::ffi::{CStr, c_char, c_uint, c_void};
use std::ptr;

pub(super) unsafe extern "C" fn g_ptr_array_unref_destroy(data: *mut c_void) {
    unsafe {
        g_ptr_array_unref(data as *mut GPtrArray);
    }
}

pub(super) unsafe extern "C" fn twitch_stream_qualities_result_free(data: *mut c_void) {
    unsafe {
        let result = data as *mut TwitchStreamQualitiesResult;
        if result.is_null() {
            return;
        }
        if !(*result).qualities.is_null() {
            g_ptr_array_unref((*result).qualities);
        }
        drop(Box::from_raw(result));
    }
}

pub(super) unsafe extern "C" fn twitch_stream_preview_free_destroy(data: *mut c_void) {
    unsafe {
        twitch_stream_preview_free(data as *mut TwitchStreamPreview);
    }
}

pub(super) unsafe extern "C" fn twitch_current_stream_free_destroy(data: *mut c_void) {
    unsafe {
        twitch_current_stream_free(data as *mut TwitchCurrentStream);
    }
}

/// # Safety
/// - Null or owned module result; original GLib-allocated string fields.
/// - Free once; no remaining borrowers. Not a stack struct or borrowed array entry.
pub unsafe fn twitch_stream_preview_free(preview: *mut TwitchStreamPreview) {
    unsafe {
        if preview.is_null() {
            return;
        }
        g_free((*preview).channel as *mut c_void);
        g_free((*preview).display_name as *mut c_void);
        g_free((*preview).title as *mut c_void);
        g_free((*preview).avatar_url as *mut c_void);
        g_free((*preview).preview_url as *mut c_void);
        g_free((*preview).started_at as *mut c_void);
        g_free((*preview).category_name as *mut c_void);
        drop(Box::from_raw(preview));
    }
}

/// # Safety
/// - Null or owned module result; original GLib-allocated string fields.
/// - Free once; no remaining borrowers. Not a stack struct or borrowed array entry.
pub unsafe fn twitch_current_stream_free(stream: *mut TwitchCurrentStream) {
    unsafe {
        if stream.is_null() {
            return;
        }
        g_free((*stream).title as *mut c_void);
        g_free((*stream).started_at as *mut c_void);
        g_free((*stream).category_name as *mut c_void);
        drop(Box::from_raw(stream));
    }
}

pub(super) unsafe extern "C" fn twitch_stream_quality_free_destroy(data: *mut c_void) {
    unsafe {
        twitch_stream_quality_free(data as *mut TwitchStreamQuality);
    }
}

/// # Safety
/// - Null or owned module result; original GLib-allocated string fields.
/// - Free once; no remaining borrowers. Not a stack struct or borrowed array entry.
pub unsafe fn twitch_stream_quality_free(quality: *mut TwitchStreamQuality) {
    unsafe {
        if quality.is_null() {
            return;
        }
        g_free((*quality).label as *mut c_void);
        g_free((*quality).url as *mut c_void);
        drop(Box::from_raw(quality));
    }
}

/// # Safety
/// - Null or owned module result; original GLib-allocated string fields.
/// - Free once; no remaining borrowers. Not a stack struct or borrowed array entry.
pub unsafe fn twitch_followed_channel_free(channel: *mut TwitchFollowedChannel) {
    unsafe {
        if channel.is_null() {
            return;
        }
        g_free((*channel).channel as *mut c_void);
        g_free((*channel).display_name as *mut c_void);
        drop(Box::from_raw(channel));
    }
}

pub(super) unsafe extern "C" fn twitch_followed_channel_free_destroy(data: *mut c_void) {
    unsafe {
        twitch_followed_channel_free(data as *mut TwitchFollowedChannel);
    }
}

pub(super) fn format_viewer_count(viewer_count: c_uint) -> String {
    if viewer_count >= 1_000_000 {
        format!("{:.1}M", viewer_count as f64 / 1_000_000.0)
    } else if viewer_count >= 1000 {
        format!("{:.1}K", viewer_count as f64 / 1000.0)
    } else {
        viewer_count.to_string()
    }
}

/// Result owned; release with `g_free`.
pub fn twitch_stream_info_format_viewer_count(viewer_count: c_uint) -> *mut c_char {
    dup_bytes(format_viewer_count(viewer_count).as_bytes())
}

pub(super) fn format_live_duration_from_span(span: i64) -> String {
    let total_minutes = span.max(0) / 60_000_000;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

/// # Safety
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
/// - Result owned; release with `g_free`.
pub unsafe fn twitch_stream_info_format_live_duration(started_at: *const c_char) -> *mut c_char {
    unsafe {
        if !is_nonempty(started_at) {
            return ptr::null_mut();
        }

        let started = g_date_time_new_from_iso8601(started_at, ptr::null_mut());
        if started.is_null() {
            return ptr::null_mut();
        }
        let now = g_date_time_new_now_utc();
        let result = format_live_duration_from_span(g_date_time_difference(now, started));
        g_date_time_unref(now);
        g_date_time_unref(started);
        dup_bytes(result.as_bytes())
    }
}

/// # Safety
/// - `stream`: null or live struct; string fields valid C strings, unchanged during call.
/// - Result owned; release with `g_free`.
pub unsafe fn twitch_stream_info_format_current_stream_title(
    stream: *const TwitchCurrentStream,
) -> *mut c_char {
    unsafe {
        if stream.is_null() || !is_nonempty((*stream).title) {
            return g_strdup(c"".as_ptr());
        }
        g_strdup((*stream).title)
    }
}

pub(super) unsafe fn append_metadata_segment(metadata: &mut Vec<u8>, segment: *const c_char) {
    unsafe {
        if !is_nonempty(segment) {
            return;
        }
        if !metadata.is_empty() {
            metadata.extend_from_slice(b" \xE2\x80\xA2 ");
        }
        metadata.extend_from_slice(CStr::from_ptr(segment).to_bytes());
    }
}

/// # Safety
/// - `stream`: null or live struct; string fields valid C strings, unchanged during call.
/// - Result owned; release with `g_free`.
pub unsafe fn twitch_stream_info_format_current_stream_metadata(
    stream: *const TwitchCurrentStream,
) -> *mut c_char {
    unsafe {
        if stream.is_null() {
            return g_strdup(c"".as_ptr());
        }

        let viewers = twitch_stream_info_format_viewer_count((*stream).viewer_count);
        let duration = twitch_stream_info_format_live_duration((*stream).started_at);
        let mut metadata = Vec::new();
        append_metadata_segment(&mut metadata, viewers);
        append_metadata_segment(&mut metadata, duration);
        append_metadata_segment(&mut metadata, (*stream).category_name);
        g_free(viewers as *mut c_void);
        g_free(duration as *mut c_void);
        dup_bytes(&metadata)
    }
}
