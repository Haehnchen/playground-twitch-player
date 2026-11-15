use super::ffi::{
    G_IO_ERROR_NOT_FOUND, GError, GPtrArray, TwitchStreamQuality, g_ascii_strcasecmp,
    g_io_error_quark, g_ptr_array_add, g_ptr_array_new_with_free_func, g_ptr_array_sort,
    g_ptr_array_unref, g_set_error,
};
use super::utils::twitch_stream_quality_free_destroy;
use crate::glib_strings::dup_bytes;
use std::ffi::{CStr, c_char, c_double, c_int, c_uint, c_void};
use std::ptr;

pub(super) fn build_quality_label(
    _width: c_uint,
    height: c_uint,
    frame_rate: c_double,
    bandwidth: c_uint,
) -> *mut c_char {
    let label = if height > 0 {
        if frame_rate >= 50.0 {
            format!("{height}p60")
        } else {
            format!("{height}p")
        }
    } else if bandwidth > 0 {
        format!("{} kbps", std::cmp::max(1, bandwidth / 1000))
    } else {
        "Unknown".to_string()
    };
    dup_bytes(label.as_bytes())
}

pub(super) unsafe extern "C" fn compare_stream_qualities(
    a: *const c_void,
    b: *const c_void,
) -> c_int {
    unsafe {
        let quality_a = *(a as *const *const TwitchStreamQuality);
        let quality_b = *(b as *const *const TwitchStreamQuality);

        if (*quality_a).height != (*quality_b).height {
            return if (*quality_a).height < (*quality_b).height {
                -1
            } else {
                1
            };
        }
        if (*quality_a).frame_rate != (*quality_b).frame_rate {
            return if (*quality_a).frame_rate < (*quality_b).frame_rate {
                -1
            } else {
                1
            };
        }
        if (*quality_a).bandwidth != (*quality_b).bandwidth {
            return if (*quality_a).bandwidth < (*quality_b).bandwidth {
                -1
            } else {
                1
            };
        }

        g_ascii_strcasecmp((*quality_a).label, (*quality_b).label)
    }
}

// Read HLS variants and sort them for the quality picker.
pub(super) unsafe fn parse_stream_qualities_playlist(
    playlist: *const c_char,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    unsafe {
        let qualities = g_ptr_array_new_with_free_func(Some(twitch_stream_quality_free_destroy));
        let playlist = if playlist.is_null() {
            &[][..]
        } else {
            CStr::from_ptr(playlist).to_bytes()
        };
        let parsed = m3u8_rs::parse_master_playlist_res(playlist);
        let playlist = match parsed {
            Ok(playlist) => playlist,
            Err(_) => {
                g_set_error(
                    error,
                    g_io_error_quark(),
                    G_IO_ERROR_NOT_FOUND,
                    c"Invalid HLS master playlist".as_ptr(),
                );
                g_ptr_array_unref(qualities);
                return ptr::null_mut();
            }
        };
        for variant in playlist.variants {
            let Some(resolution) = variant.resolution else {
                continue;
            };
            let audio_only = variant
                .other_attributes
                .as_ref()
                .and_then(|attrs| attrs.get("NAME"))
                .is_some_and(|name| name.to_string().to_ascii_lowercase().contains("audio"));
            let (Ok(width), Ok(height)) = (
                c_uint::try_from(resolution.width),
                c_uint::try_from(resolution.height),
            ) else {
                continue;
            };
            if height == 0 || variant.is_i_frame || audio_only {
                continue;
            }
            let bandwidth = variant.bandwidth.try_into().unwrap_or(0);
            let frame_rate = variant.frame_rate.unwrap_or(0.0);
            let quality = Box::new(TwitchStreamQuality {
                label: build_quality_label(width, height, frame_rate, bandwidth),
                url: dup_bytes(variant.uri.as_bytes()),
                width,
                height,
                bandwidth,
                frame_rate,
            });
            g_ptr_array_add(qualities, Box::into_raw(quality).cast());
        }

        g_ptr_array_sort(qualities, Some(compare_stream_qualities));

        if (*qualities).len == 0 {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_NOT_FOUND,
                c"No HLS stream variants found".as_ptr(),
            );
            g_ptr_array_unref(qualities);
            return ptr::null_mut();
        }

        qualities
    }
}
