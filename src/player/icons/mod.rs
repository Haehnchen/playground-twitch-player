use self::drawing::{
    draw_chat_icon, draw_info_icon, draw_layout_icon, draw_plus_icon, draw_settings_icon,
    draw_stream_settings_icon, draw_tile_focus_icon, draw_trash_icon, draw_volume_icon,
    draw_window_icon,
};
use crate::player::layout::PlayerLayout;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

mod drawing;

type DrawFunc = unsafe extern "C" fn(*mut GtkDrawingArea, *mut Cairo, c_int, c_int, *mut c_void);

fn kind_from_pointer(user_data: *mut c_void) -> c_int {
    user_data as isize as c_int
}

fn kind_to_pointer(kind: c_int) -> *mut c_void {
    kind as isize as *mut c_void
}

unsafe fn set_drawing_area(
    icon: *mut GtkWidget,
    width: c_int,
    height: c_int,
    func: DrawFunc,
    data: *mut c_void,
) {
    unsafe {
        gtk_drawing_area_set_content_width(icon as *mut GtkDrawingArea, width);
        gtk_drawing_area_set_content_height(icon as *mut GtkDrawingArea, height);
        gtk_drawing_area_set_draw_func(
            icon as *mut GtkDrawingArea,
            Some(func),
            data,
            ptr::null_mut(),
        );
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_settings_icon_new<W>() -> *mut W {
    unsafe {
        let icon = gtk_drawing_area_new();
        set_drawing_area(icon, 18, 18, draw_settings_icon, ptr::null_mut());
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_stream_settings_icon_new<W>() -> *mut W {
    unsafe {
        let icon = gtk_drawing_area_new();
        set_drawing_area(icon, 13, 13, draw_stream_settings_icon, ptr::null_mut());
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_info_icon_new<W>() -> *mut W {
    unsafe {
        let icon = gtk_drawing_area_new();
        set_drawing_area(icon, 16, 16, draw_info_icon, ptr::null_mut());
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_account_icon_new<W>() -> *mut W {
    unsafe {
        let icon = gtk_image_new_from_icon_name(c"avatar-default-symbolic".as_ptr());
        gtk_widget_set_halign(icon, GTK_ALIGN_CENTER);
        gtk_widget_set_valign(icon, GTK_ALIGN_CENTER);
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_refresh_icon_new<W>() -> *mut W {
    unsafe {
        let icon = gtk_image_new_from_icon_name(c"view-refresh-symbolic".as_ptr());
        gtk_widget_set_halign(icon, GTK_ALIGN_CENTER);
        gtk_widget_set_valign(icon, GTK_ALIGN_CENTER);
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_trash_icon_new<W>() -> *mut W {
    unsafe {
        let icon = gtk_drawing_area_new();
        set_drawing_area(icon, 15, 15, draw_trash_icon, ptr::null_mut());
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_plus_icon_new<W>() -> *mut W {
    unsafe {
        let icon = gtk_drawing_area_new();
        set_drawing_area(icon, 30, 30, draw_plus_icon, ptr::null_mut());
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_window_icon_new<W>(kind: c_int) -> *mut W {
    unsafe {
        let icon = gtk_drawing_area_new();
        set_drawing_area(icon, 16, 16, draw_window_icon, kind_to_pointer(kind));
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_layout_icon_new<W>(layout: &'static PlayerLayout) -> *mut W {
    unsafe {
        let icon = gtk_drawing_area_new();
        set_drawing_area(
            icon,
            18,
            18,
            draw_layout_icon,
            layout as *const PlayerLayout as *mut c_void,
        );
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_layout_menu_icon_new<W>() -> *mut W {
    unsafe {
        let icon = gtk_image_new_from_icon_name(c"pan-down-symbolic".as_ptr());
        gtk_widget_set_halign(icon, GTK_ALIGN_CENTER);
        gtk_widget_set_valign(icon, GTK_ALIGN_CENTER);
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_chat_icon_new<W>(kind: c_int) -> *mut W {
    unsafe {
        let icon = gtk_drawing_area_new();
        set_drawing_area(icon, 16, 16, draw_chat_icon, kind_to_pointer(kind));
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_volume_icon_new<W>(kind: c_int) -> *mut W {
    unsafe {
        let icon = gtk_drawing_area_new();
        gtk_drawing_area_set_content_width(icon as *mut GtkDrawingArea, 16);
        gtk_drawing_area_set_content_height(icon as *mut GtkDrawingArea, 16);
        gtk_widget_set_halign(icon, GTK_ALIGN_CENTER);
        gtk_widget_set_valign(icon, GTK_ALIGN_CENTER);
        gtk_widget_set_hexpand(icon, 0);
        gtk_widget_set_vexpand(icon, 0);
        gtk_drawing_area_set_draw_func(
            icon as *mut GtkDrawingArea,
            Some(draw_volume_icon),
            kind_to_pointer(kind),
            ptr::null_mut(),
        );
        icon as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `W`: compatible GTK widget type.
pub unsafe fn player_tile_focus_icon_new<W>(kind: c_int) -> *mut W {
    unsafe {
        let icon = gtk_drawing_area_new();
        set_drawing_area(icon, 16, 16, draw_tile_focus_icon, kind_to_pointer(kind));
        icon as *mut W
    }
}

pub(super) const GTK_ALIGN_CENTER: c_int = 3;

pub use gtk4::ffi::{GtkDrawingArea, GtkWidget};

pub use gtk4::cairo::ffi::cairo_t as Cairo;

unsafe extern "C" {
    pub(super) fn cairo_arc(
        cr: *mut Cairo,
        xc: f64,
        yc: f64,
        radius: f64,
        angle1: f64,
        angle2: f64,
    );
    pub(super) fn cairo_close_path(cr: *mut Cairo);
    pub(super) fn cairo_fill(cr: *mut Cairo);
    pub(super) fn cairo_fill_preserve(cr: *mut Cairo);
    pub(super) fn cairo_line_to(cr: *mut Cairo, x: f64, y: f64);
    pub(super) fn cairo_move_to(cr: *mut Cairo, x: f64, y: f64);
    pub(super) fn cairo_new_sub_path(cr: *mut Cairo);
    pub(super) fn cairo_rectangle(cr: *mut Cairo, x: f64, y: f64, width: f64, height: f64);
    pub(super) fn cairo_set_line_cap(cr: *mut Cairo, line_cap: c_int);
    pub(super) fn cairo_set_line_join(cr: *mut Cairo, line_join: c_int);
    pub(super) fn cairo_set_line_width(cr: *mut Cairo, width: f64);
    pub(super) fn cairo_set_source_rgba(
        cr: *mut Cairo,
        red: f64,
        green: f64,
        blue: f64,
        alpha: f64,
    );
    pub(super) fn cairo_stroke(cr: *mut Cairo);
    pub(super) fn gtk_drawing_area_new() -> *mut GtkWidget;
    pub(super) fn gtk_drawing_area_set_content_height(
        self_: *mut GtkDrawingArea,
        content_height: c_int,
    );
    pub(super) fn gtk_drawing_area_set_content_width(
        self_: *mut GtkDrawingArea,
        content_width: c_int,
    );
    pub(super) fn gtk_drawing_area_set_draw_func(
        self_: *mut GtkDrawingArea,
        draw_func: Option<DrawFunc>,
        user_data: *mut c_void,
        destroy: *mut c_void,
    );
    pub(super) fn gtk_image_new_from_icon_name(icon_name: *const c_char) -> *mut GtkWidget;
    pub(super) fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    pub(super) fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    pub(super) fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
    pub(super) fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
}
