use std::ffi::{c_char, c_int};

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkButton {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn gtk_button_new() -> *mut GtkWidget;
    fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    fn gtk_button_set_has_frame(button: *mut GtkButton, has_frame: c_int);
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    fn gtk_widget_set_tooltip_text(widget: *mut GtkWidget, text: *const c_char);
}

pub unsafe fn player_overlay_button_new<W>(icon: *mut W, tooltip: *const c_char) -> *mut W {
    let button = gtk_button_new();

    gtk_button_set_child(button as *mut GtkButton, icon as *mut GtkWidget);
    gtk_button_set_has_frame(button as *mut GtkButton, 0);
    gtk_widget_add_css_class(button, b"overlay-icon-button\0".as_ptr() as *const c_char);
    gtk_widget_set_tooltip_text(button, tooltip);

    button as *mut W
}
