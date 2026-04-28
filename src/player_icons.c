#include "player_icons.h"

#include <math.h>

static void draw_settings_icon(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)area;
    (void)user_data;
    double size = MIN(width, height);
    double gear = size * 0.56;
    double cx = width / 2.0;
    double cy = height / 2.0;
    double outer = gear * 0.39;
    double inner = gear * 0.20;
    static const double dirs[][2] = {
        {1.0, 0.0},
        {0.7071, 0.7071},
        {0.0, 1.0},
        {-0.7071, 0.7071},
        {-1.0, 0.0},
        {-0.7071, -0.7071},
        {0.0, -1.0},
        {0.7071, -0.7071},
    };

    cairo_set_source_rgba(cr, 1, 1, 1, 0.94);
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);
    cairo_set_line_width(cr, MAX(1.2, gear * 0.10));

    for (guint i = 0; i < G_N_ELEMENTS(dirs); i++) {
        double sx = cx + dirs[i][0] * gear * 0.31;
        double sy = cy + dirs[i][1] * gear * 0.31;
        double ex = cx + dirs[i][0] * outer;
        double ey = cy + dirs[i][1] * outer;

        cairo_move_to(cr, sx, sy);
        cairo_line_to(cr, ex, ey);
    }
    cairo_stroke(cr);

    cairo_set_line_width(cr, MAX(1.1, gear * 0.09));
    cairo_arc(cr, cx, cy, gear * 0.29, 0, 2 * G_PI);
    cairo_stroke(cr);

    cairo_arc(cr, cx, cy, inner, 0, 2 * G_PI);
    cairo_stroke(cr);
}

static void draw_info_icon(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)area;
    (void)user_data;
    double size = MIN(width, height);
    double x = width / 2.0;
    double y = height / 2.0;
    double radius = size * 0.38;

    cairo_set_source_rgba(cr, 1, 1, 1, 0.94);
    cairo_set_line_width(cr, MAX(1.7, size * 0.08));
    cairo_arc(cr, x, y, radius, 0, 2 * G_PI);
    cairo_stroke(cr);

    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_move_to(cr, x, y - radius * 0.05);
    cairo_line_to(cr, x, y + radius * 0.50);
    cairo_stroke(cr);

    cairo_arc(cr, x, y - radius * 0.50, size * 0.045, 0, 2 * G_PI);
    cairo_fill(cr);
}

static void draw_trash_icon(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)area;
    (void)user_data;
    double size = MIN(width, height);
    double x = (width - size) / 2.0;
    double y = (height - size) / 2.0;

    cairo_set_source_rgba(cr, 0.94, 0.94, 0.95, 0.82);
    cairo_set_line_width(cr, MAX(1.0, size * 0.065));
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

static void draw_window_icon(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)area;
    PlayerWindowIconKind kind = GPOINTER_TO_INT(user_data);
    double size = MIN(width, height);
    double x = (width - size) / 2.0;
    double y = (height - size) / 2.0;
    double left = x + size * 0.25;
    double right = x + size * 0.75;
    double top = y + size * 0.25;
    double bottom = y + size * 0.75;
    double center_y = y + size * 0.55;

    cairo_set_source_rgba(cr, 1, 1, 1, 0.94);
    cairo_set_line_width(cr, MAX(1.8, size * 0.10));
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);

    if (kind == PLAYER_WINDOW_ICON_MINIMIZE) {
        cairo_move_to(cr, left, center_y);
        cairo_line_to(cr, right, center_y);
    } else if (kind == PLAYER_WINDOW_ICON_FULLSCREEN) {
        double inset = size * 0.06;
        cairo_rectangle(cr, left + inset, top + inset, right - left - inset * 2, bottom - top - inset * 2);
    } else {
        cairo_move_to(cr, left, top);
        cairo_line_to(cr, right, bottom);
        cairo_move_to(cr, right, top);
        cairo_line_to(cr, left, bottom);
    }

    cairo_stroke(cr);
}

static void draw_layout_icon(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)area;
    PlayerLayoutIconKind kind = GPOINTER_TO_INT(user_data);
    double size = MIN(width, height) * 0.68;
    double x = (width - size) / 2.0 + size * 0.17;
    double y = (height - size) / 2.0 + size * 0.17;
    double extent = size * 0.66;

    cairo_set_source_rgba(cr, 1, 1, 1, 0.94);
    cairo_set_line_width(cr, MAX(1.6, size * 0.08));
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);

    if (kind == PLAYER_LAYOUT_ICON_SINGLE) {
        cairo_rectangle(cr, x, y, extent, extent);
        cairo_stroke(cr);
        return;
    }

    double gap = size * 0.08;
    double cell = (extent - gap) / 2.0;
    for (int row = 0; row < 2; row++) {
        for (int col = 0; col < 2; col++) {
            cairo_rectangle(cr, x + col * (cell + gap), y + row * (cell + gap), cell, cell);
        }
    }
    cairo_stroke(cr);
}

