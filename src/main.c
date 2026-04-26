#define G_LOG_DOMAIN "twitch-player-2"

#include <gtk/gtk.h>
#include <epoxy/egl.h>
#include <epoxy/gl.h>
#include <mpv/client.h>
#include <mpv/render_gl.h>
#include <locale.h>
#include <math.h>

#include "chat_panel.h"

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
    GtkWidget *chat_toggle_button;
    GtkWidget *bottom_panel;
    GtkWidget *footer_spacer;
    GtkWidget *stream_combo;
    GtkWidget *volume_scale;
    GtkWidget *status_label;
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
    gboolean has_last_motion;
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

    if (active >= G_N_ELEMENTS(STREAMS)) {
        set_status(state, "Kein Stream ausgewaehlt");
        return;
    }

    load_stream_url(state, STREAMS[active].url, STREAMS[active].label, STREAMS[active].channel);
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

static gboolean hide_footer(gpointer user_data)
{
    AppState *state = user_data;

    state->footer_hide_source = 0;

    if (!state->closing) {
        gtk_widget_set_visible(state->bottom_panel, FALSE);
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
    gtk_widget_set_visible(state->chat_toggle_button, TRUE);
    schedule_footer_hide(state);
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

static void on_stream_menu_clicked(GtkButton *button, gpointer user_data)
{
    AppState *state = user_data;
    gpointer index_data = g_object_get_data(G_OBJECT(button), "stream-index");

    state->active_stream = GPOINTER_TO_UINT(index_data);
    GtkWidget *child = gtk_menu_button_get_child(GTK_MENU_BUTTON(state->stream_combo));
    if (GTK_IS_LABEL(child)) {
        gtk_label_set_text(GTK_LABEL(child), STREAMS[state->active_stream].label);
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
    for (guint i = 0; i < G_N_ELEMENTS(STREAMS); i++) {
        GtkWidget *item = gtk_button_new();
        GtkWidget *item_label = gtk_label_new(STREAMS[i].label);
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
    GtkWidget *stream_label = gtk_label_new(STREAMS[state->active_stream].label);
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
        ".chat-toggle {"
        "  background: rgba(0, 0, 0, 0.58);"
        "  color: white;"
        "  border-color: transparent;"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "  margin: 8px;"
        "  min-width: 34px;"
        "  min-height: 30px;"
        "  padding: 4px 8px;"
        "}"
        ".chat-toggle:hover {"
        "  background: rgba(54, 54, 54, 0.90);"
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

static char *target_to_url(const char *target, const char **label, char **channel)
{
    if (target == NULL || target[0] == '\0') {
        return NULL;
    }

    for (guint i = 0; i < G_N_ELEMENTS(STREAMS); i++) {
        if (g_ascii_strcasecmp(target, STREAMS[i].label) == 0 ||
            g_ascii_strcasecmp(target, STREAMS[i].channel) == 0 ||
            g_ascii_strcasecmp(target, STREAMS[i].url) == 0) {
            *label = STREAMS[i].label;
            *channel = g_strdup(STREAMS[i].channel);
            return g_strdup(STREAMS[i].url);
        }
    }

    if (g_str_has_prefix(target, "http://") || g_str_has_prefix(target, "https://")) {
        *label = "custom URL";
        *channel = extract_twitch_channel(target);
        return g_strdup(target);
    }

    *label = target;
    *channel = g_ascii_strdown(target, -1);
    return g_strdup_printf("https://www.twitch.tv/%s", target);
}

static void maybe_start_initial_stream(AppState *state)
{
    const char *label = NULL;
    g_autofree char *channel = NULL;
    g_autofree char *url = target_to_url(state->startup_target, &label, &channel);

    if (url == NULL) {
        return;
    }

    load_stream_url(state, url, label, channel);
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
}

static void on_activate(GtkApplication *application, gpointer user_data)
{
    StartupConfig *config = user_data;

    install_css();

    AppState *state = g_new0(AppState, 1);
    state->application = application;
    state->startup_target = config != NULL ? config->startup_target : NULL;

    state->window = gtk_application_window_new(application);
    gtk_window_set_title(GTK_WINDOW(state->window), "Twitch Player");
    gtk_window_set_default_size(GTK_WINDOW(state->window), 1100, 680);

    GtkWidget *root = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_window_set_child(GTK_WINDOW(state->window), root);

    state->chat_width = 360;
    state->chat_paned_position = 740;
    state->active_stream = 0;
    state->chat_visible = TRUE;

    state->main_area = gtk_paned_new(GTK_ORIENTATION_HORIZONTAL);
    gtk_widget_set_hexpand(state->main_area, TRUE);
    gtk_widget_set_vexpand(state->main_area, TRUE);
    gtk_paned_set_wide_handle(GTK_PANED(state->main_area), TRUE);
    gtk_paned_set_resize_start_child(GTK_PANED(state->main_area), TRUE);
    gtk_paned_set_shrink_start_child(GTK_PANED(state->main_area), FALSE);
    gtk_paned_set_resize_end_child(GTK_PANED(state->main_area), FALSE);
    gtk_paned_set_shrink_end_child(GTK_PANED(state->main_area), FALSE);
    gtk_box_append(GTK_BOX(root), state->main_area);

    state->gl_area = gtk_gl_area_new();
    gtk_gl_area_set_auto_render(GTK_GL_AREA(state->gl_area), FALSE);
    gtk_widget_set_hexpand(state->gl_area, TRUE);
    gtk_widget_set_vexpand(state->gl_area, TRUE);

    state->video_overlay = gtk_overlay_new();
    gtk_widget_set_hexpand(state->video_overlay, TRUE);
    gtk_widget_set_vexpand(state->video_overlay, TRUE);
    gtk_overlay_set_child(GTK_OVERLAY(state->video_overlay), state->gl_area);
    gtk_paned_set_start_child(GTK_PANED(state->main_area), state->video_overlay);

    state->chat_panel = chat_panel_new(state->chat_width);
    gtk_paned_set_end_child(GTK_PANED(state->main_area), state->chat_panel->widget);
    gtk_paned_set_position(GTK_PANED(state->main_area), state->chat_paned_position);

    GtkGesture *video_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(video_click), GDK_BUTTON_PRIMARY);
    g_signal_connect(video_click, "pressed", G_CALLBACK(on_video_pressed), state);
    gtk_widget_add_controller(state->gl_area, GTK_EVENT_CONTROLLER(video_click));

    state->bottom_panel = create_controls(state);
    gtk_widget_set_halign(state->bottom_panel, GTK_ALIGN_FILL);
    gtk_widget_set_valign(state->bottom_panel, GTK_ALIGN_END);
    gtk_overlay_add_overlay(GTK_OVERLAY(state->video_overlay), state->bottom_panel);

    state->chat_toggle_button = gtk_button_new();
    gtk_button_set_child(GTK_BUTTON(state->chat_toggle_button), create_chat_icon(CHAT_ICON_CLOSE));
    gtk_widget_add_css_class(state->chat_toggle_button, "chat-toggle");
    gtk_widget_set_tooltip_text(state->chat_toggle_button, "Chat schliessen");
    gtk_widget_set_halign(state->chat_toggle_button, GTK_ALIGN_END);
    gtk_widget_set_valign(state->chat_toggle_button, GTK_ALIGN_START);
    gtk_overlay_add_overlay(GTK_OVERLAY(state->video_overlay), state->chat_toggle_button);
    g_signal_connect(state->chat_toggle_button, "clicked", G_CALLBACK(on_chat_toggle_clicked), state);

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

    g_autoptr(GtkApplication) application = gtk_application_new(
        "dev.codex.twitch-player-2",
        G_APPLICATION_NON_UNIQUE
    );

    g_signal_connect(application, "activate", G_CALLBACK(on_activate), &config);

    return g_application_run(G_APPLICATION(application), 1, argv);
}
