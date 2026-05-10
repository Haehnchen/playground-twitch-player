use std::ffi::{c_char, CStr, CString};
use std::ptr;
use std::slice;
use twitch_player_core::chat_assets;

unsafe extern "C" {
    fn g_array_unref(array: *mut chat_assets::GArray);
}

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn cstr_eq(actual: *const c_char, expected: &str) {
    assert!(!actual.is_null());
    assert_eq!(CStr::from_ptr(actual).to_str().unwrap(), expected);
}

unsafe fn parse_emote_ranges(value: *const c_char) -> *mut chat_assets::GArray {
    chat_assets::chat_assets_test_parse_emote_ranges(value)
}

unsafe fn ranges_slice<'a>(ranges: *mut chat_assets::GArray) -> &'a [chat_assets::EmoteRange] {
    slice::from_raw_parts(
        (*ranges).data as *const chat_assets::EmoteRange,
        (*ranges).len as usize,
    )
}

unsafe fn test_parse_emote_ranges_returns_sorted_ranges() {
    let input = cstring("1902:12-16/25:0-4,6-10");
    let ranges = parse_emote_ranges(input.as_ptr());

    assert!(!ranges.is_null());
    assert_eq!((*ranges).len, 3);

    let ranges_ref = ranges_slice(ranges);
    assert_eq!(ranges_ref[0].start, 0);
    assert_eq!(ranges_ref[0].end, 4);
    cstr_eq(ranges_ref[0].id, "25");
    assert_eq!(ranges_ref[1].start, 6);
    assert_eq!(ranges_ref[1].end, 10);
    cstr_eq(ranges_ref[1].id, "25");
    assert_eq!(ranges_ref[2].start, 12);
    assert_eq!(ranges_ref[2].end, 16);
    cstr_eq(ranges_ref[2].id, "1902");

    g_array_unref(ranges);
}

unsafe fn test_parse_emote_ranges_ignores_invalid_specs() {
    let input = cstring("bad/25:4-2,abc-5,1-x/33:2-3");
    let ranges = parse_emote_ranges(input.as_ptr());

    assert!(!ranges.is_null());
    assert_eq!((*ranges).len, 1);

    let ranges_ref = ranges_slice(ranges);
    assert_eq!(ranges_ref[0].start, 2);
    assert_eq!(ranges_ref[0].end, 3);
    cstr_eq(ranges_ref[0].id, "33");

    g_array_unref(ranges);
}

unsafe fn test_parse_emote_ranges_returns_null_for_empty_input() {
    assert!(parse_emote_ranges(ptr::null()).is_null());
    assert!(parse_emote_ranges(cstring("").as_ptr()).is_null());
    assert!(parse_emote_ranges(cstring("bad/also-bad").as_ptr()).is_null());
}

unsafe fn test_utf8_offset_to_pointer_safe() {
    let text = b"a\xc3\xa4b\0";
    let ptr = text.as_ptr() as *const c_char;

    assert_eq!(
        chat_assets::chat_assets_test_utf8_offset_to_pointer_safe(ptr, 0),
        ptr
    );
    cstr_eq(
        chat_assets::chat_assets_test_utf8_offset_to_pointer_safe(ptr, 1),
        "äb",
    );
    cstr_eq(
        chat_assets::chat_assets_test_utf8_offset_to_pointer_safe(ptr, 2),
        "b",
    );
    cstr_eq(
        chat_assets::chat_assets_test_utf8_offset_to_pointer_safe(ptr, 3),
        "",
    );
    assert!(chat_assets::chat_assets_test_utf8_offset_to_pointer_safe(ptr, 4).is_null());
}

fn main() {
    unsafe {
        test_parse_emote_ranges_returns_sorted_ranges();
        test_parse_emote_ranges_ignores_invalid_specs();
        test_parse_emote_ranges_returns_null_for_empty_input();
        test_utf8_offset_to_pointer_safe();
    }
}