static void draw_chat_icon(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)area;
    PlayerChatIconKind kind = GPOINTER_TO_INT(user_data);
    double size = MIN(width, height);
    double x = (width - size) / 2.0;
    double y = (height - size) / 2.0;

    cairo_set_source_rgba(cr, 1, 1, 1, 0.94);
    cairo_set_line_width(cr, MAX(1.8, size * 0.09));
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);

    double bx = x + size * 0.12;
    double by = y + size * 0.18;
    double bw = size * 0.62;
    double bh = size * 0.44;
    double r = size * 0.12;

    cairo_new_sub_path(cr);
    cairo_arc(cr, bx + bw - r, by + r, r, -G_PI / 2, 0);
    cairo_arc(cr, bx + bw - r, by + bh - r, r, 0, G_PI / 2);
    cairo_arc(cr, bx + r, by + bh - r, r, G_PI / 2, G_PI);
    cairo_arc(cr, bx + r, by + r, r, G_PI, 3 * G_PI / 2);
    cairo_close_path(cr);
    cairo_stroke(cr);

    cairo_move_to(cr, bx + size * 0.16, by + bh);
    cairo_line_to(cr, bx + size * 0.08, by + size * 0.72);
    cairo_line_to(cr, bx + size * 0.30, by + bh);
    cairo_stroke(cr);

    for (int i = 0; i < 3; i++) {
        double dot_x = bx + bw * (0.32 + i * 0.18);
        double dot_y = by + bh * 0.50;
        cairo_arc(cr, dot_x, dot_y, size * 0.030, 0, 2 * G_PI);
        cairo_fill(cr);
    }

    double badge_x = x + size * 0.68;
    double badge_y = y + size * 0.62;
    double badge_r = size * 0.20;

    cairo_set_source_rgba(cr, 0.05, 0.05, 0.05, 0.95);
    cairo_arc(cr, badge_x, badge_y, badge_r, 0, 2 * G_PI);
    cairo_fill_preserve(cr);

    cairo_set_source_rgba(cr, 1, 1, 1, 0.95);
    cairo_set_line_width(cr, MAX(1.7, size * 0.08));
    cairo_stroke(cr);

    cairo_move_to(cr, badge_x - badge_r * 0.48, badge_y);
    cairo_line_to(cr, badge_x + badge_r * 0.48, badge_y);
    if (kind == PLAYER_CHAT_ICON_OPEN) {
        cairo_move_to(cr, badge_x, badge_y - badge_r * 0.48);
        cairo_line_to(cr, badge_x, badge_y + badge_r * 0.48);
    }
    cairo_stroke(cr);
}

static void draw_tile_focus_icon(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)area;
    PlayerTileFocusIconKind kind = GPOINTER_TO_INT(user_data);
    double size = MIN(width, height);
    double x = (width - size) / 2.0;
    double y = (height - size) / 2.0;
    double outer_left = x + size * 0.20;
    double outer_right = x + size * 0.80;
    double outer_top = y + size * 0.20;
    double outer_bottom = y + size * 0.80;
    double inner_left = x + size * 0.36;
    double inner_right = x + size * 0.64;
    double inner_top = y + size * 0.36;
    double inner_bottom = y + size * 0.64;
    double corner = size * 0.16;

    cairo_set_source_rgba(cr, 1, 1, 1, 0.94);
    cairo_set_line_width(cr, MAX(1.7, size * 0.10));
    cairo_set_line_cap(cr, CAIRO_LINE_CAP_ROUND);
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);

    if (kind == PLAYER_TILE_FOCUS_ICON_EXPAND) {
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

GtkWidget *player_settings_icon_new(void)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_settings_icon, NULL, NULL);
    return icon;
}

GtkWidget *player_info_icon_new(void)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_info_icon, NULL, NULL);
    return icon;
}

GtkWidget *player_trash_icon_new(void)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 15);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 15);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_trash_icon, NULL, NULL);
    return icon;
}

GtkWidget *player_window_icon_new(PlayerWindowIconKind kind)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 16);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 16);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_window_icon, GINT_TO_POINTER(kind), NULL);
    return icon;
}

GtkWidget *player_layout_icon_new(PlayerLayoutIconKind kind)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_layout_icon, GINT_TO_POINTER(kind), NULL);
    return icon;
}

GtkWidget *player_chat_icon_new(PlayerChatIconKind kind)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_chat_icon, GINT_TO_POINTER(kind), NULL);
    return icon;
}

GtkWidget *player_tile_focus_icon_new(PlayerTileFocusIconKind kind)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_tile_focus_icon, GINT_TO_POINTER(kind), NULL);
    return icon;
}
