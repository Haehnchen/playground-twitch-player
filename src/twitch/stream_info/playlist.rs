use super::ffi::{
    G_IO_ERROR_NOT_FOUND, GError, GPtrArray, TwitchStreamQuality, g_ascii_strcasecmp,
    g_ascii_strtod, g_ascii_strtoull, g_free, g_io_error_quark, g_ptr_array_add,
    g_ptr_array_new_with_free_func, g_ptr_array_sort, g_ptr_array_unref, g_set_error,
};
use super::utils::twitch_stream_quality_free_destroy;
use crate::glib_strings::dup_bytes;
use std::ffi::{CStr, c_char, c_double, c_int, c_uint, c_void};
use std::ptr;

pub(super) fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|idx| idx + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

pub(super) fn get_m3u_attribute_value_bytes(line: &[u8], name: &[u8]) -> Option<Vec<u8>> {
    let mut needle = Vec::with_capacity(name.len() + 1);
    needle.extend_from_slice(name);
    needle.push(b'=');
    let start = line
        .windows(needle.len())
        .position(|window| window == needle.as_slice())?;
    let mut value_start = start + needle.len();

    if line.get(value_start) == Some(&b'"') {
        value_start += 1;
        let end = line[value_start..]
            .iter()
            .position(|byte| *byte == b'"')
            .map(|idx| value_start + idx)
            .unwrap_or(line.len());
        return Some(line[value_start..end].to_vec());
    }

    let end = line[value_start..]
        .iter()
        .position(|byte| *byte == b',')
        .map(|idx| value_start + idx)
        .unwrap_or(line.len());
    Some(line[value_start..end].to_vec())
}

pub(super) unsafe fn parse_uint_attribute(line: &[u8], name: &[u8]) -> c_uint {
    unsafe {
        let Some(value) = get_m3u_attribute_value_bytes(line, name) else {
            return 0;
        };
        if value.is_empty() {
            return 0;
        }

        let value = dup_bytes(&value);
        let parsed = g_ascii_strtoull(value, ptr::null_mut(), 10);
        g_free(value as *mut c_void);
        if parsed <= c_uint::MAX as u64 {
            parsed as c_uint
        } else {
            0
        }
    }
}

pub(super) unsafe fn parse_double_attribute(line: &[u8], name: &[u8]) -> c_double {
    unsafe {
        let Some(value) = get_m3u_attribute_value_bytes(line, name) else {
            return 0.0;
        };
        if value.is_empty() {
            return 0.0;
        }

        let value = dup_bytes(&value);
        let parsed = g_ascii_strtod(value, ptr::null_mut());
        g_free(value as *mut c_void);
        parsed
    }
}

pub(super) unsafe fn parse_resolution_attribute(
    line: &[u8],
    width: *mut c_uint,
    height: *mut c_uint,
) {
    unsafe {
        let Some(resolution) = get_m3u_attribute_value_bytes(line, b"RESOLUTION") else {
            return;
        };
        let Some(separator) = resolution.iter().position(|byte| *byte == b'x') else {
            return;
        };

        let width_text = dup_bytes(&resolution[..separator]);
        let height_text = dup_bytes(&resolution[separator + 1..]);
        let parsed_width = g_ascii_strtoull(width_text, ptr::null_mut(), 10);
        let parsed_height = g_ascii_strtoull(height_text, ptr::null_mut(), 10);
        g_free(width_text as *mut c_void);
        g_free(height_text as *mut c_void);

        if parsed_width <= c_uint::MAX as u64 && parsed_height <= c_uint::MAX as u64 {
            *width = parsed_width as c_uint;
            *height = parsed_height as c_uint;
        }
    }
}

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
        let mut pending_stream_info: Option<Vec<u8>> = None;

        for raw_line in playlist.split(|byte| *byte == b'\n') {
            let line = trim_ascii(raw_line);

            if line.starts_with(b"#EXT-X-STREAM-INF:") {
                pending_stream_info = Some(line.to_vec());
                continue;
            }

            if pending_stream_info.is_none() || line.is_empty() || line[0] == b'#' {
                continue;
            }

            let pending = pending_stream_info.take().unwrap();
            let mut quality = Box::new(TwitchStreamQuality {
                label: ptr::null_mut(),
                url: ptr::null_mut(),
                width: 0,
                height: 0,
                bandwidth: parse_uint_attribute(&pending, b"BANDWIDTH"),
                frame_rate: parse_double_attribute(&pending, b"FRAME-RATE"),
            });
            parse_resolution_attribute(&pending, &mut quality.width, &mut quality.height);

            let name = get_m3u_attribute_value_bytes(&pending, b"NAME");
            let lower_name = name.map(|mut value| {
                value.make_ascii_lowercase();
                value
            });
            if quality.height == 0
                || lower_name
                    .as_ref()
                    .is_some_and(|name| name.windows(5).any(|window| window == b"audio"))
            {
                continue;
            }

            quality.label = build_quality_label(
                quality.width,
                quality.height,
                quality.frame_rate,
                quality.bandwidth,
            );
            quality.url = dup_bytes(line);
            g_ptr_array_add(qualities, Box::into_raw(quality) as *mut c_void);
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
