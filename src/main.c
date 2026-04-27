#define G_LOG_DOMAIN "twitch-player"

#include <gio/gio.h>
#include <gtk/gtk.h>
#include <math.h>
#include <string.h>

#include "settings.h"
#include "player_defaults.h"
#include "single_player.h"
#include "grid_player.h"
#include "settings_window.h"

#define APP_ID "local.twitchplayer"
#define APP_ICON_RESOURCE_PATH "/local/twitch-player/icons/hicolor/scalable/apps/local.twitch-player.svg"
#define APP_ICON_RESOURCE_THEME_PATH "/local/twitch-player/icons"

#define OVERLAY_HIDE_DELAY_MS 1800

typedef enum {
    CONTENT_MODE_SINGLE,
    CONTENT_MODE_GRID,
} ContentMode;

typedef enum {
    WINDOW_ICON_MINIMIZE,
    WINDOW_ICON_FULLSCREEN,
    WINDOW_ICON_CLOSE,
} WindowIconKind;

typedef enum {
    LAYOUT_ICON_SINGLE,
    LAYOUT_ICON_GRID,
} LayoutIconKind;

typedef struct {
    GtkApplication *application;
    GtkWidget *window;
    GtkWidget *root_overlay;
    GtkWidget *top_left_controls;
    GtkWidget *top_controls;
    GtkWidget *settings_button;
    GtkWidget *layout_button;
    AppSettings *settings;
    SinglePlayer *single_player;
    GridPlayer *grid_player;
    const char *startup_target;
    const char * const *grid_targets;
    guint grid_target_count;
    char *single_target;
    gboolean has_single_target_handoff;
    int single_chat_paned_position;
    ContentMode content_mode;
    guint overlay_hide_source;
    gboolean closing;
    gboolean fullscreen;
} AppState;

typedef struct {
    const char *startup_target;
    const char * const *grid_targets;
    guint grid_target_count;
    gboolean start_in_grid;
} StartupConfig;

static void set_layout_mode(AppState *state, ContentMode mode);
static void show_window_overlay(AppState *state);

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

static GtkWidget *create_settings_icon(void)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_settings_icon, NULL, NULL);
    return icon;
}

static void draw_layout_icon(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)area;
    LayoutIconKind kind = GPOINTER_TO_INT(user_data);
    double size = MIN(width, height) * 0.68;
    double x = (width - size) / 2.0 + size * 0.17;
    double y = (height - size) / 2.0 + size * 0.17;
    double extent = size * 0.66;

    cairo_set_source_rgba(cr, 1, 1, 1, 0.94);
    cairo_set_line_width(cr, MAX(1.6, size * 0.08));
    cairo_set_line_join(cr, CAIRO_LINE_JOIN_ROUND);

    if (kind == LAYOUT_ICON_SINGLE) {
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

static GtkWidget *create_layout_icon(LayoutIconKind kind)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_layout_icon, GINT_TO_POINTER(kind), NULL);
    return icon;
}

static void draw_window_icon(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)area;
    WindowIconKind kind = GPOINTER_TO_INT(user_data);
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

    if (kind == WINDOW_ICON_MINIMIZE) {
        cairo_move_to(cr, left, center_y);
        cairo_line_to(cr, right, center_y);
    } else if (kind == WINDOW_ICON_FULLSCREEN) {
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

static GtkWidget *create_window_icon(WindowIconKind kind)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 16);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 16);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_window_icon, GINT_TO_POINTER(kind), NULL);
    return icon;
}

static GtkWidget *create_overlay_button(GtkWidget *icon, const char *tooltip)
{
    GtkWidget *button = gtk_button_new();
    gtk_button_set_child(GTK_BUTTON(button), icon);
    gtk_widget_add_css_class(button, "overlay-icon-button");
    gtk_widget_set_tooltip_text(button, tooltip);
    return button;
}

static gboolean hide_window_overlay(gpointer user_data)
{
    AppState *state = user_data;
    state->overlay_hide_source = 0;

    if (!state->closing) {
        gtk_widget_set_visible(state->top_left_controls, FALSE);
        gtk_widget_set_visible(state->top_controls, FALSE);
    }

    return G_SOURCE_REMOVE;
}

