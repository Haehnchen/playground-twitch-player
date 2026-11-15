use crate::player::icons::player_volume_icon_new;
use crate::player::session::{
    player_session_get_muted, player_session_set_muted, player_session_set_volume,
    player_session_toggle_muted,
};
use gtk4::glib::translate::{Borrowed, from_glib_borrow, from_glib_none};
use gtk4::prelude::*;
use std::ffi::{CStr, c_char, c_int};
use std::ptr;

const PLAYER_VOLUME_MIN: f64 = 0.0;
const PLAYER_VOLUME_MAX: f64 = 130.0;
const PLAYER_VOLUME_SCROLL_STEP: f64 = 5.0;

pub struct PlayerFooterStreamInfo {
    widget: gtk4::Box,
    title_label: gtk4::Label,
    metadata_label: gtk4::Label,
}

// Borrow widgets owned by the existing C-based player without changing their references.
unsafe fn borrowed_widget<W>(widget: *mut W) -> Option<Borrowed<gtk4::Widget>> {
    unsafe { (!widget.is_null()).then(|| from_glib_borrow(widget.cast())) }
}

unsafe fn text<'a>(value: *const c_char) -> std::borrow::Cow<'a, str> {
    unsafe {
        if value.is_null() {
            "".into()
        } else {
            CStr::from_ptr(value).to_string_lossy()
        }
    }
}

fn stream_info_label(css_class: &str, valign: gtk4::Align) -> gtk4::Label {
    gtk4::Label::builder()
        .css_classes([css_class])
        .halign(gtk4::Align::Fill)
        .valign(valign)
        .hexpand(true)
        .xalign(0.0)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .single_line_mode(true)
        .build()
}

/// # Safety
/// - GTK initialized; GTK main thread.
pub unsafe fn player_footer_stream_info_new() -> *mut PlayerFooterStreamInfo {
    let widget = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["stream-info-labels"])
        .halign(gtk4::Align::Fill)
        .valign(gtk4::Align::Center)
        .hexpand(true)
        .build();
    let title_label = stream_info_label("stream-title-label", gtk4::Align::End);
    let metadata_label = stream_info_label("stream-metadata-label", gtk4::Align::Start);
    widget.append(&title_label);
    widget.append(&metadata_label);
    Box::into_raw(Box::new(PlayerFooterStreamInfo {
        widget,
        title_label,
        metadata_label,
    }))
}

/// # Safety
/// - GTK main thread; `info`: null or live instance.
/// - Result: borrowed GTK widget, valid until the owner is freed; compatible `W`.
pub unsafe fn player_footer_stream_info_get_widget<W>(info: *mut PlayerFooterStreamInfo) -> *mut W {
    unsafe {
        info.as_ref()
            .map_or(ptr::null_mut(), |info| info.widget.as_ptr().cast())
    }
}

/// Update the title, metadata and tooltip for the active stream.
///
/// # Safety
/// - GTK main thread; `info`: null or live instance.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn player_footer_stream_info_set(
    info: *mut PlayerFooterStreamInfo,
    title: *const c_char,
    metadata: *const c_char,
) {
    unsafe {
        let Some(info) = info.as_ref() else { return };
        let title = text(title);
        let metadata = text(metadata);
        info.title_label.set_text(&title);
        info.metadata_label.set_text(&metadata);
        let tooltip = [title.as_ref(), metadata.as_ref()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" • ");
        let tooltip = (!tooltip.is_empty()).then_some(tooltip.as_str());
        info.title_label.set_tooltip_text(tooltip);
        info.metadata_label.set_tooltip_text(tooltip);
    }
}

/// # Safety
/// - GTK main thread; `info`: null or live instance.
pub unsafe fn player_footer_stream_info_clear(info: *mut PlayerFooterStreamInfo) {
    unsafe { player_footer_stream_info_set(info, ptr::null(), ptr::null()) }
}

/// # Safety
/// - GTK main thread; release a non-null owner exactly once.
pub unsafe fn player_footer_stream_info_free(info: *mut PlayerFooterStreamInfo) {
    if !info.is_null() {
        unsafe { drop(Box::from_raw(info)) }
    }
}

