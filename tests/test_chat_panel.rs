use std::ffi::{c_char, c_double, c_void, CStr, CString};
use std::ptr;
use twitch_player_core::chat_panel;

unsafe extern "C" {
    fn g_object_unref(object: *mut c_void);
    fn gtk_adjustment_new(
        value: c_double,
        lower: c_double,
        upper: c_double,
        step_increment: c_double,
        page_increment: c_double,
        page_size: c_double,
    ) -> *mut chat_panel::GtkAdjustment;
    fn gtk_adjustment_set_value(adjustment: *mut chat_panel::GtkAdjustment, value: c_double);
}

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn color_for(name: *const c_char) -> &'static str {
    CStr::from_ptr(chat_panel::chat_panel_test_fallback_username_color(name))
        .to_str()
        .unwrap()
}

unsafe fn test_fallback_username_color_is_deterministic() {
    let alice = cstring("alice");
    let first = color_for(alice.as_ptr());
    let second = color_for(alice.as_ptr());
    let empty = color_for(ptr::null());

    assert_eq!(first, second);
    assert!(first.starts_with('#'));
    assert_eq!(first.len(), 7);
    assert!(empty.starts_with('#'));
    assert_eq!(empty.len(), 7);
}

unsafe fn test_adjustment_is_at_bottom() {
    let adjustment = gtk_adjustment_new(90.0, 0.0, 100.0, 1.0, 10.0, 10.0);

    assert_ne!(
        chat_panel::chat_panel_test_adjustment_is_at_bottom(ptr::null_mut()),
        0
    );
    assert_ne!(
        chat_panel::chat_panel_test_adjustment_is_at_bottom(adjustment),
        0
    );

    gtk_adjustment_set_value(adjustment, 87.0);
    assert_eq!(
        chat_panel::chat_panel_test_adjustment_is_at_bottom(adjustment),
        0
    );

    gtk_adjustment_set_value(adjustment, 88.0);
    assert_ne!(
        chat_panel::chat_panel_test_adjustment_is_at_bottom(adjustment),
        0
    );

    g_object_unref(adjustment as *mut c_void);
}

fn main() {
    unsafe {
        test_fallback_username_color_is_deterministic();
        test_adjustment_is_at_bottom();
    }
}
