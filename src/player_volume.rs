use std::ffi::{c_char, c_int};
use std::ptr;

use crate::player_icons::player_volume_icon_new;
use crate::player_overlay_controls::player_overlay_button_new;
use crate::player_session::{
    player_session_get_muted, player_session_set_muted, player_session_set_volume,
    player_session_toggle_muted, PlayerSession,
};

const PLAYER_VOLUME_MIN: f64 = 0.0;
const PLAYER_VOLUME_MAX: f64 = 130.0;
const PLAYER_VOLUME_SCROLL_STEP: f64 = 5.0;
const PLAYER_VOLUME_ICON_SOUND: c_int = 0;
const PLAYER_VOLUME_ICON_MUTED: c_int = 1;

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkButton {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkRange {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GTypeInstance {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn g_type_check_instance_is_a(instance: *mut GTypeInstance, iface_type: usize) -> c_int;
    fn gtk_button_get_type() -> usize;
    fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    fn gtk_range_get_type() -> usize;
    fn gtk_range_get_value(range: *mut GtkRange) -> f64;
    fn gtk_range_set_value(range: *mut GtkRange, value: f64);
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
}

unsafe fn is_a(widget: *mut GtkWidget, type_: usize) -> bool {
    !widget.is_null() && g_type_check_instance_is_a(widget as *mut GTypeInstance, type_) != 0
}

pub unsafe fn player_volume_sync_session_from_range<S, R>(session: *mut S, range: *mut R) {
    let range = range as *mut GtkRange;
    if range.is_null() {
        return;
    }

    player_session_set_volume(session as *mut PlayerSession, gtk_range_get_value(range));
}

pub unsafe fn player_volume_apply_scroll<W>(volume_scale: *mut W, dx: f64, dy: f64) -> c_int {
    let volume_scale = volume_scale as *mut GtkWidget;
    if !is_a(volume_scale, gtk_range_get_type()) || dy.abs() < dx.abs() || dy == 0.0 {
        return 0;
    }

    let range = volume_scale as *mut GtkRange;
    let volume = (gtk_range_get_value(range) - dy * PLAYER_VOLUME_SCROLL_STEP)
        .clamp(PLAYER_VOLUME_MIN, PLAYER_VOLUME_MAX);
    gtk_range_set_value(range, volume);

    1
}

pub unsafe fn player_volume_mute_button_new<S, W>(session: *mut S) -> *mut W {
    let session = session as *mut PlayerSession;
    let icon_kind = if player_session_get_muted(session) != 0 {
        PLAYER_VOLUME_ICON_MUTED
    } else {
        PLAYER_VOLUME_ICON_SOUND
    };
    let button = player_overlay_button_new(player_volume_icon_new(icon_kind), ptr::null());
    gtk_widget_add_css_class(button, b"volume-mute-button\0".as_ptr() as *const c_char);
    button as *mut W
}

pub unsafe fn player_volume_update_mute_button<W, S>(mute_button: *mut W, session: *mut S) {
    let mute_button = mute_button as *mut GtkWidget;
    let session = session as *mut PlayerSession;
    if !is_a(mute_button, gtk_button_get_type()) {
        return;
    }

    let icon_kind = if player_session_get_muted(session) != 0 {
        PLAYER_VOLUME_ICON_MUTED
    } else {
        PLAYER_VOLUME_ICON_SOUND
    };
    gtk_button_set_child(
        mute_button as *mut GtkButton,
        player_volume_icon_new(icon_kind),
    );
}

pub unsafe fn player_volume_set_muted<S, W>(session: *mut S, mute_button: *mut W, muted: c_int) {
    let session = session as *mut PlayerSession;
    player_session_set_muted(session, muted);
    player_volume_update_mute_button(mute_button, session);
}

pub unsafe fn player_volume_toggle_muted<S, W>(session: *mut S, mute_button: *mut W) {
    let session = session as *mut PlayerSession;
    player_session_toggle_muted(session);
    player_volume_update_mute_button(mute_button, session);
}
