#define G_LOG_DOMAIN "twitch-player"

#include <gtk/gtk.h>
#include <epoxy/egl.h>
#include <epoxy/gl.h>
#include <mpv/client.h>
#include <mpv/render_gl.h>
#include <locale.h>
#include <math.h>

#include "chat_panel.h"

#define APP_ID "local.twitchplayer"
#define APP_ICON_RESOURCE_PATH "/local/twitch-player/icons/hicolor/scalable/apps/local.twitch-player.svg"
#define APP_ICON_RESOURCE_THEME_PATH "/local/twitch-player/icons"

typedef struct {
    const char *label;
    const char *channel;
    const char *url;
} StreamEntry;

static const StreamEntry STREAMS[] = {
    {"MontanaBlack88", "montanablack88", "https://www.twitch.tv/montanablack88"},
    {"Papaplatte", "papaplatte", "https://www.twitch.tv/papaplatte"},
    {"Rumathra", "rumathra", "https://www.twitch.tv/rumathra"},
};

typedef struct {
    GtkApplication *application;
    const char *startup_target;
    GtkWidget *window;
    GtkWidget *video_overlay;
    GtkWidget *gl_area;
    GtkWidget *main_area;
    GtkWidget *top_controls;
    GtkWidget *chat_toggle_button;
    GtkWidget *bottom_panel;
    GtkWidget *footer_spacer;
    GtkWidget *stream_combo;
    GtkWidget *volume_scale;
    GtkWidget *status_label;
    StreamEntry *streams;
    guint stream_count;
    mpv_handle *mpv;
    mpv_render_context *mpv_gl;
    ChatPanel *chat_panel;
    int chat_width;
    int chat_paned_position;
    guint active_stream;
    gboolean chat_visible;
    guint footer_hide_source;
    double last_motion_x;
    double last_motion_y;
    double move_press_x;
    double move_press_y;
    gboolean has_last_motion;
    gboolean move_pressed;
    gboolean closing;
    gboolean fullscreen;
} AppState;

typedef struct {
    const char *startup_target;
} StartupConfig;

typedef enum {
    CHAT_ICON_OPEN,
    CHAT_ICON_CLOSE,
} ChatIconKind;

typedef enum {
    WINDOW_ICON_MINIMIZE,
    WINDOW_ICON_FULLSCREEN,
    WINDOW_ICON_CLOSE,
} WindowIconKind;

static void set_status(AppState *state, const char *message)
{
    if (state->status_label != NULL) {
        gtk_label_set_text(GTK_LABEL(state->status_label), message);
    }
}

static void start_chat(AppState *state, const char *channel)
{
    if (channel == NULL || channel[0] == '\0') {
        return;
    }

    chat_panel_start(state->chat_panel, channel);
}

static void check_mpv(int status, const char *action)
{
    if (status < 0) {
        g_warning("%s: %s", action, mpv_error_string(status));
    }
}

static void *get_proc_address(void *ctx, const char *name)
{
    (void)ctx;
    return (void *)eglGetProcAddress(name);
}

static gboolean queue_mpv_render(gpointer user_data)
{
    AppState *state = user_data;

    if (!state->closing && state->gl_area != NULL) {
        gtk_gl_area_queue_render(GTK_GL_AREA(state->gl_area));
    }

    return G_SOURCE_REMOVE;
}

static void on_mpv_render_update(void *ctx)
{
    g_idle_add(queue_mpv_render, ctx);
}

static gboolean process_mpv_events(gpointer user_data)
{
    AppState *state = user_data;

    if (state->closing || state->mpv == NULL) {
        return G_SOURCE_REMOVE;
    }

    while (true) {
        mpv_event *event = mpv_wait_event(state->mpv, 0);

        if (event->event_id == MPV_EVENT_NONE) {
            break;
        }

        switch (event->event_id) {
        case MPV_EVENT_START_FILE:
            set_status(state, "Stream wird geladen");
            break;
        case MPV_EVENT_FILE_LOADED:
            set_status(state, "Wiedergabe laeuft");
            break;
        case MPV_EVENT_END_FILE: {
            mpv_event_end_file *end = event->data;
            if (end != NULL && end->reason == MPV_END_FILE_REASON_ERROR) {
                set_status(state, "Stream konnte nicht abgespielt werden");
            } else {
                set_status(state, "Gestoppt");
            }
            break;
        }
        case MPV_EVENT_LOG_MESSAGE: {
            mpv_event_log_message *log = event->data;
            if (log != NULL && log->prefix != NULL && log->text != NULL) {
                g_debug("mpv[%s]: %s", log->prefix, log->text);
            }
            break;
        }
        case MPV_EVENT_SHUTDOWN:
            return G_SOURCE_REMOVE;
        default:
            break;
        }
    }

    return G_SOURCE_REMOVE;
}