static void schedule_window_overlay_hide(AppState *state)
{
    if (state->overlay_hide_source != 0) {
        g_source_remove(state->overlay_hide_source);
    }

    state->overlay_hide_source = g_timeout_add(OVERLAY_HIDE_DELAY_MS, hide_window_overlay, state);
}

static void show_window_overlay(AppState *state)
{
    if (state->closing) {
        return;
    }

    gtk_widget_set_visible(state->top_left_controls, TRUE);
    gtk_widget_set_visible(state->top_controls, TRUE);
    if (state->single_player != NULL) {
        single_player_show_overlay(state->single_player);
    }
    schedule_window_overlay_hide(state);
}

static gboolean get_toplevel_event_data(GtkWidget *window, GtkGesture *gesture, GdkToplevel **toplevel, GdkDevice **device, double *x, double *y, guint32 *timestamp)
{
    GtkNative *native = gtk_widget_get_native(window);
    GdkSurface *surface = native != NULL ? gtk_native_get_surface(native) : NULL;
    GdkEventSequence *sequence = gtk_gesture_get_last_updated_sequence(gesture);
    GdkEvent *event = gtk_gesture_get_last_event(gesture, sequence);

    if (surface == NULL || !GDK_IS_TOPLEVEL(surface) || event == NULL) {
        return FALSE;
    }

    *device = gdk_event_get_device(event);
    *timestamp = gdk_event_get_time(event);

    if (*device == NULL || !gdk_event_get_position(event, x, y)) {
        return FALSE;
    }

    *toplevel = GDK_TOPLEVEL(surface);
    return TRUE;
}

static void begin_window_resize(AppState *state, GtkGesture *gesture, GdkSurfaceEdge edge)
{
    if (state->fullscreen) {
        return;
    }

    GdkToplevel *toplevel = NULL;
    GdkDevice *device = NULL;
    double x = 0;
    double y = 0;
    guint32 timestamp = 0;

    if (get_toplevel_event_data(state->window, gesture, &toplevel, &device, &x, &y, &timestamp)) {
        gdk_toplevel_begin_resize(toplevel, edge, device, GDK_BUTTON_PRIMARY, x, y, timestamp);
    }
}

static void on_resize_pressed(GtkGestureClick *gesture, int n_press, double x, double y, gpointer user_data)
{
    (void)x;
    (void)y;

    if (n_press != 1) {
        return;
    }

    GdkSurfaceEdge edge = GPOINTER_TO_INT(g_object_get_data(G_OBJECT(gesture), "resize-edge"));
    begin_window_resize(user_data, GTK_GESTURE(gesture), edge);
}

static GtkWidget *create_resize_handle(AppState *state, GdkSurfaceEdge edge, GtkAlign halign, GtkAlign valign, int width, int height, const char *cursor)
{
    GtkWidget *handle = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class(handle, "resize-handle");
    gtk_widget_set_halign(handle, halign);
    gtk_widget_set_valign(handle, valign);
    gtk_widget_set_size_request(handle, width, height);
    gtk_widget_set_cursor_from_name(handle, cursor);

    if (halign == GTK_ALIGN_FILL) {
        gtk_widget_set_hexpand(handle, TRUE);
    }
    if (valign == GTK_ALIGN_FILL) {
        gtk_widget_set_vexpand(handle, TRUE);
    }

    GtkGesture *click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(click), GDK_BUTTON_PRIMARY);
    g_object_set_data(G_OBJECT(click), "resize-edge", GINT_TO_POINTER(edge));
    g_signal_connect(click, "pressed", G_CALLBACK(on_resize_pressed), state);
    gtk_widget_add_controller(handle, GTK_EVENT_CONTROLLER(click));

    return handle;
}

