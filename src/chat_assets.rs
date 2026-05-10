use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void, CStr};
use std::mem;
use std::ptr;

const CHAT_EMOTE_SIZE: c_int = 14;
const GTK_CONTENT_FIT_CONTAIN: c_int = 1;

pub struct ChatAssets {
    image_cache: *mut GHashTable,
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
    static g_utf8_skip: *const c_char;

    fn g_array_append_vals(array: *mut GArray, data: *const c_void, len: c_uint) -> *mut GArray;
    fn g_array_new(zero_terminated: c_int, clear_: c_int, element_size: c_uint) -> *mut GArray;
    fn g_array_set_clear_func(array: *mut GArray, clear_func: Option<GDestroyNotify>);
    fn g_array_sort(array: *mut GArray, compare_func: Option<GCompareFunc>);
    fn g_array_unref(array: *mut GArray);
    fn g_ascii_strtoull(nptr: *const c_char, endptr: *mut *mut c_char, base: c_uint)
        -> c_ulonglong;
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

unsafe fn dup_bytes(bytes: &[u8]) -> *mut c_char {
    let mut value = Vec::with_capacity(bytes.len() + 1);
    value.extend_from_slice(bytes);
    value.push(0);
    g_strdup(value.as_ptr() as *const c_char)
}

unsafe fn image_load_data_free(data: *mut ImageLoadData) {
    if data.is_null() {
        return;
    }

    if !(*data).picture.is_null() {
        g_object_unref((*data).picture as *mut c_void);
    }
    g_free((*data).url as *mut c_void);
    drop(Box::from_raw(data));
}

unsafe extern "C" fn on_image_loaded(
    source: *mut GObject,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
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

    g_hash_table_insert(
        (*(*data).assets).image_cache,
        g_strdup((*data).url) as *mut c_void,
        g_object_ref(texture as *mut c_void),
    );
    gtk_picture_set_paintable((*data).picture, texture as *mut GdkPaintable);
    g_object_unref(texture as *mut c_void);
    image_load_data_free(data);
}

unsafe fn create_inline_image(assets: *mut ChatAssets, url: *const c_char) -> *mut GtkWidget {
    let picture = gtk_picture_new();
    gtk_widget_add_css_class(picture, b"chat-emote\0".as_ptr() as *const c_char);
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
        assets,
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

unsafe fn insert_emote(
    assets: *mut ChatAssets,
    buffer: *mut GtkTextBuffer,
    view: *mut GtkTextView,
    iter: *mut GtkTextIter,
    url: *const c_char,
) {
    let anchor = gtk_text_child_anchor_new();

    gtk_text_buffer_insert_child_anchor(buffer, iter, anchor);
    gtk_text_view_add_child_at_anchor(view, create_inline_image(assets, url), anchor);
    g_object_unref(anchor as *mut c_void);
}

unsafe extern "C" fn emote_range_clear(data: *mut c_void) {
    let range = data as *mut EmoteRange;

    g_free((*range).id as *mut c_void);
}

unsafe extern "C" fn compare_emote_ranges(a: *const c_void, b: *const c_void) -> c_int {
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

unsafe fn parse_uint(bytes: &[u8]) -> Option<c_ulonglong> {
    let mut value = Vec::with_capacity(bytes.len() + 1);
    value.extend_from_slice(bytes);
    value.push(0);

    let mut end_ptr: *mut c_char = ptr::null_mut();
    let parsed = g_ascii_strtoull(value.as_ptr() as *const c_char, &mut end_ptr, 10);
    if end_ptr == value.as_mut_ptr() as *mut c_char || *end_ptr != 0 {
        return None;
    }

    Some(parsed)
}

unsafe fn parse_emote_ranges(emotes: *const c_char) -> *mut GArray {
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
            if end < start || end > c_uint::MAX as c_ulonglong {
                continue;
            }

            let range = EmoteRange {
                start: start as c_uint,
                end: end as c_uint,
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

unsafe fn utf8_offset_to_pointer_safe(text: *const c_char, offset: c_uint) -> *const c_char {
    let mut p = text;

    for _ in 0..offset {
        if *p == 0 {
            return ptr::null();
        }

        let skip = *g_utf8_skip.add(*(p as *const u8) as usize) as isize;
        p = p.offset(skip);
    }

    p
}

pub unsafe fn chat_assets_new() -> *mut ChatAssets {
    Box::into_raw(Box::new(ChatAssets {
        image_cache: g_hash_table_new_full(
            Some(g_str_hash),
            Some(g_str_equal),
            Some(g_free),
            Some(g_object_unref),
        ),
    }))
}

pub unsafe fn chat_assets_free(assets: *mut ChatAssets) {
    if assets.is_null() {
        return;
    }

    if !(*assets).image_cache.is_null() {
        g_hash_table_destroy((*assets).image_cache);
    }
    drop(Box::from_raw(assets));
}

pub unsafe fn chat_assets_insert_message_text<B, V, I>(
    assets: *mut ChatAssets,
    buffer: *mut B,
    view: *mut V,
    iter: *mut I,
    message: *const c_char,
    emotes: *const c_char,
) {
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

pub unsafe fn chat_assets_test_parse_emote_ranges(emotes: *const c_char) -> *mut GArray {
    parse_emote_ranges(emotes)
}

pub unsafe fn chat_assets_test_utf8_offset_to_pointer_safe(
    text: *const c_char,
    offset: c_uint,
) -> *const c_char {
    utf8_offset_to_pointer_safe(text, offset)
}