static void on_mpv_wakeup(void *ctx)
{
    g_idle_add(process_mpv_events, ctx);
}

static void load_stream_url(AppState *state, const char *url, const char *label, const char *channel)
{
    (void)label;

    const char *cmd[] = {
        "loadfile",
        url,
        "replace",
        NULL,
    };

    set_status(state, "Stream wird gestartet");
    start_chat(state, channel);
    check_mpv(mpv_command_async(state->mpv, 0, cmd), "loadfile");
}

static void play_selected_stream(AppState *state)
{
    if (state->mpv == NULL) {
        g_warning("play requested, but mpv is not available");
        return;
    }

    guint active = state->active_stream;

    if (active >= state->stream_count) {
        set_status(state, "Kein Stream ausgewaehlt");
        return;
    }

    load_stream_url(state, state->streams[active].url, state->streams[active].label, state->streams[active].channel);
}

static void on_volume_changed(GtkRange *range, gpointer user_data)
{
    AppState *state = user_data;
    double volume = gtk_range_get_value(range);

    check_mpv(mpv_set_property(state->mpv, "volume", MPV_FORMAT_DOUBLE, &volume), "set volume");
}

static void draw_chat_icon(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)area;
    ChatIconKind kind = GPOINTER_TO_INT(user_data);
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
    if (kind == CHAT_ICON_OPEN) {
        cairo_move_to(cr, badge_x, badge_y - badge_r * 0.48);
        cairo_line_to(cr, badge_x, badge_y + badge_r * 0.48);
    }
    cairo_stroke(cr);
}

static GtkWidget *create_chat_icon(ChatIconKind kind)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_chat_icon, GINT_TO_POINTER(kind), NULL);
    return icon;
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

static GtkWidget *create_info_icon(void)
{
    GtkWidget *icon = gtk_drawing_area_new();
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(icon), 18);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(icon), draw_info_icon, NULL, NULL);
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

static gboolean hide_footer(gpointer user_data)
{
    AppState *state = user_data;

    state->footer_hide_source = 0;

    if (!state->closing) {
        gtk_widget_set_visible(state->bottom_panel, FALSE);
        gtk_widget_set_visible(state->top_controls, FALSE);
        gtk_widget_set_visible(state->chat_toggle_button, FALSE);
    }

    return G_SOURCE_REMOVE;
}

static void schedule_footer_hide(AppState *state)
{
    if (state->footer_hide_source != 0) {
        g_source_remove(state->footer_hide_source);
    }

    state->footer_hide_source = g_timeout_add(1800, hide_footer, state);
}