static void add_resize_handles(GtkOverlay *overlay, AppState *state)
{
    gtk_overlay_add_overlay(overlay, create_resize_handle(state, GDK_SURFACE_EDGE_NORTH, GTK_ALIGN_FILL, GTK_ALIGN_START, -1, 6, "n-resize"));
    gtk_overlay_add_overlay(overlay, create_resize_handle(state, GDK_SURFACE_EDGE_SOUTH, GTK_ALIGN_FILL, GTK_ALIGN_END, -1, 6, "s-resize"));
    gtk_overlay_add_overlay(overlay, create_resize_handle(state, GDK_SURFACE_EDGE_WEST, GTK_ALIGN_START, GTK_ALIGN_FILL, 6, -1, "w-resize"));
    gtk_overlay_add_overlay(overlay, create_resize_handle(state, GDK_SURFACE_EDGE_EAST, GTK_ALIGN_END, GTK_ALIGN_FILL, 6, -1, "e-resize"));

    gtk_overlay_add_overlay(overlay, create_resize_handle(state, GDK_SURFACE_EDGE_NORTH_WEST, GTK_ALIGN_START, GTK_ALIGN_START, 12, 12, "nw-resize"));
    gtk_overlay_add_overlay(overlay, create_resize_handle(state, GDK_SURFACE_EDGE_NORTH_EAST, GTK_ALIGN_END, GTK_ALIGN_START, 12, 12, "ne-resize"));
    gtk_overlay_add_overlay(overlay, create_resize_handle(state, GDK_SURFACE_EDGE_SOUTH_WEST, GTK_ALIGN_START, GTK_ALIGN_END, 12, 12, "sw-resize"));
    gtk_overlay_add_overlay(overlay, create_resize_handle(state, GDK_SURFACE_EDGE_SOUTH_EAST, GTK_ALIGN_END, GTK_ALIGN_END, 12, 12, "se-resize"));
}

static void set_fullscreen(AppState *state, gboolean fullscreen)
{
    if (state->fullscreen == fullscreen) {
        return;
    }

    state->fullscreen = fullscreen;
    if (fullscreen) {
        gtk_window_fullscreen(GTK_WINDOW(state->window));
    } else {
        gtk_window_unfullscreen(GTK_WINDOW(state->window));
    }

    if (state->single_player != NULL) {
        single_player_set_fullscreen(state->single_player, fullscreen);
    }
    if (state->grid_player != NULL) {
        grid_player_set_fullscreen(state->grid_player, fullscreen);
    }

    show_window_overlay(state);
}

static void toggle_fullscreen(AppState *state)
{
    set_fullscreen(state, !state->fullscreen);
}

static void on_content_fullscreen_requested(gpointer user_data)
{
    toggle_fullscreen(user_data);
}

static void destroy_active_content(AppState *state)
{
    gtk_overlay_set_child(GTK_OVERLAY(state->root_overlay), NULL);

    if (state->single_player != NULL) {
        single_player_free(state->single_player);
        state->single_player = NULL;
    }
    if (state->grid_player != NULL) {
        grid_player_free(state->grid_player);
        state->grid_player = NULL;
    }
}

static void capture_single_handoff(AppState *state)
{
    if (state->single_player == NULL) {
        return;
    }

    g_clear_pointer(&state->single_target, g_free);
    state->single_target = single_player_dup_current_target(state->single_player);
    state->has_single_target_handoff = TRUE;
    state->single_chat_paned_position = single_player_get_chat_paned_position(state->single_player);
}

static void capture_grid_handoff(AppState *state)
{
    if (state->grid_player == NULL) {
        return;
    }

    g_clear_pointer(&state->single_target, g_free);
    state->single_target = grid_player_dup_first_target(state->grid_player);
    state->has_single_target_handoff = TRUE;
}

static void create_single_content(AppState *state)
{
    const char *target = state->has_single_target_handoff ? state->single_target : state->startup_target;

    state->single_player = single_player_new(
        GTK_WINDOW(state->window),
        state->settings,
        target,
        target != NULL && target[0] != '\0',
        state->single_chat_paned_position,
        on_content_fullscreen_requested,
        state
    );
    single_player_set_fullscreen(state->single_player, state->fullscreen);
    gtk_overlay_set_child(GTK_OVERLAY(state->root_overlay), single_player_get_widget(state->single_player));
    state->content_mode = CONTENT_MODE_SINGLE;
    gtk_widget_set_tooltip_text(state->layout_button, "Switch to grid player");
    gtk_button_set_child(GTK_BUTTON(state->layout_button), create_layout_icon(LAYOUT_ICON_GRID));
}

