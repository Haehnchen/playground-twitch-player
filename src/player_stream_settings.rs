use std::ffi::{c_char, c_int, c_uint, c_void, CStr};
use std::ptr;

use crate::player_icons::player_info_icon_new;

const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
const GTK_ORIENTATION_VERTICAL: c_int = 1;
const GTK_ALIGN_FILL: c_int = 0;
const GTK_ALIGN_START: c_int = 1;
const GTK_ALIGN_CENTER: c_int = 3;
const GTK_POS_TOP: c_int = 2;

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkBox {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkButton {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkLabel {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkPopover {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GObject {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GPtrArray {
    pdata: *mut *mut c_void,
    len: c_uint,
}

#[repr(C)]
pub struct TwitchStreamQuality {
    label: *mut c_char,
    url: *mut c_char,
    width: c_uint,
    height: c_uint,
    bandwidth: c_uint,
    frame_rate: f64,
}

unsafe extern "C" {
    fn g_object_set_data(object: *mut GObject, key: *const c_char, data: *mut c_void);
    fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *mut c_void,
        data: *mut c_void,
        destroy_data: *mut c_void,
        connect_flags: c_int,
    ) -> usize;
    fn g_strcmp0(str1: *const c_char, str2: *const c_char) -> c_int;
    fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    fn gtk_box_remove(box_: *mut GtkBox, child: *mut GtkWidget);
    fn gtk_button_new() -> *mut GtkWidget;
    fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    fn gtk_label_new(str: *const c_char) -> *mut GtkWidget;
    fn gtk_label_set_text(label: *mut GtkLabel, str: *const c_char);
    fn gtk_label_set_xalign(label: *mut GtkLabel, xalign: f32);
    fn gtk_popover_new() -> *mut GtkWidget;
    fn gtk_popover_set_child(popover: *mut GtkPopover, child: *mut GtkWidget);
    fn gtk_popover_set_has_arrow(popover: *mut GtkPopover, has_arrow: c_int);
    fn gtk_popover_set_position(popover: *mut GtkPopover, position: c_int);
    fn gtk_separator_new(orientation: c_int) -> *mut GtkWidget;
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    fn gtk_widget_get_first_child(widget: *mut GtkWidget) -> *mut GtkWidget;
    fn gtk_widget_get_next_sibling(widget: *mut GtkWidget) -> *mut GtkWidget;
    fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_parent(widget: *mut GtkWidget, parent: *mut GtkWidget);
    fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
}

unsafe fn connect_clicked(widget: *mut GtkWidget, callback: *const c_void, user_data: *mut c_void) {
    g_signal_connect_data(
        widget as *mut c_void,
        b"clicked\0".as_ptr() as *const c_char,
        callback as *mut c_void,
        user_data,
        ptr::null_mut(),
        0,
    );
}

unsafe fn player_stream_settings_label_new(
    text: *const c_char,
    css_class: *const c_char,
) -> *mut GtkWidget {
    let label = gtk_label_new(text);
    gtk_label_set_xalign(label as *mut GtkLabel, 0.0);
    gtk_widget_set_halign(label, GTK_ALIGN_START);
    gtk_widget_add_css_class(label, css_class);
    label
}

unsafe fn player_stream_settings_item_button_new(
    label: *const c_char,
    selected: c_int,
) -> *mut GtkWidget {
    let button = gtk_button_new();
    let button_label = gtk_label_new(label);

    gtk_label_set_xalign(button_label as *mut GtkLabel, 0.0);
    gtk_widget_set_halign(button_label, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand(button_label, 1);
    gtk_button_set_child(button as *mut GtkButton, button_label);
    gtk_widget_add_css_class(button, b"stream-settings-item\0".as_ptr() as *const c_char);
    if selected != 0 {
        gtk_widget_add_css_class(
            button,
            b"stream-settings-item-selected\0".as_ptr() as *const c_char,
        );
    }
    gtk_widget_set_halign(button, GTK_ALIGN_FILL);
    button
}

unsafe fn player_stream_settings_info_button_new() -> *mut GtkWidget {
    let button = gtk_button_new();
    let content = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    let label = gtk_label_new(b"Stream Info\0".as_ptr() as *const c_char);

    gtk_widget_add_css_class(button, b"stream-settings-item\0".as_ptr() as *const c_char);
    gtk_widget_set_halign(button, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand(button, 1);
    gtk_widget_set_halign(content, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand(content, 1);
    gtk_label_set_xalign(label as *mut GtkLabel, 0.0);
    gtk_widget_set_hexpand(label, 1);

    gtk_box_append(content as *mut GtkBox, player_info_icon_new());
    gtk_box_append(content as *mut GtkBox, label);
    gtk_button_set_child(button as *mut GtkButton, content);

    button
}

pub unsafe fn player_stream_settings_popover_new<W>(
    relative_to: *mut W,
    quality_list_box_out: *mut *mut W,
    quality_status_label_out: *mut *mut W,
    info_button_out: *mut *mut W,
) -> *mut W {
    let popover = gtk_popover_new();
    gtk_widget_add_css_class(
        popover,
        b"stream-settings-popover\0".as_ptr() as *const c_char,
    );
    gtk_popover_set_position(popover as *mut GtkPopover, GTK_POS_TOP);
    gtk_popover_set_has_arrow(popover as *mut GtkPopover, 0);
    gtk_widget_set_parent(popover, relative_to as *mut GtkWidget);

    let settings_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 6);
    gtk_widget_add_css_class(
        settings_box,
        b"stream-settings-menu\0".as_ptr() as *const c_char,
    );
    gtk_popover_set_child(popover as *mut GtkPopover, settings_box);

    let quality_header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_set_halign(quality_header, GTK_ALIGN_FILL);
    gtk_widget_set_valign(quality_header, GTK_ALIGN_CENTER);
    gtk_box_append(settings_box as *mut GtkBox, quality_header);

    let quality_title = player_stream_settings_label_new(
        b"Quality\0".as_ptr() as *const c_char,
        b"stream-settings-heading\0".as_ptr() as *const c_char,
    );
    gtk_widget_set_valign(quality_title, GTK_ALIGN_CENTER);
    gtk_box_append(quality_header as *mut GtkBox, quality_title);

    let quality_status_label = player_stream_settings_label_new(
        b"\0".as_ptr() as *const c_char,
        b"stream-settings-status\0".as_ptr() as *const c_char,
    );
    gtk_widget_set_valign(quality_status_label, GTK_ALIGN_CENTER);
    gtk_widget_set_hexpand(quality_status_label, 0);
    gtk_box_append(quality_header as *mut GtkBox, quality_status_label);

    let quality_list_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 2);
    gtk_widget_set_halign(quality_list_box, GTK_ALIGN_FILL);
    gtk_box_append(settings_box as *mut GtkBox, quality_list_box);

    let divider = gtk_separator_new(GTK_ORIENTATION_HORIZONTAL);
    gtk_widget_add_css_class(
        divider,
        b"stream-settings-divider\0".as_ptr() as *const c_char,
    );
    gtk_box_append(settings_box as *mut GtkBox, divider);

    let info_button = player_stream_settings_info_button_new();
    gtk_box_append(settings_box as *mut GtkBox, info_button);

    if !quality_list_box_out.is_null() {
        *quality_list_box_out = quality_list_box as *mut W;
    }
    if !quality_status_label_out.is_null() {
        *quality_status_label_out = quality_status_label as *mut W;
    }
    if !info_button_out.is_null() {
        *info_button_out = info_button as *mut W;
    }

    popover as *mut W
}

unsafe fn quality_at(qualities: *mut GPtrArray, index: c_uint) -> *mut TwitchStreamQuality {
    *(*qualities).pdata.add(index as usize) as *mut TwitchStreamQuality
}

unsafe fn current_label(label: *const c_char) -> Vec<u8> {
    let mut value = Vec::new();
    if label.is_null() {
        value.extend_from_slice(b"(null)");
    } else {
        value.extend_from_slice(CStr::from_ptr(label).to_bytes());
    }
    value.extend_from_slice(b" (current)\0");
    value
}

pub unsafe fn player_stream_settings_quality_list_populate<W, A>(
    quality_list_box: *mut W,
    quality_status_label: *mut W,
    qualities: *mut A,
    selected_quality_url: *const c_char,
    selected_quality_label: *const c_char,
    quality_clicked_callback: *const c_void,
    quality_user_data: *mut c_void,
    auto_clicked_callback: *const c_void,
    auto_user_data: *mut c_void,
) {
    let quality_list_box = quality_list_box as *mut GtkWidget;
    let quality_status_label = quality_status_label as *mut GtkWidget;
    let qualities = qualities as *mut GPtrArray;
    if quality_list_box.is_null() || quality_status_label.is_null() {
        return;
    }

    let mut child = gtk_widget_get_first_child(quality_list_box);
    while !child.is_null() {
        let next = gtk_widget_get_next_sibling(child);
        gtk_box_remove(quality_list_box as *mut GtkBox, child);
        child = next;
    }

    if qualities.is_null() || (*qualities).len == 0 {
        gtk_label_set_text(
            quality_status_label as *mut GtkLabel,
            b"No qualities found\0".as_ptr() as *const c_char,
        );
        return;
    }

    gtk_label_set_text(
        quality_status_label as *mut GtkLabel,
        b"\0".as_ptr() as *const c_char,
    );

    for i in 0..(*qualities).len {
        let quality = quality_at(qualities, i);
        let selected = ((!selected_quality_url.is_null()
            && g_strcmp0(selected_quality_url, (*quality).url) == 0)
            || (!selected_quality_label.is_null()
                && g_strcmp0(selected_quality_label, (*quality).label) == 0))
            as c_int;
        let label_storage;
        let label = if selected != 0 {
            label_storage = current_label((*quality).label);
            label_storage.as_ptr() as *const c_char
        } else {
            (*quality).label
        };
        let button = player_stream_settings_item_button_new(label, selected);
        g_object_set_data(
            button as *mut GObject,
            b"stream-quality\0".as_ptr() as *const c_char,
            quality as *mut c_void,
        );
        connect_clicked(button, quality_clicked_callback, quality_user_data);
        gtk_box_append(quality_list_box as *mut GtkBox, button);
    }

    let auto_selected = selected_quality_url.is_null() as c_int;
    let auto_button = player_stream_settings_item_button_new(
        if auto_selected != 0 {
            b"Auto (current)\0".as_ptr() as *const c_char
        } else {
            b"Auto\0".as_ptr() as *const c_char
        },
        auto_selected,
    );
    connect_clicked(auto_button, auto_clicked_callback, auto_user_data);
    gtk_box_append(quality_list_box as *mut GtkBox, auto_button);
}