static void show_footer(AppState *state)
{
    if (state->closing) {
        return;
    }

    gtk_widget_set_visible(state->bottom_panel, TRUE);
    gtk_widget_set_visible(state->top_controls, TRUE);
    gtk_widget_set_visible(state->chat_toggle_button, TRUE);
    schedule_footer_hide(state);
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

static gboolean get_toplevel_event_data_from_event(GtkWidget *window, GdkEvent *event, GdkToplevel **toplevel, GdkDevice **device, double *x, double *y, guint32 *timestamp)
{
    GtkNative *native = gtk_widget_get_native(window);
    GdkSurface *surface = native != NULL ? gtk_native_get_surface(native) : NULL;

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

static void begin_window_move_from_event(AppState *state, GdkEvent *event, guint button)
{
    GdkToplevel *toplevel = NULL;
    GdkDevice *device = NULL;
    double x = 0;
    double y = 0;
    guint32 timestamp = 0;

    if (get_toplevel_event_data_from_event(state->window, event, &toplevel, &device, &x, &y, &timestamp)) {
        gdk_toplevel_begin_move(toplevel, device, button, x, y, timestamp);
    }
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

static void on_video_motion(GtkEventControllerMotion *controller, double x, double y, gpointer user_data)
{
    (void)controller;
    AppState *state = user_data;

    if (state->has_last_motion &&
        fabs(x - state->last_motion_x) < 0.5 &&
        fabs(y - state->last_motion_y) < 0.5) {
        return;
    }

    state->last_motion_x = x;
    state->last_motion_y = y;
    state->has_last_motion = TRUE;

    show_footer(state);
}

static void toggle_fullscreen(AppState *state)
{
    if (state->fullscreen) {
        gtk_window_unfullscreen(GTK_WINDOW(state->window));
        show_footer(state);
        state->fullscreen = FALSE;
    } else {
        gtk_window_fullscreen(GTK_WINDOW(state->window));
        show_footer(state);
        state->fullscreen = TRUE;
    }
}

static void on_video_pressed(GtkGestureClick *gesture, int n_press, double x, double y, gpointer user_data)
{
    (void)gesture;
    (void)x;
    (void)y;

    if (n_press == 2) {
        toggle_fullscreen(user_data);
    }
}

static gboolean on_video_legacy_event(GtkEventControllerLegacy *controller, GdkEvent *event, gpointer user_data)
{
    (void)controller;
    AppState *state = user_data;
    GdkEventType type = gdk_event_get_event_type(event);

    if (state->fullscreen) {
        return GDK_EVENT_PROPAGATE;
    }

    if (type == GDK_BUTTON_PRESS && gdk_button_event_get_button(event) == GDK_BUTTON_PRIMARY) {
        state->move_pressed = gdk_event_get_position(event, &state->move_press_x, &state->move_press_y);
        return GDK_EVENT_PROPAGATE;
    }

    if (type == GDK_BUTTON_RELEASE && gdk_button_event_get_button(event) == GDK_BUTTON_PRIMARY) {
        state->move_pressed = FALSE;
        return GDK_EVENT_PROPAGATE;
    }

    if (type != GDK_MOTION_NOTIFY || !state->move_pressed) {
        return GDK_EVENT_PROPAGATE;
    }

    if ((gdk_event_get_modifier_state(event) & GDK_BUTTON1_MASK) == 0) {
        state->move_pressed = FALSE;
        return GDK_EVENT_PROPAGATE;
    }

    double x = 0;
    double y = 0;
    if (!gdk_event_get_position(event, &x, &y)) {
        return GDK_EVENT_PROPAGATE;
    }

    if (fabs(x - state->move_press_x) < 4.0 && fabs(y - state->move_press_y) < 4.0) {
        return GDK_EVENT_PROPAGATE;
    }

    state->move_pressed = FALSE;
    begin_window_move_from_event(state, event, GDK_BUTTON_PRIMARY);
    return GDK_EVENT_STOP;
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

static void set_chat_visible(AppState *state, gboolean visible)
{
    state->chat_visible = visible;

    if (visible) {
        gtk_paned_set_end_child(GTK_PANED(state->main_area), state->chat_panel->widget);
        gtk_paned_set_position(GTK_PANED(state->main_area), state->chat_paned_position);
    } else {
        state->chat_paned_position = gtk_paned_get_position(GTK_PANED(state->main_area));
        g_object_ref(state->chat_panel->widget);
        gtk_paned_set_end_child(GTK_PANED(state->main_area), NULL);
    }

    gtk_widget_set_tooltip_text(state->chat_toggle_button, visible ? "Chat schliessen" : "Chat oeffnen");
    gtk_button_set_child(
        GTK_BUTTON(state->chat_toggle_button),
        create_chat_icon(visible ? CHAT_ICON_CLOSE : CHAT_ICON_OPEN)
    );
}

static void on_chat_toggle_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    AppState *state = user_data;
    set_chat_visible(state, !state->chat_visible);
    show_footer(state);
}

static void on_stream_info_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    AppState *state = user_data;

    if (state->mpv == NULL) {
        return;
    }

    const char *stats_cmd[] = {
        "script-binding",
        "stats/display-stats-toggle",
        NULL,
    };

    int status = mpv_command(state->mpv, stats_cmd);
    if (status < 0) {
        const char *keypress_cmd[] = {
            "keypress",
            "i",
            NULL,
        };
        check_mpv(mpv_command(state->mpv, keypress_cmd), "toggle stream info");
    }

    show_footer(state);
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

static void on_stream_menu_clicked(GtkButton *button, gpointer user_data)
{
    AppState *state = user_data;
    gpointer index_data = g_object_get_data(G_OBJECT(button), "stream-index");

    state->active_stream = GPOINTER_TO_UINT(index_data);
    GtkWidget *child = gtk_menu_button_get_child(GTK_MENU_BUTTON(state->stream_combo));
    if (GTK_IS_LABEL(child)) {
        gtk_label_set_text(GTK_LABEL(child), state->streams[state->active_stream].label);
    }

    GtkPopover *popover = gtk_menu_button_get_popover(GTK_MENU_BUTTON(state->stream_combo));
    if (popover != NULL) {
        gtk_popover_popdown(popover);
    }

    play_selected_stream(state);
}

static gboolean on_gl_render(GtkGLArea *area, GdkGLContext *context, gpointer user_data)
{
    (void)context;
    AppState *state = user_data;

    if (state->mpv_gl == NULL) {
        return TRUE;
    }

    int scale = gtk_widget_get_scale_factor(GTK_WIDGET(area));
    int width = gtk_widget_get_width(GTK_WIDGET(area)) * scale;
    int height = gtk_widget_get_height(GTK_WIDGET(area)) * scale;

    if (width <= 0 || height <= 0) {
        return TRUE;
    }

    gtk_gl_area_attach_buffers(area);

    GLint current_fbo = 0;
    glGetIntegerv(GL_FRAMEBUFFER_BINDING, &current_fbo);

    mpv_opengl_fbo fbo = {
        .fbo = (int)current_fbo,
        .w = width,
        .h = height,
        .internal_format = 0,
    };
    int flip_y = 1;
    mpv_render_param params[] = {
        {MPV_RENDER_PARAM_OPENGL_FBO, &fbo},
        {MPV_RENDER_PARAM_FLIP_Y, &flip_y},
        {MPV_RENDER_PARAM_INVALID, NULL},
    };

    mpv_render_context_render(state->mpv_gl, params);

    return TRUE;
}

static void on_gl_realize(GtkGLArea *area, gpointer user_data)
{
    AppState *state = user_data;

    if (state->mpv == NULL) {
        g_debug("GL realize skipped: mpv is not available");
        return;
    }

    gtk_gl_area_make_current(area);

    if (gtk_gl_area_get_error(area) != NULL) {
        g_warning("GTK GL area error: %s", gtk_gl_area_get_error(area)->message);
        set_status(state, "OpenGL konnte nicht initialisiert werden");
        return;
    }

    mpv_opengl_init_params gl_init_params = {
        .get_proc_address = get_proc_address,
        .get_proc_address_ctx = NULL,
    };
    mpv_render_param params[] = {
        {MPV_RENDER_PARAM_API_TYPE, (void *)MPV_RENDER_API_TYPE_OPENGL},
        {MPV_RENDER_PARAM_OPENGL_INIT_PARAMS, &gl_init_params},
        {MPV_RENDER_PARAM_INVALID, NULL},
    };

    int status = mpv_render_context_create(&state->mpv_gl, state->mpv, params);
    if (status < 0) {
        g_warning("mpv render context: %s", mpv_error_string(status));
        set_status(state, "mpv-Rendering konnte nicht gestartet werden");
        return;
    }

    mpv_render_context_set_update_callback(state->mpv_gl, on_mpv_render_update, state);
}

static void on_gl_unrealize(GtkGLArea *area, gpointer user_data)
{
    AppState *state = user_data;

    gtk_gl_area_make_current(area);

    if (state->mpv_gl != NULL) {
        mpv_render_context_free(state->mpv_gl);
        state->mpv_gl = NULL;
    }
}

static GtkWidget *create_controls(AppState *state)
{
    GtkWidget *box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_add_css_class(box, "video-footer");

    GtkWidget *stream_menu = gtk_box_new(GTK_ORIENTATION_VERTICAL, 2);
    for (guint i = 0; i < state->stream_count; i++) {
        GtkWidget *item = gtk_button_new();
        GtkWidget *item_label = gtk_label_new(state->streams[i].label);
        gtk_label_set_xalign(GTK_LABEL(item_label), 0.0);
        gtk_widget_set_halign(item_label, GTK_ALIGN_FILL);
        gtk_widget_set_hexpand(item_label, TRUE);
        gtk_button_set_child(GTK_BUTTON(item), item_label);
        gtk_widget_add_css_class(item, "stream-menu-item");
        gtk_widget_set_halign(item, GTK_ALIGN_FILL);
        gtk_widget_set_hexpand(item, TRUE);
        g_object_set_data(G_OBJECT(item), "stream-index", GUINT_TO_POINTER(i));
        g_signal_connect(item, "clicked", G_CALLBACK(on_stream_menu_clicked), state);
        gtk_box_append(GTK_BOX(stream_menu), item);
    }

    GtkWidget *stream_popover = gtk_popover_new();
    gtk_widget_add_css_class(stream_popover, "stream-popover");
    gtk_popover_set_position(GTK_POPOVER(stream_popover), GTK_POS_TOP);
    gtk_popover_set_has_arrow(GTK_POPOVER(stream_popover), FALSE);
    gtk_popover_set_child(GTK_POPOVER(stream_popover), stream_menu);

    state->stream_combo = gtk_menu_button_new();
    gtk_widget_add_css_class(state->stream_combo, "stream-dropdown");
    GtkWidget *stream_label = gtk_label_new(state->streams[state->active_stream].label);
    gtk_widget_add_css_class(stream_label, "stream-button-label");
    gtk_widget_set_halign(stream_label, GTK_ALIGN_START);
    gtk_label_set_xalign(GTK_LABEL(stream_label), 0.0);
    gtk_menu_button_set_child(GTK_MENU_BUTTON(state->stream_combo), stream_label);
    gtk_widget_set_halign(state->stream_combo, GTK_ALIGN_START);
    gtk_menu_button_set_direction(GTK_MENU_BUTTON(state->stream_combo), GTK_ARROW_UP);
    gtk_menu_button_set_always_show_arrow(GTK_MENU_BUTTON(state->stream_combo), FALSE);
    gtk_menu_button_set_popover(GTK_MENU_BUTTON(state->stream_combo), stream_popover);
    gtk_widget_set_size_request(state->stream_combo, 170, -1);
    gtk_widget_set_hexpand(state->stream_combo, FALSE);

    state->volume_scale = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, 0, 130, 1);
    gtk_range_set_value(GTK_RANGE(state->volume_scale), 80);
    gtk_widget_set_size_request(state->volume_scale, 140, -1);
    gtk_scale_set_draw_value(GTK_SCALE(state->volume_scale), FALSE);

    gtk_box_append(GTK_BOX(box), state->stream_combo);
    state->footer_spacer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_set_hexpand(state->footer_spacer, TRUE);
    gtk_box_append(GTK_BOX(box), state->footer_spacer);
    gtk_box_append(GTK_BOX(box), state->volume_scale);

    GtkWidget *stream_info_button = create_overlay_button(create_info_icon(), "Stream-Info anzeigen");
    gtk_box_append(GTK_BOX(box), stream_info_button);

    state->chat_toggle_button = create_overlay_button(create_chat_icon(CHAT_ICON_CLOSE), "Chat schliessen");
    gtk_widget_add_css_class(state->chat_toggle_button, "chat-toggle");
    gtk_box_append(GTK_BOX(box), state->chat_toggle_button);

    g_signal_connect(stream_info_button, "clicked", G_CALLBACK(on_stream_info_clicked), state);
    g_signal_connect(state->chat_toggle_button, "clicked", G_CALLBACK(on_chat_toggle_clicked), state);
    g_signal_connect(state->volume_scale, "value-changed", G_CALLBACK(on_volume_changed), state);

    return box;
}

static gboolean init_mpv(AppState *state)
{
    if (setlocale(LC_NUMERIC, "C") == NULL) {
        g_warning("LC_NUMERIC could not be set to C; libmpv may refuse to start");
    }

    state->mpv = mpv_create();
    if (state->mpv == NULL) {
        g_warning("mpv_create returned NULL");
        set_status(state, "mpv konnte nicht erstellt werden");
        return FALSE;
    }

    check_mpv(mpv_set_option_string(state->mpv, "terminal", "no"), "set terminal");
    check_mpv(mpv_set_option_string(state->mpv, "config", "no"), "set config");
    check_mpv(mpv_set_option_string(state->mpv, "vo", "libmpv"), "set vo");
    check_mpv(mpv_set_option_string(state->mpv, "ytdl", "yes"), "set ytdl");
    check_mpv(mpv_set_option_string(state->mpv, "hwdec", "auto-safe"), "set hwdec");
    check_mpv(mpv_set_option_string(state->mpv, "volume", "80"), "set volume");
    int status = mpv_initialize(state->mpv);
    if (status < 0) {
        g_warning("mpv init: %s", mpv_error_string(status));
        set_status(state, "mpv konnte nicht initialisiert werden");
        return FALSE;
    }

    mpv_set_wakeup_callback(state->mpv, on_mpv_wakeup, state);
    return TRUE;
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
        ".top-overlay-controls {"
        "  margin: 8px;"
        "}"
        ".overlay-icon-button {"
        "  background: rgba(0, 0, 0, 0.58);"
        "  color: white;"
        "  border-color: transparent;"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "  min-width: 34px;"
        "  min-height: 30px;"
        "  padding: 4px 8px;"
        "}"
        ".overlay-icon-button:hover {"
        "  background: rgba(54, 54, 54, 0.90);"
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
        "  min-width: 160px;"
        "}"
        ".stream-dropdown > button {"
        "  padding-left: 10px;"
        "  padding-right: 8px;"
        "}"
        ".stream-button-label {"
        "  color: white;"
        "}"
        ".stream-popover contents,"
        ".stream-popover box,"
        ".stream-menu-item {"
        "  background: rgba(28, 28, 28, 0.98);"
        "  color: white;"
        "  border-color: transparent;"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "  padding-left: 10px;"
        "  padding-right: 10px;"
        "}"
        ".stream-menu-item label {"
        "  color: white;"
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

static char *extract_twitch_channel(const char *target)
{
    const char *start = strstr(target, "twitch.tv/");

    if (start == NULL) {
        return NULL;
    }

    start += strlen("twitch.tv/");

    while (*start == '/') {
        start++;
    }

    const char *end = start;
    while (g_ascii_isalnum(*end) || *end == '_') {
        end++;
    }

    if (end == start) {
        return NULL;
    }

    g_autofree char *channel = g_strndup(start, end - start);
    return g_ascii_strdown(channel, -1);
}

static gboolean target_matches_stream(const char *target, const char *target_channel, const StreamEntry *stream)
{
    if (target == NULL || target[0] == '\0') {
        return FALSE;
    }

    return g_ascii_strcasecmp(target, stream->label) == 0 ||
        g_ascii_strcasecmp(target, stream->channel) == 0 ||
        g_ascii_strcasecmp(target, stream->url) == 0 ||
        (target_channel != NULL && g_ascii_strcasecmp(target_channel, stream->channel) == 0);
}

static void init_streams(AppState *state)
{
    guint base_count = G_N_ELEMENTS(STREAMS);
    guint extra_count = 0;
    guint active_stream = 0;
    gboolean startup_known = FALSE;
    g_autofree char *startup_channel = extract_twitch_channel(state->startup_target);
    char *extra_label = NULL;
    char *extra_channel = NULL;
    char *extra_url = NULL;

    if (state->startup_target != NULL && state->startup_target[0] != '\0') {
        for (guint i = 0; i < base_count; i++) {
            if (target_matches_stream(state->startup_target, startup_channel, &STREAMS[i])) {
                active_stream = i;
                startup_known = TRUE;
                break;
            }
        }
    }

    if (!startup_known && state->startup_target != NULL && state->startup_target[0] != '\0') {
        extra_count = 1;

        if (startup_channel != NULL) {
            extra_label = g_strdup(startup_channel);
            extra_channel = g_strdup(startup_channel);
        } else if (g_str_has_prefix(state->startup_target, "http://") || g_str_has_prefix(state->startup_target, "https://")) {
            extra_label = g_strdup(state->startup_target);
        } else {
            extra_label = g_strdup(state->startup_target);
            extra_channel = g_ascii_strdown(state->startup_target, -1);
        }

        if (g_str_has_prefix(state->startup_target, "http://") || g_str_has_prefix(state->startup_target, "https://")) {
            extra_url = g_strdup(state->startup_target);
        } else {
            extra_url = g_strdup_printf("https://www.twitch.tv/%s", state->startup_target);
        }

        active_stream = base_count;
    }

    state->stream_count = base_count + extra_count;
    state->streams = g_new0(StreamEntry, state->stream_count);

    for (guint i = 0; i < G_N_ELEMENTS(STREAMS); i++) {
        state->streams[i].label = g_strdup(STREAMS[i].label);
        state->streams[i].channel = g_strdup(STREAMS[i].channel);
        state->streams[i].url = g_strdup(STREAMS[i].url);
    }

    if (extra_count == 1) {
        state->streams[base_count].label = extra_label;
        state->streams[base_count].channel = extra_channel;
        state->streams[base_count].url = extra_url;
    }

    state->active_stream = active_stream;
}

static void free_streams(AppState *state)
{
    if (state->streams == NULL) {
        return;
    }

    for (guint i = 0; i < state->stream_count; i++) {
        g_free((char *)state->streams[i].label);
        g_free((char *)state->streams[i].channel);
        g_free((char *)state->streams[i].url);
    }

    g_clear_pointer(&state->streams, g_free);
    state->stream_count = 0;
}

static void maybe_start_initial_stream(AppState *state)
{
    if (state->startup_target == NULL || state->startup_target[0] == '\0') {
        return;
    }

    play_selected_stream(state);
}

static void destroy_state(gpointer user_data)
{
    AppState *state = user_data;

    state->closing = TRUE;

    if (state->footer_hide_source != 0) {
        g_source_remove(state->footer_hide_source);
        state->footer_hide_source = 0;
    }

    if (state->mpv_gl != NULL) {
        mpv_render_context_free(state->mpv_gl);
        state->mpv_gl = NULL;
    }

    if (state->mpv != NULL) {
        mpv_terminate_destroy(state->mpv);
        state->mpv = NULL;
    }

    chat_panel_free(state->chat_panel);
    state->chat_panel = NULL;
    free_streams(state);
}

static void on_activate(GtkApplication *application, gpointer user_data)
{
    StartupConfig *config = user_data;

    install_css();
    install_app_icon();

    AppState *state = g_new0(AppState, 1);
    state->application = application;
    state->startup_target = config != NULL ? config->startup_target : NULL;

    state->window = gtk_application_window_new(application);
    gtk_window_set_title(GTK_WINDOW(state->window), "Twitch Player");
    gtk_window_set_default_size(GTK_WINDOW(state->window), 1100, 680);
    gtk_window_set_decorated(GTK_WINDOW(state->window), FALSE);
    gtk_window_set_icon_name(GTK_WINDOW(state->window), APP_ID);

    GtkWidget *root = gtk_overlay_new();
    gtk_window_set_child(GTK_WINDOW(state->window), root);

    state->chat_width = 360;
    state->chat_paned_position = 740;
    state->active_stream = 0;
    state->chat_visible = TRUE;
    init_streams(state);

    state->main_area = gtk_paned_new(GTK_ORIENTATION_HORIZONTAL);
    gtk_widget_set_hexpand(state->main_area, TRUE);
    gtk_widget_set_vexpand(state->main_area, TRUE);
    gtk_paned_set_wide_handle(GTK_PANED(state->main_area), TRUE);
    gtk_paned_set_resize_start_child(GTK_PANED(state->main_area), TRUE);
    gtk_paned_set_shrink_start_child(GTK_PANED(state->main_area), FALSE);
    gtk_paned_set_resize_end_child(GTK_PANED(state->main_area), FALSE);
    gtk_paned_set_shrink_end_child(GTK_PANED(state->main_area), FALSE);
    gtk_overlay_set_child(GTK_OVERLAY(root), state->main_area);
    add_resize_handles(GTK_OVERLAY(root), state);

    state->gl_area = gtk_gl_area_new();
    gtk_gl_area_set_auto_render(GTK_GL_AREA(state->gl_area), FALSE);
    gtk_widget_set_hexpand(state->gl_area, TRUE);
    gtk_widget_set_vexpand(state->gl_area, TRUE);

    state->video_overlay = gtk_overlay_new();
    gtk_widget_set_hexpand(state->video_overlay, TRUE);
    gtk_widget_set_vexpand(state->video_overlay, TRUE);
    gtk_overlay_set_child(GTK_OVERLAY(state->video_overlay), state->gl_area);
    gtk_paned_set_start_child(GTK_PANED(state->main_area), state->video_overlay);

    state->chat_panel = chat_panel_new(state->chat_width / 2);
    gtk_paned_set_end_child(GTK_PANED(state->main_area), state->chat_panel->widget);
    gtk_paned_set_position(GTK_PANED(state->main_area), state->chat_paned_position);

    GtkGesture *video_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(video_click), GDK_BUTTON_PRIMARY);
    g_signal_connect(video_click, "pressed", G_CALLBACK(on_video_pressed), state);
    gtk_widget_add_controller(state->gl_area, GTK_EVENT_CONTROLLER(video_click));

    GtkEventController *video_legacy = gtk_event_controller_legacy_new();
    g_signal_connect(video_legacy, "event", G_CALLBACK(on_video_legacy_event), state);
    gtk_widget_add_controller(state->gl_area, video_legacy);

    state->bottom_panel = create_controls(state);
    gtk_widget_set_halign(state->bottom_panel, GTK_ALIGN_FILL);
    gtk_widget_set_valign(state->bottom_panel, GTK_ALIGN_END);
    gtk_overlay_add_overlay(GTK_OVERLAY(state->video_overlay), state->bottom_panel);

    state->top_controls = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    gtk_widget_add_css_class(state->top_controls, "top-overlay-controls");
    gtk_widget_set_halign(state->top_controls, GTK_ALIGN_END);
    gtk_widget_set_valign(state->top_controls, GTK_ALIGN_START);
    gtk_overlay_add_overlay(GTK_OVERLAY(state->video_overlay), state->top_controls);

    GtkWidget *minimize_button = create_overlay_button(create_window_icon(WINDOW_ICON_MINIMIZE), "Minimieren");
    gtk_box_append(GTK_BOX(state->top_controls), minimize_button);
    g_signal_connect(minimize_button, "clicked", G_CALLBACK(on_minimize_clicked), state);

    GtkWidget *fullscreen_button = create_overlay_button(create_window_icon(WINDOW_ICON_FULLSCREEN), "Vollbild");
    gtk_box_append(GTK_BOX(state->top_controls), fullscreen_button);
    g_signal_connect(fullscreen_button, "clicked", G_CALLBACK(on_fullscreen_clicked), state);

    GtkWidget *close_button = create_overlay_button(create_window_icon(WINDOW_ICON_CLOSE), "Schliessen");
    gtk_widget_add_css_class(close_button, "close-button");
    gtk_box_append(GTK_BOX(state->top_controls), close_button);
    g_signal_connect(close_button, "clicked", G_CALLBACK(on_close_clicked), state);

    GtkEventController *video_motion = gtk_event_controller_motion_new();
    gtk_event_controller_set_propagation_phase(video_motion, GTK_PHASE_CAPTURE);
    g_signal_connect(video_motion, "motion", G_CALLBACK(on_video_motion), state);
    gtk_widget_add_controller(state->video_overlay, video_motion);

    if (!init_mpv(state)) {
        gtk_widget_set_sensitive(state->stream_combo, FALSE);
    }

    g_signal_connect(state->gl_area, "realize", G_CALLBACK(on_gl_realize), state);
    g_signal_connect(state->gl_area, "unrealize", G_CALLBACK(on_gl_unrealize), state);
    g_signal_connect(state->gl_area, "render", G_CALLBACK(on_gl_render), state);
    g_object_set_data_full(G_OBJECT(state->window), "app-state", state, destroy_state);

    gtk_window_present(GTK_WINDOW(state->window));
    schedule_footer_hide(state);

    if (state->mpv != NULL && state->startup_target != NULL) {
        maybe_start_initial_stream(state);
    }
}

int main(int argc, char **argv)
{
    StartupConfig config = {
        .startup_target = argc > 1 ? argv[1] : NULL,
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
