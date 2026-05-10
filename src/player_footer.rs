use std::ffi::{c_char, c_float, c_int, c_void, CStr};
use std::ptr;

const GTK_ORIENTATION_VERTICAL: c_int = 1;
const GTK_ALIGN_FILL: c_int = 0;
const GTK_ALIGN_START: c_int = 1;
const GTK_ALIGN_END: c_int = 2;
const GTK_ALIGN_CENTER: c_int = 3;
const PANGO_ELLIPSIZE_END: c_int = 3;

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkBox {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkLabel {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GObject {
    _private: [u8; 0],
}

pub struct PlayerFooterStreamInfo {
    widget: *mut GtkWidget,
    title_label: *mut GtkWidget,
    metadata_label: *mut GtkWidget,
}

unsafe extern "C" {
    fn g_object_add_weak_pointer(object: *mut GObject, weak_pointer_location: *mut *mut c_void);
    fn g_object_remove_weak_pointer(object: *mut GObject, weak_pointer_location: *mut *mut c_void);
    fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    fn gtk_label_new(str: *const c_char) -> *mut GtkWidget;
    fn gtk_label_set_ellipsize(label: *mut GtkLabel, mode: c_int);
    fn gtk_label_set_single_line_mode(label: *mut GtkLabel, single_line_mode: c_int);
    fn gtk_label_set_text(label: *mut GtkLabel, str: *const c_char);
    fn gtk_label_set_xalign(label: *mut GtkLabel, xalign: c_float);
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_tooltip_text(widget: *mut GtkWidget, text: *const c_char);
    fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
}

unsafe fn add_weak_pointer(widget: *mut GtkWidget, field: *mut *mut GtkWidget) {
    g_object_add_weak_pointer(widget as *mut GObject, field as *mut *mut c_void);
}

unsafe fn remove_weak_pointer(widget: *mut GtkWidget, field: *mut *mut GtkWidget) {
    g_object_remove_weak_pointer(widget as *mut GObject, field as *mut *mut c_void);
}

unsafe fn create_stream_info_label(css_class: *const c_char, valign: c_int) -> *mut GtkWidget {
    let label = gtk_label_new(b"\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(label, css_class);
    gtk_widget_set_halign(label, GTK_ALIGN_FILL);
    gtk_widget_set_valign(label, valign);
    gtk_widget_set_hexpand(label, 1);
    gtk_label_set_xalign(label as *mut GtkLabel, 0.0);
    gtk_label_set_ellipsize(label as *mut GtkLabel, PANGO_ELLIPSIZE_END);
    gtk_label_set_single_line_mode(label as *mut GtkLabel, 1);
    label
}

unsafe fn is_nonempty(text: *const c_char) -> bool {
    !text.is_null() && *text != 0
}

pub unsafe fn player_footer_stream_info_new() -> *mut PlayerFooterStreamInfo {
    let mut info = Box::new(PlayerFooterStreamInfo {
        widget: ptr::null_mut(),
        title_label: ptr::null_mut(),
        metadata_label: ptr::null_mut(),
    });

    info.widget = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class(
        info.widget,
        b"stream-info-labels\0".as_ptr() as *const c_char,
    );
    gtk_widget_set_halign(info.widget, GTK_ALIGN_FILL);
    gtk_widget_set_valign(info.widget, GTK_ALIGN_CENTER);
    gtk_widget_set_hexpand(info.widget, 1);
    add_weak_pointer(info.widget, &mut info.widget);

    info.title_label = create_stream_info_label(
        b"stream-title-label\0".as_ptr() as *const c_char,
        GTK_ALIGN_END,
    );
    info.metadata_label = create_stream_info_label(
        b"stream-metadata-label\0".as_ptr() as *const c_char,
        GTK_ALIGN_START,
    );
    add_weak_pointer(info.title_label, &mut info.title_label);
    add_weak_pointer(info.metadata_label, &mut info.metadata_label);

    gtk_box_append(info.widget as *mut GtkBox, info.title_label);
    gtk_box_append(info.widget as *mut GtkBox, info.metadata_label);

    Box::into_raw(info)
}

pub unsafe fn player_footer_stream_info_get_widget<W>(info: *mut PlayerFooterStreamInfo) -> *mut W {
    if info.is_null() {
        return ptr::null_mut();
    }

    (*info).widget as *mut W
}

pub unsafe fn player_footer_stream_info_set(
    info: *mut PlayerFooterStreamInfo,
    title: *const c_char,
    metadata: *const c_char,
) {
    if info.is_null() || (*info).title_label.is_null() {
        return;
    }

    let empty = b"\0".as_ptr() as *const c_char;
    gtk_label_set_text(
        (*info).title_label as *mut GtkLabel,
        if title.is_null() { empty } else { title },
    );
    if !(*info).metadata_label.is_null() {
        gtk_label_set_text(
            (*info).metadata_label as *mut GtkLabel,
            if metadata.is_null() { empty } else { metadata },
        );
    }

    let mut tooltip_storage = Vec::new();
    let tooltip = if is_nonempty(title) && is_nonempty(metadata) {
        tooltip_storage.extend_from_slice(CStr::from_ptr(title).to_bytes());
        tooltip_storage.extend_from_slice(b" \xE2\x80\xA2 ");
        tooltip_storage.extend_from_slice(CStr::from_ptr(metadata).to_bytes());
        tooltip_storage.push(0);
        tooltip_storage.as_ptr() as *const c_char
    } else if is_nonempty(title) {
        title
    } else if is_nonempty(metadata) {
        metadata
    } else {
        ptr::null()
    };

    gtk_widget_set_tooltip_text((*info).title_label, tooltip);
    if !(*info).metadata_label.is_null() {
        gtk_widget_set_tooltip_text((*info).metadata_label, tooltip);
    }
}

pub unsafe fn player_footer_stream_info_clear(info: *mut PlayerFooterStreamInfo) {
    player_footer_stream_info_set(
        info,
        b"\0".as_ptr() as *const c_char,
        b"\0".as_ptr() as *const c_char,
    );
}

pub unsafe fn player_footer_stream_info_free(info: *mut PlayerFooterStreamInfo) {
    if info.is_null() {
        return;
    }

    if !(*info).widget.is_null() {
        remove_weak_pointer((*info).widget, &mut (*info).widget);
    }
    if !(*info).title_label.is_null() {
        remove_weak_pointer((*info).title_label, &mut (*info).title_label);
    }
    if !(*info).metadata_label.is_null() {
        remove_weak_pointer((*info).metadata_label, &mut (*info).metadata_label);
    }

    drop(Box::from_raw(info));
}
