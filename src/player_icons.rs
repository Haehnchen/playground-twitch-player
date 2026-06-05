use std::f64::consts::PI;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

const GTK_ALIGN_CENTER: c_int = 3;
const CAIRO_LINE_CAP_ROUND: c_int = 1;
const CAIRO_LINE_JOIN_ROUND: c_int = 1;

const PLAYER_WINDOW_ICON_MINIMIZE: c_int = 0;
const PLAYER_WINDOW_ICON_FULLSCREEN: c_int = 1;
const PLAYER_LAYOUT_ICON_SINGLE: c_int = 0;
const PLAYER_CHAT_ICON_OPEN: c_int = 0;
const PLAYER_VOLUME_ICON_MUTED: c_int = 1;
const PLAYER_TILE_FOCUS_ICON_EXPAND: c_int = 0;

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkDrawingArea {
    _private: [u8; 0],
}

#[repr(C)]
pub struct Cairo {
    _private: [u8; 0],
}

type DrawFunc = unsafe extern "C" fn(*mut GtkDrawingArea, *mut Cairo, c_int, c_int, *mut c_void);

unsafe extern "C" {
    fn cairo_arc(cr: *mut Cairo, xc: f64, yc: f64, radius: f64, angle1: f64, angle2: f64);
    fn cairo_close_path(cr: *mut Cairo);
    fn cairo_fill(cr: *mut Cairo);
    fn cairo_fill_preserve(cr: *mut Cairo);
    fn cairo_line_to(cr: *mut Cairo, x: f64, y: f64);
    fn cairo_move_to(cr: *mut Cairo, x: f64, y: f64);
    fn cairo_new_sub_path(cr: *mut Cairo);
    fn cairo_rectangle(cr: *mut Cairo, x: f64, y: f64, width: f64, height: f64);
    fn cairo_set_line_cap(cr: *mut Cairo, line_cap: c_int);
    fn cairo_set_line_join(cr: *mut Cairo, line_join: c_int);
    fn cairo_set_line_width(cr: *mut Cairo, width: f64);
    fn cairo_set_source_rgba(cr: *mut Cairo, red: f64, green: f64, blue: f64, alpha: f64);
    fn cairo_stroke(cr: *mut Cairo);
    fn gtk_drawing_area_new() -> *mut GtkWidget;
    fn gtk_drawing_area_set_content_height(self_: *mut GtkDrawingArea, content_height: c_int);
    fn gtk_drawing_area_set_content_width(self_: *mut GtkDrawingArea, content_width: c_int);
    fn gtk_drawing_area_set_draw_func(
        self_: *mut GtkDrawingArea,
        draw_func: Option<DrawFunc>,
        user_data: *mut c_void,
        destroy: *mut c_void,
    );
    fn gtk_image_new_from_icon_name(icon_name: *const c_char) -> *mut GtkWidget;
    fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
}

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
    gtk_drawing_area_set_content_width(icon as *mut GtkDrawingArea, width);
    gtk_drawing_area_set_content_height(icon as *mut GtkDrawingArea, height);
    gtk_drawing_area_set_draw_func(
        icon as *mut GtkDrawingArea,
        Some(func),
        data,
        ptr::null_mut(),
    );
}

unsafe extern "C" fn draw_settings_icon(
    _area: *mut GtkDrawingArea,
    cr: *mut Cairo,
    width: c_int,
    height: c_int,
    _user_data: *mut c_void,
) {
    let size = (width.min(height)) as f64;
    let gear = size * 0.56;
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let outer = gear * 0.39;
    let inner = gear * 0.20;
    let dirs = [
        (1.0, 0.0),
        (0.7071, 0.7071),
        (0.0, 1.0),
        (-0.7071, 0.7071),
        (-1.0, 0.0),
        (-0.7071, -0.7071),
        (0.0, -1.0),
        (0.7071, -0.7071),
    ];

    cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.94);
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);
    cairo_set_line_width(cr, 1.2_f64.max(gear * 0.10));

    for (dx, dy) in dirs {
        cairo_move_to(cr, cx + dx * gear * 0.31, cy + dy * gear * 0.31);
        cairo_line_to(cr, cx + dx * outer, cy + dy * outer);
    }
    cairo_stroke(cr);

    cairo_set_line_width(cr, 1.1_f64.max(gear * 0.09));
    cairo_arc(cr, cx, cy, gear * 0.29, 0.0, 2.0 * PI);
    cairo_stroke(cr);

    cairo_arc(cr, cx, cy, inner, 0.0, 2.0 * PI);
    cairo_stroke(cr);
}

