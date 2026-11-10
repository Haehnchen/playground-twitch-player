use crate::glib_strings::dup_bytes;
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};

use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const CHAT_EMOTE_SIZE: c_int = 14;
const GTK_CONTENT_FIT_CONTAIN: c_int = 1;

pub struct ChatAssets {
    image_cache: *mut GHashTable,
    ref_count: AtomicUsize,
    closing: AtomicBool,
}

pub struct ImageLoadData {
    picture: *mut GtkPicture,
    assets: *mut ChatAssets,
    url: *mut c_char,
}

pub struct EmoteRange {
    pub start: c_uint,
    pub end: c_uint,
    pub id: *mut c_char,
}

#[repr(C)]
pub struct GArray {
    pub data: *mut c_char,
    pub len: c_uint,
}

#[repr(C)]
pub struct GAsyncResult {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GBytes {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GCancellable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkPaintable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkTexture {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GError {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GFile {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GHashTable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GObject {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkPicture {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextBuffer {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextChildAnchor {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextIter {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextView {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

type GAsyncReadyCallback = unsafe extern "C" fn(*mut GObject, *mut GAsyncResult, *mut c_void);
type GCompareFunc = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;
type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

unsafe extern "C" {

    fn g_array_append_vals(array: *mut GArray, data: *const c_void, len: c_uint) -> *mut GArray;
    fn g_array_new(zero_terminated: c_int, clear_: c_int, element_size: c_uint) -> *mut GArray;
    fn g_array_set_clear_func(array: *mut GArray, clear_func: Option<GDestroyNotify>);
    fn g_array_sort(array: *mut GArray, compare_func: Option<GCompareFunc>);
    fn g_array_unref(array: *mut GArray);
    fn g_bytes_new_take(data: *mut c_void, size: usize) -> *mut GBytes;
    fn g_bytes_unref(bytes: *mut GBytes);
    fn g_clear_error(error: *mut *mut GError);
    fn g_file_load_contents_async(
        file: *mut GFile,
        cancellable: *mut GCancellable,
        callback: Option<GAsyncReadyCallback>,
        user_data: *mut c_void,
    );
    fn g_file_load_contents_finish(
        file: *mut GFile,
        res: *mut GAsyncResult,
        contents: *mut *mut c_char,
        length: *mut usize,
        etag_out: *mut *mut c_char,
        error: *mut *mut GError,
    ) -> c_int;
    fn g_file_new_for_uri(uri: *const c_char) -> *mut GFile;
    fn g_free(mem: *mut c_void);
    fn g_hash_table_destroy(hash_table: *mut GHashTable);
    fn g_hash_table_insert(
        hash_table: *mut GHashTable,
        key: *mut c_void,
        value: *mut c_void,
    ) -> c_int;
    fn g_hash_table_lookup(hash_table: *mut GHashTable, key: *const c_void) -> *mut c_void;
    fn g_hash_table_new_full(
        hash_func: Option<unsafe extern "C" fn(*const c_void) -> c_uint>,
        key_equal_func: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
        key_destroy_func: Option<GDestroyNotify>,
        value_destroy_func: Option<GDestroyNotify>,
    ) -> *mut GHashTable;
    fn g_object_ref(object: *mut c_void) -> *mut c_void;
    fn g_object_unref(object: *mut c_void);
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn g_str_equal(v1: *const c_void, v2: *const c_void) -> c_int;
    fn g_str_hash(v: *const c_void) -> c_uint;

    fn gdk_texture_new_from_bytes(bytes: *mut GBytes, error: *mut *mut GError) -> *mut GdkTexture;
    fn gtk_picture_new() -> *mut GtkWidget;
    fn gtk_picture_set_can_shrink(self_: *mut GtkPicture, can_shrink: c_int);
    fn gtk_picture_set_content_fit(self_: *mut GtkPicture, content_fit: c_int);
    fn gtk_picture_set_paintable(self_: *mut GtkPicture, paintable: *mut GdkPaintable);
    fn gtk_text_buffer_insert(
        buffer: *mut GtkTextBuffer,
        iter: *mut GtkTextIter,
        text: *const c_char,
        len: c_int,
    );
    fn gtk_text_buffer_insert_child_anchor(
        buffer: *mut GtkTextBuffer,
        iter: *mut GtkTextIter,
        anchor: *mut GtkTextChildAnchor,
    );
    fn gtk_text_child_anchor_new() -> *mut GtkTextChildAnchor;
    fn gtk_text_view_add_child_at_anchor(
        text_view: *mut GtkTextView,
        child: *mut GtkWidget,
        anchor: *mut GtkTextChildAnchor,
    );
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    fn gtk_widget_set_focusable(widget: *mut GtkWidget, focusable: c_int);
    fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
}

unsafe fn chat_assets_ref(assets: *mut ChatAssets) -> *mut ChatAssets {
    unsafe {
        if !assets.is_null() {
            (*assets).ref_count.fetch_add(1, Ordering::Relaxed);
        }
        assets
    }
}

unsafe fn chat_assets_unref(assets: *mut ChatAssets) {
    unsafe {
        if assets.is_null() || (*assets).ref_count.fetch_sub(1, Ordering::AcqRel) != 1 {
            return;
        }

        if !(*assets).image_cache.is_null() {
            g_hash_table_destroy((*assets).image_cache);
            (*assets).image_cache = ptr::null_mut();
        }
        drop(Box::from_raw(assets));
    }
}

unsafe fn image_load_data_free(data: *mut ImageLoadData) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data);
        if !data.picture.is_null() {
            g_object_unref(data.picture as *mut c_void);
        }
        g_free(data.url as *mut c_void);
        chat_assets_unref(data.assets);
    }
}

unsafe extern "C" fn on_image_loaded(
    source: *mut GObject,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let data = user_data as *mut ImageLoadData;
        let mut error: *mut GError = ptr::null_mut();
        let mut contents: *mut c_char = ptr::null_mut();
        let mut length: usize = 0;

        if g_file_load_contents_finish(
            source as *mut GFile,
            result,
            &mut contents,
            &mut length,
            ptr::null_mut(),
            &mut error,
        ) == 0
        {
            g_clear_error(&mut error);
            image_load_data_free(data);
            return;
        }

        let bytes = g_bytes_new_take(contents as *mut c_void, length);
        let texture = gdk_texture_new_from_bytes(bytes, &mut error);
        g_bytes_unref(bytes);
        if texture.is_null() {
            g_clear_error(&mut error);
            image_load_data_free(data);
            return;
        }

        if !(*data).assets.is_null()
            && !(*(*data).assets).closing.load(Ordering::Acquire)
            && !(*(*data).assets).image_cache.is_null()
        {
            g_hash_table_insert(
                (*(*data).assets).image_cache,
                g_strdup((*data).url) as *mut c_void,
                g_object_ref(texture as *mut c_void),
            );
            gtk_picture_set_paintable((*data).picture, texture as *mut GdkPaintable);
        }
        g_object_unref(texture as *mut c_void);
        image_load_data_free(data);
    }
}

unsafe fn create_inline_image(assets: *mut ChatAssets, url: *const c_char) -> *mut GtkWidget {
    unsafe {
        let picture = gtk_picture_new();
        gtk_widget_add_css_class(picture, c"chat-emote".as_ptr());
        gtk_widget_set_focusable(picture, 0);
        gtk_widget_set_size_request(picture, CHAT_EMOTE_SIZE, CHAT_EMOTE_SIZE);
        gtk_picture_set_content_fit(picture as *mut GtkPicture, GTK_CONTENT_FIT_CONTAIN);
        gtk_picture_set_can_shrink(picture as *mut GtkPicture, 1);

        let cached =
            g_hash_table_lookup((*assets).image_cache, url as *const c_void) as *mut GdkPaintable;
        if !cached.is_null() {
            gtk_picture_set_paintable(picture as *mut GtkPicture, cached);
            return picture;
        }

        let data = Box::new(ImageLoadData {
            picture: g_object_ref(picture as *mut c_void) as *mut GtkPicture,
            assets: chat_assets_ref(assets),
            url: g_strdup(url),
        });
        let data = Box::into_raw(data);

        let file = g_file_new_for_uri(url);
        g_file_load_contents_async(
            file,
            ptr::null_mut(),
            Some(on_image_loaded),
            data as *mut c_void,
        );
        g_object_unref(file as *mut c_void);

        picture
    }
}

unsafe fn insert_emote(
    assets: *mut ChatAssets,
    buffer: *mut GtkTextBuffer,
    view: *mut GtkTextView,
    iter: *mut GtkTextIter,
    url: *const c_char,
) {
    unsafe {
        let anchor = gtk_text_child_anchor_new();

        gtk_text_buffer_insert_child_anchor(buffer, iter, anchor);
        gtk_text_view_add_child_at_anchor(view, create_inline_image(assets, url), anchor);
        g_object_unref(anchor as *mut c_void);
    }
}

unsafe extern "C" fn emote_range_clear(data: *mut c_void) {
    unsafe {
        let range = data as *mut EmoteRange;

        g_free((*range).id as *mut c_void);
    }
}

unsafe extern "C" fn compare_emote_ranges(a: *const c_void, b: *const c_void) -> c_int {
    unsafe {
        let range_a = a as *const EmoteRange;
        let range_b = b as *const EmoteRange;

        if (*range_a).start == (*range_b).start {
            return 0;
        }

        if (*range_a).start < (*range_b).start {
            -1
        } else {
            1
        }
    }
}

fn parse_uint(bytes: &[u8]) -> Option<c_uint> {
    if !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

unsafe fn parse_emote_ranges(emotes: *const c_char) -> *mut GArray {
    unsafe {
        if emotes.is_null() || *emotes == 0 {
            return ptr::null_mut();
        }

        let ranges = g_array_new(0, 0, mem::size_of::<EmoteRange>() as c_uint);
        g_array_set_clear_func(ranges, Some(emote_range_clear));

        let emote_bytes = CStr::from_ptr(emotes).to_bytes();
        for emote_spec in emote_bytes.split(|byte| *byte == b'/') {
            let Some(colon) = emote_spec.iter().position(|byte| *byte == b':') else {
                continue;
            };
            if colon == 0 || colon + 1 == emote_spec.len() {
                continue;
            }

            let id = &emote_spec[..colon];
            for position in emote_spec[colon + 1..].split(|byte| *byte == b',') {
                let Some(dash) = position.iter().position(|byte| *byte == b'-') else {
                    continue;
                };
                if dash == 0 || dash + 1 == position.len() {
                    continue;
                }

                let Some(start) = parse_uint(&position[..dash]) else {
                    continue;
                };
                let Some(end) = parse_uint(&position[dash + 1..]) else {
                    continue;
                };
                if end < start || end == c_uint::MAX {
                    continue;
                }

                let range = EmoteRange {
                    start,
                    end,
                    id: dup_bytes(id),
                };
                g_array_append_vals(ranges, &range as *const EmoteRange as *const c_void, 1);
            }
        }

        if (*ranges).len == 0 {
            g_array_unref(ranges);
            return ptr::null_mut();
        }

        g_array_sort(ranges, Some(compare_emote_ranges));
        ranges
    }
}

fn utf8_byte_offset(text: &[u8], offset: usize) -> Option<usize> {
    let text = std::str::from_utf8(text).ok()?;
    text.char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .nth(offset)
}

unsafe fn utf8_offset_to_pointer_safe(text: *const c_char, offset: c_uint) -> *const c_char {
    unsafe {
        if text.is_null() {
            return ptr::null();
        }
        let bytes = CStr::from_ptr(text).to_bytes();
        utf8_byte_offset(bytes, offset as usize).map_or(ptr::null(), |index| text.add(index))
    }
}

pub fn chat_assets_new() -> *mut ChatAssets {
    unsafe {
        Box::into_raw(Box::new(ChatAssets {
            image_cache: g_hash_table_new_full(
                Some(g_str_hash),
                Some(g_str_equal),
                Some(g_free),
                Some(g_object_unref),
            ),
            ref_count: AtomicUsize::new(1),
            closing: AtomicBool::new(false),
        }))
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `assets`: null or owned instance; release once; no subsequent pointer access.
pub unsafe fn chat_assets_free(assets: *mut ChatAssets) {
    unsafe {
        if assets.is_null() {
            return;
        }

        (*assets).closing.store(true, Ordering::Release);
        chat_assets_unref(assets);
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `assets`: live/open; `buffer` and `view`: matching GTK buffer/view.
/// - `iter`: writable, valid iterator for `buffer`; generic types match GTK types.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn chat_assets_insert_message_text<B, V, I>(
    assets: *mut ChatAssets,
    buffer: *mut B,
    view: *mut V,
    iter: *mut I,
    message: *const c_char,
    emotes: *const c_char,
) {
    unsafe {
        let buffer = buffer as *mut GtkTextBuffer;
        let view = view as *mut GtkTextView;
        let iter = iter as *mut GtkTextIter;
        let ranges = parse_emote_ranges(emotes);

        if message.is_null() {
            if !ranges.is_null() {
                g_array_unref(ranges);
            }
            return;
        }

        if ranges.is_null() {
            gtk_text_buffer_insert(buffer, iter, message, -1);
            return;
        }

        let mut cursor = message;
        for i in 0..(*ranges).len {
            let range = ((*ranges).data as *mut EmoteRange).add(i as usize);
            let start = utf8_offset_to_pointer_safe(message, (*range).start);
            let end = utf8_offset_to_pointer_safe(message, (*range).end + 1);

            if start.is_null()
                || end.is_null()
                || (start as usize) < (cursor as usize)
                || (end as usize) < (start as usize)
            {
                continue;
            }

            if (start as usize) > (cursor as usize) {
                gtk_text_buffer_insert(buffer, iter, cursor, start.offset_from(cursor) as c_int);
            }

            let id = CStr::from_ptr((*range).id).to_bytes();
            let mut url = Vec::with_capacity(id.len() + 57);
            url.extend_from_slice(b"https://static-cdn.jtvnw.net/emoticons/v2/");
            url.extend_from_slice(id);
            url.extend_from_slice(b"/default/dark/1.0\0");
            insert_emote(assets, buffer, view, iter, url.as_ptr() as *const c_char);
            cursor = end;
        }

        if *cursor != 0 {
            gtk_text_buffer_insert(buffer, iter, cursor, -1);
        }

        g_array_unref(ranges);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_offsets_respect_character_boundaries() {
        let text = "aä😀".as_bytes();
        assert_eq!(utf8_byte_offset(text, 0), Some(0));
        assert_eq!(utf8_byte_offset(text, 1), Some(1));
        assert_eq!(utf8_byte_offset(text, 2), Some(3));
        assert_eq!(utf8_byte_offset(text, 3), Some(7));
        assert_eq!(utf8_byte_offset(text, 4), None);
        assert_eq!(utf8_byte_offset(b"", 0), Some(0));
        assert_eq!(utf8_byte_offset(b"", 1), None);
        assert_eq!(utf8_byte_offset(b"\xc3", 1), None);
        assert_eq!(utf8_byte_offset(b"\xff", 0), None);
    }

    #[test]
    fn emote_offsets_reject_signs_whitespace_and_overflow() {
        assert_eq!(parse_uint(b"0"), Some(0));
        assert_eq!(parse_uint(b"4294967295"), Some(4294967295));
        for invalid in [
            b"".as_slice(),
            b"-1",
            b"+1",
            b" 1",
            b"1 ",
            b"4294967296",
            b"1x",
        ] {
            assert_eq!(parse_uint(invalid), None, "{invalid:?}");
        }
    }

    #[test]
    fn owner_release_waits_for_pending_image_reference() {
        unsafe {
            let assets = chat_assets_new();
            let pending = chat_assets_ref(assets);

            chat_assets_free(assets);

            assert!((*pending).closing.load(Ordering::Acquire));
            assert_eq!((*pending).ref_count.load(Ordering::Acquire), 1);
            assert!(!(*pending).image_cache.is_null());

            chat_assets_unref(pending);
        }
    }
}

#[cfg(test)]
mod behavior_tests {
    use super as chat_assets;
    use std::ffi::{CStr, CString, c_char};
    use std::ptr;
    use std::slice;

    use super::g_array_unref;

    fn cstring(value: &str) -> CString {
        CString::new(value).unwrap()
    }

    unsafe fn cstr_eq(actual: *const c_char, expected: &str) {
        unsafe {
            assert!(!actual.is_null());
            assert_eq!(CStr::from_ptr(actual).to_str().unwrap(), expected);
        }
    }

    unsafe fn parse_emote_ranges(value: *const c_char) -> *mut chat_assets::GArray {
        unsafe { chat_assets::parse_emote_ranges(value) }
    }

    unsafe fn ranges_slice<'a>(ranges: *mut chat_assets::GArray) -> &'a [chat_assets::EmoteRange] {
        unsafe {
            slice::from_raw_parts(
                (*ranges).data as *const chat_assets::EmoteRange,
                (*ranges).len as usize,
            )
        }
    }

    #[test]
    fn test_parse_emote_ranges_returns_sorted_ranges() {
        unsafe {
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
    }

    #[test]
    fn test_parse_emote_ranges_ignores_invalid_specs() {
        unsafe {
            let input = cstring("bad/25:4-2,abc-5,1-x,0-4294967295,0-4294967296/33:2-3");
            let ranges = parse_emote_ranges(input.as_ptr());

            assert!(!ranges.is_null());
            assert_eq!((*ranges).len, 1);

            let ranges_ref = ranges_slice(ranges);
            assert_eq!(ranges_ref[0].start, 2);
            assert_eq!(ranges_ref[0].end, 3);
            cstr_eq(ranges_ref[0].id, "33");

            g_array_unref(ranges);
        }
    }

    #[test]
    fn test_parse_emote_ranges_returns_null_for_empty_input() {
        unsafe {
            assert!(parse_emote_ranges(ptr::null()).is_null());
            assert!(parse_emote_ranges(cstring("").as_ptr()).is_null());
            assert!(parse_emote_ranges(cstring("bad/also-bad").as_ptr()).is_null());
        }
    }

    #[test]
    fn test_utf8_offset_to_pointer_safe() {
        unsafe {
            let text = b"a\xc3\xa4b\0";
            let ptr = text.as_ptr() as *const c_char;

            assert_eq!(chat_assets::utf8_offset_to_pointer_safe(ptr, 0), ptr);
            cstr_eq(chat_assets::utf8_offset_to_pointer_safe(ptr, 1), "äb");
            cstr_eq(chat_assets::utf8_offset_to_pointer_safe(ptr, 2), "b");
            cstr_eq(chat_assets::utf8_offset_to_pointer_safe(ptr, 3), "");
            assert!(chat_assets::utf8_offset_to_pointer_safe(ptr, 4).is_null());
        }
    }
}