/// Create an icon button with the shared overlay style and tooltip.
///
/// # Safety
/// - GTK initialized; GTK main thread.
/// - `icon`: null or unparented GTK widget, compatible `W`; ownership transfers.
/// - Non-null tooltip: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn player_overlay_button_new<W>(icon: *mut W, tooltip: *const c_char) -> *mut W {
    unsafe {
        // C callers expect a floating reference that the parent will take over.
        let raw = gtk4::ffi::gtk_button_new();
        let button: Borrowed<gtk4::Button> = from_glib_borrow(raw.cast());
        let icon: Option<gtk4::Widget> = (!icon.is_null()).then(|| from_glib_none(icon.cast()));
        button.set_child(icon.as_ref());
        button.set_has_frame(false);
        button.add_css_class("overlay-icon-button");
        button.set_tooltip_text((!tooltip.is_null()).then(|| text(tooltip)).as_deref());
        raw.cast()
    }
}

/// # Safety
/// - GTK main thread; session: null or live instance; range: null or live GtkRange.
/// - Generic types must match the GTK/session types.
pub unsafe fn player_volume_sync_session_from_range<S, R>(session: *mut S, range: *mut R) {
    unsafe {
        if let Some(widget) = borrowed_widget(range)
            && let Some(range) = widget.downcast_ref::<gtk4::Range>()
        {
            player_session_set_volume(session.cast(), range.value());
        }
    }
}

/// Apply vertical scrolling to volume within the allowed range.
///
/// # Safety
/// - GTK main thread; `volume_scale`: null or live GTK widget, compatible `W`.
pub unsafe fn player_volume_apply_scroll<W>(volume_scale: *mut W, dx: f64, dy: f64) -> c_int {
    unsafe {
        if dy.abs() < dx.abs() || dy == 0.0 {
            return 0;
        }
        let Some(widget) = borrowed_widget(volume_scale) else {
            return 0;
        };
        let Some(range) = widget.downcast_ref::<gtk4::Range>() else {
            return 0;
        };
        range.set_value(
            (range.value() - dy * PLAYER_VOLUME_SCROLL_STEP)
                .clamp(PLAYER_VOLUME_MIN, PLAYER_VOLUME_MAX),
        );
        1
    }
}

/// # Safety
/// - GTK main thread; session: null or live PlayerSession; `W` must be a GTK widget.
pub unsafe fn player_volume_mute_button_new<S, W>(session: *mut S) -> *mut W {
    unsafe {
        let kind = (player_session_get_muted(session.cast()) != 0) as c_int;
        let raw: *mut gtk4::ffi::GtkWidget =
            player_overlay_button_new(player_volume_icon_new(kind), ptr::null());
        borrowed_widget(raw)
            .unwrap()
            .add_css_class("volume-mute-button");
        raw.cast()
    }
}

/// # Safety
/// - GTK main thread; session: null or live PlayerSession; button: null or live GTK widget.
/// - Generic types must match the GTK/session types.
pub unsafe fn player_volume_update_mute_button<W, S>(mute_button: *mut W, session: *mut S) {
    unsafe {
        let Some(widget) = borrowed_widget(mute_button) else {
            return;
        };
        let Some(button) = widget.downcast_ref::<gtk4::Button>() else {
            return;
        };
        let kind = (player_session_get_muted(session.cast()) != 0) as c_int;
        let raw: *mut gtk4::ffi::GtkWidget = player_volume_icon_new(kind);
        let icon: gtk4::Widget = from_glib_none(raw);
        button.set_child(Some(&icon));
    }
}

/// # Safety
/// - GTK main thread; session: null or live PlayerSession; button: null or live GTK widget.
/// - Generic types must match the GTK/session types.
pub unsafe fn player_volume_set_muted<S, W>(session: *mut S, mute_button: *mut W, muted: c_int) {
    unsafe {
        player_session_set_muted(session.cast(), muted);
        player_volume_update_mute_button(mute_button, session);
    }
}

/// # Safety
/// - GTK main thread; session: null or live PlayerSession; button: null or live GTK widget.
/// - Generic types must match the GTK/session types.
pub unsafe fn player_volume_toggle_muted<S, W>(session: *mut S, mute_button: *mut W) {
    unsafe {
        player_session_toggle_muted(session.cast());
        player_volume_update_mute_button(mute_button, session);
    }
}