unsafe extern "C" fn draw_info_icon(
    _area: *mut GtkDrawingArea,
    cr: *mut Cairo,
    width: c_int,
    height: c_int,
    _user_data: *mut c_void,
) {
    let size = (width.min(height)) as f64;
    let x = width as f64 / 2.0;
    let y = height as f64 / 2.0;
    let radius = size * 0.38;

    cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.94);
    cairo_set_line_width(cr, 1.7_f64.max(size * 0.08));
    cairo_arc(cr, x, y, radius, 0.0, 2.0 * PI);
    cairo_stroke(cr);

    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_move_to(cr, x, y - radius * 0.05);
    cairo_line_to(cr, x, y + radius * 0.50);
    cairo_stroke(cr);

    cairo_arc(cr, x, y - radius * 0.50, size * 0.045, 0.0, 2.0 * PI);
    cairo_fill(cr);
}

unsafe extern "C" fn draw_stream_settings_icon(
    _area: *mut GtkDrawingArea,
    cr: *mut Cairo,
    width: c_int,
    height: c_int,
    _user_data: *mut c_void,
) {
    let size = (width.min(height)) as f64;
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let outer = size * 0.34;
    let tooth = size * 0.05;
    let inner = size * 0.15;
    let dirs = [
        (1.0, 0.0),
        (0.7071, 0.7071),
        (0.0, 1.0),
        (-0.7071, 0.7071),
        (-1.0, 0.0),
        (-0.7071, -0.7071),
        (0.0, -1.0),
        (0.7071, -0.7071),
    ];

    cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.95);
    cairo_set_line_width(cr, 1.5_f64.max(size * 0.085));
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);

    for (dx, dy) in dirs {
        cairo_move_to(cr, cx + dx * (outer - tooth), cy + dy * (outer - tooth));
        cairo_line_to(
            cr,
            cx + dx * (outer + tooth * 0.45),
            cy + dy * (outer + tooth * 0.45),
        );
    }
    cairo_stroke(cr);

    cairo_arc(cr, cx, cy, outer - tooth * 0.7, 0.0, 2.0 * PI);
    cairo_stroke(cr);

    cairo_arc(cr, cx, cy, inner, 0.0, 2.0 * PI);
    cairo_stroke(cr);
}

unsafe extern "C" fn draw_trash_icon(
    _area: *mut GtkDrawingArea,
    cr: *mut Cairo,
    width: c_int,
    height: c_int,
    _user_data: *mut c_void,
) {
    let size = (width.min(height)) as f64;
    let x = (width as f64 - size) / 2.0;
    let y = (height as f64 - size) / 2.0;

    cairo_set_source_rgba(cr, 0.94, 0.94, 0.95, 0.82);
    cairo_set_line_width(cr, 1.0_f64.max(size * 0.065));
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);

    cairo_move_to(cr, x + size * 0.27, y + size * 0.34);
    cairo_line_to(cr, x + size * 0.73, y + size * 0.34);
    cairo_stroke(cr);

    cairo_move_to(cr, x + size * 0.39, y + size * 0.34);
    cairo_line_to(cr, x + size * 0.39, y + size * 0.27);
    cairo_line_to(cr, x + size * 0.61, y + size * 0.27);
    cairo_line_to(cr, x + size * 0.61, y + size * 0.34);
    cairo_stroke(cr);

    cairo_move_to(cr, x + size * 0.33, y + size * 0.43);
    cairo_line_to(cr, x + size * 0.38, y + size * 0.76);
    cairo_line_to(cr, x + size * 0.62, y + size * 0.76);
    cairo_line_to(cr, x + size * 0.67, y + size * 0.43);
    cairo_stroke(cr);

    cairo_move_to(cr, x + size * 0.45, y + size * 0.51);
    cairo_line_to(cr, x + size * 0.45, y + size * 0.67);
    cairo_move_to(cr, x + size * 0.55, y + size * 0.51);
    cairo_line_to(cr, x + size * 0.55, y + size * 0.67);
    cairo_stroke(cr);
}