static void create_grid_content(AppState *state)
{
    const char *targets[GRID_PLAYER_MAX_TILES] = {0};
    guint target_count = 0;

    if (state->single_target != NULL && state->single_target[0] != '\0') {
        targets[target_count++] = state->single_target;
    }

    for (guint i = 0; i < state->grid_target_count && target_count < GRID_PLAYER_MAX_TILES; i++) {
        const char *target = state->grid_targets[i];
        if (target == NULL || target[0] == '\0') {
            continue;
        }
        if (state->single_target != NULL && g_strcmp0(target, state->single_target) == 0) {
            continue;
        }

        targets[target_count++] = target;
    }

    state->grid_player = grid_player_new(
        GTK_WINDOW(state->window),
        state->settings,
        targets,
        target_count
    );
    grid_player_set_fullscreen(state->grid_player, state->fullscreen);
    gtk_overlay_set_child(GTK_OVERLAY(state->root_overlay), grid_player_get_widget(state->grid_player));
    grid_player_start(state->grid_player);
    state->content_mode = CONTENT_MODE_GRID;
    gtk_widget_set_tooltip_text(state->layout_button, "Switch to single player");
    gtk_button_set_child(GTK_BUTTON(state->layout_button), create_layout_icon(LAYOUT_ICON_SINGLE));
}

static void set_layout_mode(AppState *state, ContentMode mode)
{
    if (state->single_player == NULL && state->grid_player == NULL) {
        if (mode == CONTENT_MODE_GRID) {
            create_grid_content(state);
        } else {
            create_single_content(state);
        }
        show_window_overlay(state);
        return;
    }

    if (state->content_mode == mode) {
        show_window_overlay(state);
        return;
    }

    if (mode == CONTENT_MODE_GRID) {
        capture_single_handoff(state);
    } else {
        capture_grid_handoff(state);
    }

    destroy_active_content(state);
    if (mode == CONTENT_MODE_GRID) {
        create_grid_content(state);
    } else {
        create_single_content(state);
    }
    show_window_overlay(state);
}

static void on_layout_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    AppState *state = user_data;
    set_layout_mode(state, state->content_mode == CONTENT_MODE_SINGLE ? CONTENT_MODE_GRID : CONTENT_MODE_SINGLE);
}

static void on_settings_saved(AppSettings *settings, gpointer user_data)
{
    (void)settings;
    AppState *state = user_data;

    if (state->single_player != NULL) {
        single_player_set_settings(state->single_player, state->settings);
    }
    if (state->grid_player != NULL) {
        grid_player_set_settings(state->grid_player, state->settings);
    }

    show_window_overlay(state);
}

static void on_settings_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    AppState *state = user_data;
    settings_window_show(GTK_WINDOW(state->window), state->settings, on_settings_saved, state);
    show_window_overlay(state);
}

static void on_minimize_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    AppState *state = user_data;
    gtk_window_minimize(GTK_WINDOW(state->window));
}

static void on_fullscreen_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    toggle_fullscreen(user_data);
}

static void on_close_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    AppState *state = user_data;
    gtk_window_close(GTK_WINDOW(state->window));
}

static void on_root_motion(GtkEventControllerMotion *controller, double x, double y, gpointer user_data)
{
    (void)controller;
    (void)x;
    (void)y;
    show_window_overlay(user_data);
}

static gboolean on_key_pressed(GtkEventControllerKey *controller, guint keyval, guint keycode, GdkModifierType modifiers, gpointer user_data)
{
    (void)controller;
    (void)keycode;
    AppState *state = user_data;

    if (state->single_player != NULL) {
        return single_player_handle_key(state->single_player, keyval, modifiers);
    }

    return GDK_EVENT_PROPAGATE;
}

