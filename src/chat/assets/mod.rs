use gtk4::ffi::{
    GtkTextBuffer, GtkTextIter, GtkTextView, gtk_text_buffer_insert,
    gtk_text_buffer_insert_child_anchor, gtk_text_view_add_child_at_anchor,
};
use gtk4::{gdk, gio, glib, prelude::*};
use std::{
    collections::HashMap,
    ffi::{CStr, c_char, c_int, c_uint},
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub struct ChatAssets {
    image_cache: HashMap<String, gdk::Texture>,
    ref_count: AtomicUsize,
    closing: AtomicBool,
}

#[derive(Debug, PartialEq, Eq)]
struct EmoteRange {
    start: c_uint,
    end: c_uint,
    id: String,
}

struct AssetsRef(*mut ChatAssets);

impl AssetsRef {
    fn get(&self) -> *mut ChatAssets {
        self.0
    }
}

impl Drop for AssetsRef {
    fn drop(&mut self) {
        unsafe {
            chat_assets_unref(self.0);
        }
    }
}

unsafe fn chat_assets_ref(assets: *mut ChatAssets) -> *mut ChatAssets {
    unsafe {
        (*assets).ref_count.fetch_add(1, Ordering::Relaxed);
        assets
    }
}

unsafe fn chat_assets_unref(assets: *mut ChatAssets) {
    unsafe {
        if !assets.is_null() && (*assets).ref_count.fetch_sub(1, Ordering::AcqRel) == 1 {
            drop(Box::from_raw(assets));
        }
    }
}

pub fn chat_assets_new() -> *mut ChatAssets {
    Box::into_raw(Box::new(ChatAssets {
        image_cache: HashMap::new(),
        ref_count: AtomicUsize::new(1),
        closing: AtomicBool::new(false),
    }))
}

/// # Safety
/// - GTK main thread; release an owned instance once, with no subsequent access.
pub unsafe fn chat_assets_free(assets: *mut ChatAssets) {
    unsafe {
        if !assets.is_null() {
            (*assets).closing.store(true, Ordering::Release);
            chat_assets_unref(assets);
        }
    }
}

// Cache emote textures so later messages can reuse them.
unsafe fn create_inline_image(assets: *mut ChatAssets, url: &str) -> gtk4::Picture {
    unsafe {
        let picture = gtk4::Picture::builder()
            .can_shrink(true)
            .focusable(false)
            .width_request(14)
            .height_request(14)
            .content_fit(gtk4::ContentFit::Contain)
            .build();
        picture.add_css_class("chat-emote");
        if let Some(texture) = (*assets).image_cache.get(url) {
            picture.set_paintable(Some(texture));
            return picture;
        }
        let owner = AssetsRef(chat_assets_ref(assets));
        let url = url.to_owned();
        let pending_picture = picture.clone();
        glib::MainContext::default().spawn_local(async move {
            let Ok((contents, _)) = gio::File::for_uri(&url).load_contents_future().await else {
                return;
            };
            if (*owner.get()).closing.load(Ordering::Acquire) {
                return;
            }
            if let Ok(texture) = gdk::Texture::from_bytes(&glib::Bytes::from_owned(contents)) {
                (*owner.get()).image_cache.insert(url, texture.clone());
                pending_picture.set_paintable(Some(&texture));
            }
        });
        picture
    }
}

/// # Safety
/// - GTK main thread; assets live/open; buffer and view belong together.
/// - `iter` points to a writable GTK iterator for buffer; generic types match GTK types.
/// - Non-null strings are readable, NUL-terminated UTF-8 for this call.
pub unsafe fn chat_assets_insert_message_text<B, V, I>(
    assets: *mut ChatAssets,
    buffer: *mut B,
    view: *mut V,
    iter: *mut I,
    message: *const c_char,
    emotes: *const c_char,
) {
    unsafe {
        if message.is_null() {
            return;
        }
        let buffer = buffer.cast::<GtkTextBuffer>();
        let view = view.cast::<GtkTextView>();
        let iter = iter.cast::<GtkTextIter>();
        let emotes = if emotes.is_null() {
            &[]
        } else {
            CStr::from_ptr(emotes).to_bytes()
        };
        let ranges = parse_emote_ranges(emotes);
        let mut cursor = message;
        for range in ranges {
            let start = utf8_offset_to_pointer_safe(message, range.start);
            let end = utf8_offset_to_pointer_safe(message, range.end + 1);
            if start.is_null()
                || end.is_null()
                || (start as usize) < (cursor as usize)
                || (end as usize) < (start as usize)
            {
                continue;
            }
            if start != cursor {
                gtk_text_buffer_insert(buffer, iter, cursor, start.offset_from(cursor) as c_int);
            }
            let anchor = gtk4::TextChildAnchor::new();
            gtk_text_buffer_insert_child_anchor(buffer, iter, anchor.as_ptr());
            let url = format!(
                "https://static-cdn.jtvnw.net/emoticons/v2/{}/default/dark/1.0",
                range.id
            );
            let picture = create_inline_image(assets, &url);
            gtk_text_view_add_child_at_anchor(view, picture.as_ptr().cast(), anchor.as_ptr());
            cursor = end;
        }
        if *cursor != 0 {
            gtk_text_buffer_insert(buffer, iter, cursor, -1);
        }
    }
}

fn parse_uint(bytes: &[u8]) -> Option<c_uint> {
    if !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

// Parse character ranges; invalid and overflowing positions are skipped.
fn parse_emote_ranges(emotes: &[u8]) -> Vec<EmoteRange> {
    let mut ranges = Vec::new();
    for spec in emotes.split(|byte| *byte == b'/') {
        let Some(colon) = spec.iter().position(|byte| *byte == b':') else {
            continue;
        };
        let Ok(id) = std::str::from_utf8(&spec[..colon]) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        for position in spec[colon + 1..].split(|byte| *byte == b',') {
            let Some(dash) = position.iter().position(|byte| *byte == b'-') else {
                continue;
            };
            let (Some(start), Some(end)) = (
                parse_uint(&position[..dash]),
                parse_uint(&position[dash + 1..]),
            ) else {
                continue;
            };
            if end < start || end == c_uint::MAX {
                continue;
            }
            ranges.push(EmoteRange {
                start,
                end,
                id: id.to_owned(),
            });
        }
    }
    ranges.sort_by_key(|range| range.start);
    ranges
}

fn utf8_byte_offset(text: &[u8], offset: usize) -> Option<usize> {
    std::str::from_utf8(text)
        .ok()?
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .nth(offset)
}

unsafe fn utf8_offset_to_pointer_safe(text: *const c_char, offset: c_uint) -> *const c_char {
    unsafe {
        if text.is_null() {
            return ptr::null();
        }
        utf8_byte_offset(CStr::from_ptr(text).to_bytes(), offset as usize)
            .map_or(ptr::null(), |index| text.add(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emote_positions_are_sorted_and_invalid_ranges_are_ignored() {
        let ranges = parse_emote_ranges(
            b"1902:12-16/25:0-4,6-10/bad/33:4-2,abc-5,1-x,0-4294967295,0-4294967296",
        );
        assert_eq!(
            ranges,
            vec![
                EmoteRange {
                    start: 0,
                    end: 4,
                    id: "25".into()
                },
                EmoteRange {
                    start: 6,
                    end: 10,
                    id: "25".into()
                },
                EmoteRange {
                    start: 12,
                    end: 16,
                    id: "1902".into()
                },
            ]
        );
        assert!(parse_emote_ranges(b"").is_empty());
        assert!(parse_emote_ranges(b"bad/also-bad").is_empty());
    }

    #[test]
    fn emote_offsets_reject_signs_whitespace_and_overflow() {
        assert_eq!(parse_uint(b"0"), Some(0));
        assert_eq!(parse_uint(b"4294967295"), Some(u32::MAX));
        for invalid in [
            b"".as_slice(),
            b"-1",
            b"+1",
            b" 1",
            b"1 ",
            b"4294967296",
            b"1x",
        ] {
            assert_eq!(parse_uint(invalid), None);
        }
    }

    #[test]
    fn utf8_offsets_respect_character_boundaries() {
        let text = "aä😀".as_bytes();
        for (offset, expected) in [
            (0, Some(0)),
            (1, Some(1)),
            (2, Some(3)),
            (3, Some(7)),
            (4, None),
        ] {
            assert_eq!(utf8_byte_offset(text, offset), expected);
        }
        assert_eq!(utf8_byte_offset(b"\xc3", 1), None);
        assert_eq!(utf8_byte_offset(b"\xff", 0), None);
        unsafe {
            let text = c"aäb";
            assert_eq!(
                utf8_offset_to_pointer_safe(text.as_ptr(), 1),
                text.as_ptr().add(1)
            );
            assert!(utf8_offset_to_pointer_safe(c"aäb".as_ptr(), 4).is_null());
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
            assert!((*pending).image_cache.is_empty());
            chat_assets_unref(pending);
        }
    }
}