unsafe extern "C" fn draw_plus_icon(
    _area: *mut GtkDrawingArea,
    cr: *mut Cairo,
    width: c_int,
    height: c_int,
    _user_data: *mut c_void,
) {
    let size = (width.min(height)) as f64;
    let x = width as f64 / 2.0;
    let y = height as f64 / 2.0;
    let radius = size * 0.34;

    cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.50);
    cairo_set_line_width(cr, 1.6_f64.max(size * 0.08));
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_arc(cr, x, y, radius, 0.0, 2.0 * PI);
    cairo_stroke(cr);

    cairo_move_to(cr, x - radius * 0.48, y);
    cairo_line_to(cr, x + radius * 0.48, y);
    cairo_move_to(cr, x, y - radius * 0.48);
    cairo_line_to(cr, x, y + radius * 0.48);
    cairo_stroke(cr);
}

unsafe extern "C" fn draw_window_icon(
    _area: *mut GtkDrawingArea,
    cr: *mut Cairo,
    width: c_int,
    height: c_int,
    user_data: *mut c_void,
) {
    let kind = kind_from_pointer(user_data);
    let size = (width.min(height)) as f64;
    let x = (width as f64 - size) / 2.0;
    let y = (height as f64 - size) / 2.0;
    let left = x + size * 0.25;
    let right = x + size * 0.75;
    let top = y + size * 0.25;
    let bottom = y + size * 0.75;
    let center_y = y + size * 0.55;

    cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.94);
    cairo_set_line_width(cr, 1.8_f64.max(size * 0.10));
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);

    if kind == PLAYER_WINDOW_ICON_MINIMIZE {
        cairo_move_to(cr, left, center_y);
        cairo_line_to(cr, right, center_y);
    } else if kind == PLAYER_WINDOW_ICON_FULLSCREEN {
        let inset = size * 0.06;
        cairo_rectangle(
            cr,
            left + inset,
            top + inset,
            right - left - inset * 2.0,
            bottom - top - inset * 2.0,
        );
    } else {
        cairo_move_to(cr, left, top);
        cairo_line_to(cr, right, bottom);
        cairo_move_to(cr, right, top);
        cairo_line_to(cr, left, bottom);
    }

    cairo_stroke(cr);
}

unsafe extern "C" fn draw_layout_icon(
    _area: *mut GtkDrawingArea,
    cr: *mut Cairo,
    width: c_int,
    height: c_int,
    user_data: *mut c_void,
) {
    let kind = kind_from_pointer(user_data);
    let size = (width.min(height)) as f64 * 0.68;
    let x = (width as f64 - size) / 2.0 + size * 0.17;
    let y = (height as f64 - size) / 2.0 + size * 0.17;
    let extent = size * 0.66;

    cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.94);
    cairo_set_line_width(cr, 1.6_f64.max(size * 0.08));
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);

    if kind == PLAYER_LAYOUT_ICON_SINGLE {
        cairo_rectangle(cr, x, y, extent, extent);
        cairo_stroke(cr);
        return;
    }

    let gap = size * 0.08;
    let cell = (extent - gap) / 2.0;
    for row in 0..2 {
        for col in 0..2 {
            cairo_rectangle(
                cr,
                x + col as f64 * (cell + gap),
                y + row as f64 * (cell + gap),
                cell,
                cell,
            );
        }
    }
    cairo_stroke(cr);
}