static void install_css(void)
{
    GtkCssProvider *provider = gtk_css_provider_new();

    gtk_css_provider_load_from_string(
        provider,
        ".video-footer {"
        "  background: rgba(0, 0, 0, 0.58);"
        "  padding: 8px;"
        "  border-radius: 0;"
        "}"
        ".main-area {"
        "  background: #0e0e10;"
        "}"
        "paned.main-area > separator,"
        "paned.main-area > separator.wide,"
        ".main-area separator,"
        ".main-area separator.wide,"
        ".main-area > separator,"
        ".main-area > separator.wide {"
        "  background: transparent;"
        "  background-image: none;"
        "  border: none;"
        "  outline: none;"
        "  box-shadow: none;"
        "  color: transparent;"
        "  margin: 0;"
        "  padding: 0;"
        "  min-width: 1px;"
        "}"
        "paned.main-area > separator:hover,"
        "paned.main-area > separator.wide:hover,"
        ".main-area separator:hover,"
        ".main-area separator.wide:hover {"
        "  background: transparent;"
        "  background-image: none;"
        "  border: none;"
        "  outline: none;"
        "  box-shadow: none;"
        "}"
        ".chat-panel,"
        ".chat-scroll,"
        ".chat-scroll viewport,"
        ".chat-view,"
        ".chat-view text {"
        "  background: #0e0e10;"
        "  color: #efeff1;"
        "}"
        ".chat-view {"
        "  caret-color: transparent;"
        "  font-size: 14px;"
        "}"
        ".chat-emote {"
        "  background: transparent;"
        "}"
        ".chat-view text selection {"
        "  background: rgba(145, 70, 255, 0.35);"
        "  color: #ffffff;"
        "}"
        ".chat-scroll scrollbar {"
        "  background: transparent;"
        "}"
        ".chat-scroll scrollbar slider {"
        "  background: rgba(255, 255, 255, 0.28);"
        "  border-radius: 999px;"
        "  min-width: 4px;"
        "}"
        ".top-overlay-controls {"
        "  margin: 6px;"
        "}"
        ".overlay-icon-button {"
        "  background: rgba(0, 0, 0, 0.58);"
        "  color: white;"
        "  border-color: transparent;"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "  min-width: 30px;"
        "  min-height: 28px;"
        "  padding: 3px 7px;"
        "}"
        ".overlay-icon-button:hover {"
        "  background: rgba(54, 54, 54, 0.90);"
        "}"
        ".settings-overlay-button {"
        "  background: rgba(0, 0, 0, 0.30);"
        "}"
        ".settings-overlay-button:hover {"
        "  background: rgba(38, 38, 38, 0.62);"
        "}"
        ".close-button:hover {"
        "  background: rgba(170, 36, 36, 0.90);"
        "}"
        ".video-footer button,"
        ".video-footer menubutton,"
        ".video-footer menubutton > button,"
        ".video-footer popover,"
        ".video-footer scale {"
        "  color: white;"
        "}"
        ".video-footer button,"
        ".video-footer menubutton > button {"
        "  background: rgba(30, 30, 30, 0.82);"
        "  color: white;"
        "  border-color: transparent;"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "}"
        ".video-footer button:hover,"
        ".video-footer menubutton > button:hover {"
        "  background: rgba(54, 54, 54, 0.90);"
        "}"
        ".stream-dropdown {"
        "  min-width: 170px;"
        "}"
        ".stream-dropdown > button {"
        "  padding-left: 10px;"
        "  padding-right: 8px;"
        "}"
        ".stream-button-label {"
        "  color: white;"
        "}"
        ".stream-title-label {"
        "  color: rgba(255, 255, 255, 0.92);"
        "  font-size: 13px;"
        "  margin-left: 4px;"
        "  margin-right: 12px;"
        "}"
        ".stream-popover contents {"
        "  background: rgba(28, 28, 28, 0.98);"
        "  padding: 0;"
        "  margin: 0;"
        "  border: none;"
        "  border-radius: 4px;"
        "  box-shadow: none;"
        "}"
        ".stream-popover {"
        "  padding: 0;"
        "  margin: 0;"
        "  border: none;"
        "  border-radius: 4px;"
        "  box-shadow: none;"
        "}"
        ".stream-menu {"
        "  background: rgba(28, 28, 28, 0.98);"
        "  padding: 2px 0;"
        "  margin: 0;"
        "}"
        ".stream-menu-item {"
        "  background: transparent;"
        "  color: white;"
        "  border-color: transparent;"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "  border-radius: 0;"
        "  margin: 0;"
        "  min-height: 0;"
        "  padding: 6px 10px;"
        "}"
        ".stream-menu-item box {"
        "  padding: 0;"
        "  margin: 0;"
        "}"
        ".stream-menu-item label {"
        "  color: white;"
        "  padding: 0;"
        "  margin: 0;"
        "}"
        ".stream-menu-item:hover {"
        "  background: rgba(74, 74, 74, 0.98);"
        "  color: white;"
        "}"
        ".video-footer scale trough {"
        "  background: rgba(255, 255, 255, 0.20);"
        "}"
        ".video-footer scale highlight {"
        "  background: rgba(255, 255, 255, 0.72);"
        "}"
        ".video-footer scale slider {"
        "  background: rgba(255, 255, 255, 0.95);"
        "  border-color: rgba(0, 0, 0, 0.45);"
        "}"
        ".settings-window {"
        "  background: #141417;"
        "  color: #efeff1;"
        "}"
        ".settings-sidebar {"
        "  background: #1f1f23;"
        "  border-right: 1px solid rgba(255, 255, 255, 0.10);"
        "  padding: 8px;"
        "}"
        ".settings-sidebar row {"
        "  border-radius: 6px;"
        "  padding: 10px 12px;"
        "}"
        ".settings-sidebar row:selected {"
        "  background: #2f2f35;"
        "}"
        ".settings-sidebar-label,"
        ".settings-page-title,"
        ".settings-channel-header label,"
        ".settings-empty-label,"
        ".settings-status-label {"
        "  color: #efeff1;"
        "}"
        ".settings-page {"
        "  padding: 18px;"
        "  background: #141417;"
        "}"
        ".settings-page-title {"
        "  font-size: 20px;"
        "  font-weight: 700;"
        "}"
        ".settings-channel-header label {"
        "  color: rgba(239, 239, 241, 0.70);"
        "  font-size: 12px;"
        "  font-weight: 700;"
        "}"
        ".settings-channel-row entry {"
        "  background: #222226;"
        "  color: #ffffff;"
        "  border-color: rgba(255, 255, 255, 0.12);"
        "  font-size: 13px;"
        "  min-height: 28px;"
        "  padding: 3px 8px;"
        "}"
        ".settings-channel-row entry selection {"
        "  background: rgba(145, 70, 255, 0.55);"
        "  color: #ffffff;"
        "}"
        ".settings-page button {"
        "  background: #26262c;"
        "  color: #efeff1;"
        "  border-color: rgba(255, 255, 255, 0.12);"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "}"
        ".settings-page button:hover {"
        "  background: #34343b;"
        "}"
        ".settings-page .settings-primary-button {"
        "  background: #3a2b52;"
        "  color: #ffffff;"
        "}"
        ".settings-page .settings-primary-button:hover {"
        "  background: #4b3670;"
        "}"
        ".settings-page .settings-remove-button {"
        "  background: transparent;"
        "  border-color: transparent;"
        "  min-width: 24px;"
        "  min-height: 24px;"
        "  padding: 2px 4px;"
        "}"
        ".settings-page .settings-remove-button:hover {"
        "  background: rgba(255, 255, 255, 0.08);"
        "}"
        ".settings-empty-label {"
        "  color: rgba(239, 239, 241, 0.62);"
        "  margin-top: 8px;"
        "  margin-bottom: 8px;"
        "}"
        ".settings-status-label {"
        "  color: #ffb4ab;"
        "}"
    );

    gtk_style_context_add_provider_for_display(
        gdk_display_get_default(),
        GTK_STYLE_PROVIDER(provider),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    g_object_unref(provider);
}

static void install_app_icon(void)
{
    GtkIconTheme *theme = gtk_icon_theme_get_for_display(gdk_display_get_default());
    gtk_icon_theme_add_resource_path(theme, APP_ICON_RESOURCE_THEME_PATH);
    gtk_window_set_default_icon_name(APP_ID);
}

static char *get_executable_path(const char *argv0)
{
    g_autofree char *link_path = g_file_read_link("/proc/self/exe", NULL);
    if (link_path != NULL) {
        return g_steal_pointer(&link_path);
    }

    if (g_path_is_absolute(argv0)) {
        return g_strdup(argv0);
    }

    g_autofree char *cwd = g_get_current_dir();
    return g_canonicalize_filename(argv0, cwd);
}

static char *quote_desktop_exec_path(const char *path)
{
    GString *quoted = g_string_new("\"");

    for (const char *p = path; *p != '\0'; p++) {
        if (*p == '"' || *p == '\\' || *p == '`' || *p == '$') {
            g_string_append_c(quoted, '\\');
        }
        g_string_append_c(quoted, *p);
    }

    g_string_append_c(quoted, '"');
    return g_string_free(quoted, FALSE);
}

static void write_user_desktop_identity(const char *argv0)
{
    g_autofree char *applications_dir = g_build_filename(g_get_user_data_dir(), "applications", NULL);
    g_autofree char *icons_dir = g_build_filename(g_get_user_data_dir(), "icons", "hicolor", "scalable", "apps", NULL);
    g_autofree char *desktop_path = g_build_filename(applications_dir, APP_ID ".desktop", NULL);
    g_autofree char *icon_path = g_build_filename(icons_dir, APP_ID ".svg", NULL);
    g_autofree char *exec_path = get_executable_path(argv0);
    g_autofree char *quoted_exec = quote_desktop_exec_path(exec_path);
    g_autoptr(GError) error = NULL;

    if (g_mkdir_with_parents(applications_dir, 0700) < 0 || g_mkdir_with_parents(icons_dir, 0700) < 0) {
        g_debug("could not create user desktop/icon directories");
        return;
    }

    g_autoptr(GBytes) icon_data = g_resources_lookup_data(APP_ICON_RESOURCE_PATH, G_RESOURCE_LOOKUP_FLAGS_NONE, &error);
    if (icon_data == NULL) {
        g_debug("could not load embedded app icon: %s", error != NULL ? error->message : "unknown error");
        return;
    }

    gsize icon_size = 0;
    const char *icon_bytes = g_bytes_get_data(icon_data, &icon_size);
    if (!g_file_set_contents(icon_path, icon_bytes, icon_size, &error)) {
        g_debug("could not write user app icon: %s", error->message);
        return;
    }

    g_autofree char *desktop = g_strdup_printf(
        "[Desktop Entry]\n"
        "Type=Application\n"
        "Name=Twitch Player\n"
        "Exec=%s %%u\n"
        "Icon=%s\n"
        "Terminal=false\n"
        "Categories=AudioVideo;Player;Network;\n"
        "StartupNotify=true\n"
        "StartupWMClass=%s\n",
        quoted_exec,
        icon_path,
        APP_ID
    );

    if (!g_file_set_contents(desktop_path, desktop, -1, &error)) {
        g_debug("could not write desktop entry: %s", error->message);
    }
}


static void destroy_state(gpointer user_data)
{
    AppState *state = user_data;
    state->closing = TRUE;

    if (state->overlay_hide_source != 0) {
        g_source_remove(state->overlay_hide_source);
        state->overlay_hide_source = 0;
    }

    destroy_active_content(state);
    g_clear_pointer(&state->single_target, g_free);
    app_settings_free(state->settings);
    state->settings = NULL;
}

static void on_activate(GtkApplication *application, gpointer user_data)
{
    StartupConfig *config = user_data;

    install_css();
    install_app_icon();

    AppState *state = g_new0(AppState, 1);
    state->application = application;
    state->startup_target = config != NULL ? config->startup_target : NULL;
    state->grid_targets = config != NULL ? config->grid_targets : NULL;
    state->grid_target_count = config != NULL ? config->grid_target_count : 0;
    state->settings = app_settings_load();

    state->window = gtk_application_window_new(application);
    gtk_window_set_title(GTK_WINDOW(state->window), "Twitch Player");
    gtk_window_set_default_size(GTK_WINDOW(state->window), 1100, 680);
    gtk_window_set_decorated(GTK_WINDOW(state->window), FALSE);
    gtk_window_set_icon_name(GTK_WINDOW(state->window), APP_ID);

    state->root_overlay = gtk_overlay_new();
    gtk_window_set_child(GTK_WINDOW(state->window), state->root_overlay);
    add_resize_handles(GTK_OVERLAY(state->root_overlay), state);

    state->top_left_controls = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    gtk_widget_add_css_class(state->top_left_controls, "top-overlay-controls");
    gtk_widget_set_halign(state->top_left_controls, GTK_ALIGN_START);
    gtk_widget_set_valign(state->top_left_controls, GTK_ALIGN_START);
    gtk_overlay_add_overlay(GTK_OVERLAY(state->root_overlay), state->top_left_controls);

    state->settings_button = create_overlay_button(create_settings_icon(), "Settings");
    gtk_widget_add_css_class(state->settings_button, "settings-overlay-button");
    gtk_box_append(GTK_BOX(state->top_left_controls), state->settings_button);
    g_signal_connect(state->settings_button, "clicked", G_CALLBACK(on_settings_clicked), state);

    state->layout_button = create_overlay_button(create_layout_icon(LAYOUT_ICON_GRID), "Switch to grid player");
    gtk_widget_add_css_class(state->layout_button, "settings-overlay-button");
    gtk_box_append(GTK_BOX(state->top_left_controls), state->layout_button);
    g_signal_connect(state->layout_button, "clicked", G_CALLBACK(on_layout_clicked), state);

    state->top_controls = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    gtk_widget_add_css_class(state->top_controls, "top-overlay-controls");
    gtk_widget_set_halign(state->top_controls, GTK_ALIGN_END);
    gtk_widget_set_valign(state->top_controls, GTK_ALIGN_START);
    gtk_overlay_add_overlay(GTK_OVERLAY(state->root_overlay), state->top_controls);

    GtkWidget *minimize_button = create_overlay_button(create_window_icon(WINDOW_ICON_MINIMIZE), "Minimize");
    gtk_box_append(GTK_BOX(state->top_controls), minimize_button);
    g_signal_connect(minimize_button, "clicked", G_CALLBACK(on_minimize_clicked), state);

    GtkWidget *fullscreen_button = create_overlay_button(create_window_icon(WINDOW_ICON_FULLSCREEN), "Fullscreen");
    gtk_box_append(GTK_BOX(state->top_controls), fullscreen_button);
    g_signal_connect(fullscreen_button, "clicked", G_CALLBACK(on_fullscreen_clicked), state);

    GtkWidget *close_button = create_overlay_button(create_window_icon(WINDOW_ICON_CLOSE), "Close");
    gtk_widget_add_css_class(close_button, "close-button");
    gtk_box_append(GTK_BOX(state->top_controls), close_button);
    g_signal_connect(close_button, "clicked", G_CALLBACK(on_close_clicked), state);

    GtkEventController *motion = gtk_event_controller_motion_new();
    gtk_event_controller_set_propagation_phase(motion, GTK_PHASE_CAPTURE);
    g_signal_connect(motion, "motion", G_CALLBACK(on_root_motion), state);
    gtk_widget_add_controller(state->root_overlay, motion);

    GtkEventController *key_controller = gtk_event_controller_key_new();
    gtk_event_controller_set_propagation_phase(key_controller, GTK_PHASE_CAPTURE);
    g_signal_connect(key_controller, "key-pressed", G_CALLBACK(on_key_pressed), state);
    gtk_widget_add_controller(state->window, key_controller);

    g_object_set_data_full(G_OBJECT(state->window), "app-state", state, destroy_state);

    set_layout_mode(state, config != NULL && config->start_in_grid ? CONTENT_MODE_GRID : CONTENT_MODE_SINGLE);
    gtk_window_present(GTK_WINDOW(state->window));
    schedule_window_overlay_hide(state);
}

int main(int argc, char **argv)
{
    const char *grid_targets[GRID_PLAYER_MAX_TILES] = {0};
    guint grid_target_count = 0;
    gboolean start_in_grid = FALSE;
    const char *startup_target = NULL;

    for (int i = 1; i < argc; i++) {
        if (g_strcmp0(argv[i], "--grid") == 0) {
            start_in_grid = TRUE;
            continue;
        }

        if (startup_target == NULL) {
            startup_target = argv[i];
        }
        if (grid_target_count < GRID_PLAYER_MAX_TILES) {
            grid_targets[grid_target_count++] = argv[i];
        }
    }

    StartupConfig config = {
        .startup_target = startup_target,
        .grid_targets = grid_targets,
        .grid_target_count = grid_target_count,
        .start_in_grid = start_in_grid,
    };

    g_set_prgname(APP_ID);
    g_set_application_name("Twitch Player");
    write_user_desktop_identity(argv[0]);

    g_autoptr(GtkApplication) application = gtk_application_new(
        APP_ID,
        G_APPLICATION_NON_UNIQUE
    );

    g_signal_connect(application, "activate", G_CALLBACK(on_activate), &config);

    return g_application_run(G_APPLICATION(application), 1, argv);
}