unsafe extern "C" fn draw_chat_icon(
    _area: *mut GtkDrawingArea,
    cr: *mut Cairo,
    width: c_int,
    height: c_int,
    user_data: *mut c_void,
) {
    let kind = kind_from_pointer(user_data);
    let size = (width.min(height)) as f64;
    let x = (width as f64 - size) / 2.0;
    let y = (height as f64 - size) / 2.0;

    cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.94);
    cairo_set_line_width(cr, 1.8_f64.max(size * 0.09));
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);

    let bx = x + size * 0.12;
    let by = y + size * 0.18;
    let bw = size * 0.62;
    let bh = size * 0.44;
    let r = size * 0.12;

    cairo_new_sub_path(cr);
    cairo_arc(cr, bx + bw - r, by + r, r, -PI / 2.0, 0.0);
    cairo_arc(cr, bx + bw - r, by + bh - r, r, 0.0, PI / 2.0);
    cairo_arc(cr, bx + r, by + bh - r, r, PI / 2.0, PI);
    cairo_arc(cr, bx + r, by + r, r, PI, 3.0 * PI / 2.0);
    cairo_close_path(cr);
    cairo_stroke(cr);

    cairo_move_to(cr, bx + size * 0.16, by + bh);
    cairo_line_to(cr, bx + size * 0.08, by + size * 0.72);
    cairo_line_to(cr, bx + size * 0.30, by + bh);
    cairo_stroke(cr);

    for i in 0..3 {
        cairo_arc(
            cr,
            bx + bw * (0.32 + i as f64 * 0.18),
            by + bh * 0.50,
            size * 0.030,
            0.0,
            2.0 * PI,
        );
        cairo_fill(cr);
    }

    let badge_x = x + size * 0.68;
    let badge_y = y + size * 0.62;
    let badge_r = size * 0.20;

    cairo_set_source_rgba(cr, 0.05, 0.05, 0.05, 0.95);
    cairo_arc(cr, badge_x, badge_y, badge_r, 0.0, 2.0 * PI);
    cairo_fill_preserve(cr);

    cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.95);
    cairo_set_line_width(cr, 1.7_f64.max(size * 0.08));
    cairo_stroke(cr);

    cairo_move_to(cr, badge_x - badge_r * 0.48, badge_y);
    cairo_line_to(cr, badge_x + badge_r * 0.48, badge_y);
    if kind == PLAYER_CHAT_ICON_OPEN {
        cairo_move_to(cr, badge_x, badge_y - badge_r * 0.48);
        cairo_line_to(cr, badge_x, badge_y + badge_r * 0.48);
    }
    cairo_stroke(cr);
}

unsafe extern "C" fn draw_volume_icon(
    _area: *mut GtkDrawingArea,
    cr: *mut Cairo,
    width: c_int,
    height: c_int,
    user_data: *mut c_void,
) {
    let kind = kind_from_pointer(user_data);
    let size = (width.min(height)) as f64;
    let x = (width as f64 - size) / 2.0;
    let y = (height as f64 - size) / 2.0;
    let left = x + size * 0.06;
    let right = x + size * 0.60;
    let top = y + size * 0.12;
    let bottom = y + size * 0.88;
    let body_right = x + size * 0.34;
    let body_top = y + size * 0.28;
    let body_bottom = y + size * 0.72;
    let center_y = y + size * 0.50;

    cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.94);
    cairo_set_line_width(cr, 1.5_f64.max(size * 0.095));
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);

    cairo_move_to(cr, left, body_top);
    cairo_line_to(cr, body_right, body_top);
    cairo_line_to(cr, right, top);
    cairo_line_to(cr, right, bottom);
    cairo_line_to(cr, body_right, body_bottom);
    cairo_line_to(cr, left, body_bottom);
    cairo_close_path(cr);
    cairo_stroke(cr);

    if kind == PLAYER_VOLUME_ICON_MUTED {
        let cx = x + size * 0.80;
        let cy = center_y;
        let mark = size * 0.22;

        cairo_set_line_width(cr, 1.5_f64.max(size * 0.095));
        cairo_move_to(cr, cx - mark, cy - mark);
        cairo_line_to(cr, cx + mark, cy + mark);
        cairo_move_to(cr, cx + mark, cy - mark);
        cairo_line_to(cr, cx - mark, cy + mark);
    } else {
        cairo_set_line_width(cr, 1.35_f64.max(size * 0.085));
        cairo_new_sub_path(cr);
        cairo_arc(cr, x + size * 0.52, center_y, size * 0.29, -0.82, 0.82);
        cairo_new_sub_path(cr);
        cairo_arc(cr, x + size * 0.51, center_y, size * 0.46, -0.73, 0.73);
    }

    cairo_stroke(cr);
}

unsafe extern "C" fn draw_tile_focus_icon(
    _area: *mut GtkDrawingArea,
    cr: *mut Cairo,
    width: c_int,
    height: c_int,
    user_data: *mut c_void,
) {
    let kind = kind_from_pointer(user_data);
    let size = (width.min(height)) as f64;
    let x = (width as f64 - size) / 2.0;
    let y = (height as f64 - size) / 2.0;
    let outer_left = x + size * 0.20;
    let outer_right = x + size * 0.80;
    let outer_top = y + size * 0.20;
    let outer_bottom = y + size * 0.80;
    let inner_left = x + size * 0.36;
    let inner_right = x + size * 0.64;
    let inner_top = y + size * 0.36;
    let inner_bottom = y + size * 0.64;
    let corner = size * 0.16;

    cairo_set_source_rgba(cr, 1.0, 1.0, 1.0, 0.94);
    cairo_set_line_width(cr, 1.7_f64.max(size * 0.10));
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);

    if kind == PLAYER_TILE_FOCUS_ICON_EXPAND {
        cairo_move_to(cr, inner_left, outer_top);
        cairo_line_to(cr, outer_left, outer_top);
        cairo_line_to(cr, outer_left, inner_top);
        cairo_move_to(cr, inner_right, outer_top);
        cairo_line_to(cr, outer_right, outer_top);
        cairo_line_to(cr, outer_right, inner_top);
        cairo_move_to(cr, outer_left, inner_bottom);
        cairo_line_to(cr, outer_left, outer_bottom);
        cairo_line_to(cr, inner_left, outer_bottom);
        cairo_move_to(cr, outer_right, inner_bottom);
        cairo_line_to(cr, outer_right, outer_bottom);
        cairo_line_to(cr, inner_right, outer_bottom);
    } else {
        cairo_move_to(cr, outer_left, inner_top);
        cairo_line_to(cr, inner_left + corner, inner_top);
        cairo_line_to(cr, inner_left + corner, outer_top);
        cairo_move_to(cr, outer_right, inner_top);
        cairo_line_to(cr, inner_right - corner, inner_top);
        cairo_line_to(cr, inner_right - corner, outer_top);
        cairo_move_to(cr, inner_left + corner, outer_bottom);
        cairo_line_to(cr, inner_left + corner, inner_bottom);
        cairo_line_to(cr, outer_left, inner_bottom);
        cairo_move_to(cr, inner_right - corner, outer_bottom);
        cairo_line_to(cr, inner_right - corner, inner_bottom);
        cairo_line_to(cr, outer_right, inner_bottom);
    }

    cairo_stroke(cr);
}

pub unsafe fn player_settings_icon_new<W>() -> *mut W {
    let icon = gtk_drawing_area_new();
    set_drawing_area(icon, 18, 18, draw_settings_icon, ptr::null_mut());
    icon as *mut W
}

pub unsafe fn player_stream_settings_icon_new<W>() -> *mut W {
    let icon = gtk_drawing_area_new();
    set_drawing_area(icon, 13, 13, draw_stream_settings_icon, ptr::null_mut());
    icon as *mut W
}

pub unsafe fn player_info_icon_new<W>() -> *mut W {
    let icon = gtk_drawing_area_new();
    set_drawing_area(icon, 16, 16, draw_info_icon, ptr::null_mut());
    icon as *mut W
}

pub unsafe fn player_refresh_icon_new<W>() -> *mut W {
    let icon = gtk_image_new_from_icon_name(b"view-refresh-symbolic\0".as_ptr() as *const c_char);
    gtk_widget_set_halign(icon, GTK_ALIGN_CENTER);
    gtk_widget_set_valign(icon, GTK_ALIGN_CENTER);
    icon as *mut W
}

pub unsafe fn player_trash_icon_new<W>() -> *mut W {
    let icon = gtk_drawing_area_new();
    set_drawing_area(icon, 15, 15, draw_trash_icon, ptr::null_mut());
    icon as *mut W
}

pub unsafe fn player_plus_icon_new<W>() -> *mut W {
    let icon = gtk_drawing_area_new();
    set_drawing_area(icon, 30, 30, draw_plus_icon, ptr::null_mut());
    icon as *mut W
}

pub unsafe fn player_window_icon_new<W>(kind: c_int) -> *mut W {
    let icon = gtk_drawing_area_new();
    set_drawing_area(icon, 16, 16, draw_window_icon, kind_to_pointer(kind));
    icon as *mut W
}

pub unsafe fn player_layout_icon_new<W>(kind: c_int) -> *mut W {
    let icon = gtk_drawing_area_new();
    set_drawing_area(icon, 18, 18, draw_layout_icon, kind_to_pointer(kind));
    icon as *mut W
}

pub unsafe fn player_chat_icon_new<W>(kind: c_int) -> *mut W {
    let icon = gtk_drawing_area_new();
    set_drawing_area(icon, 16, 16, draw_chat_icon, kind_to_pointer(kind));
    icon as *mut W
}

pub unsafe fn player_volume_icon_new<W>(kind: c_int) -> *mut W {
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

pub unsafe fn player_tile_focus_icon_new<W>(kind: c_int) -> *mut W {
    let icon = gtk_drawing_area_new();
    set_drawing_area(icon, 16, 16, draw_tile_focus_icon, kind_to_pointer(kind));
    icon as *mut W
}
