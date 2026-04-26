#define G_LOG_DOMAIN "twitch-player-2"

#include <gtk/gtk.h>
#include <epoxy/egl.h>
#include <epoxy/gl.h>
#include <mpv/client.h>
#include <mpv/render_gl.h>
#include <locale.h>

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
    GtkWidget *gl_area;
    GtkWidget *main_area;
    GtkWidget *chat_toggle_button;
    GtkWidget *bottom_panel;
    GtkWidget *stream_combo;
    GtkWidget *volume_scale;
    GtkWidget *status_label;
    mpv_handle *mpv;
    mpv_render_context *mpv_gl;
    ChatPanel *chat_panel;
    int chat_width;
    int chat_paned_position;
    gboolean chat_visible;
    gboolean closing;
    gboolean fullscreen;
} AppState;

typedef struct {
    const char *startup_target;
} StartupConfig;

static void set_status(AppState *state, const char *message)
{
    gtk_label_set_text(GTK_LABEL(state->status_label), message);
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
    } else {
        g_debug("%s: ok", action);
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
    g_message("mpv loadfile: %s (%s)", label, url);

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

    guint active = gtk_drop_down_get_selected(GTK_DROP_DOWN(state->stream_combo));

    if (active == GTK_INVALID_LIST_POSITION || active >= G_N_ELEMENTS(STREAMS)) {
        set_status(state, "Kein Stream ausgewaehlt");
        return;
    }

    load_stream_url(state, STREAMS[active].url, STREAMS[active].label, STREAMS[active].channel);
}

static void on_stream_changed(GObject *object, GParamSpec *pspec, gpointer user_data)
{
    (void)object;
    (void)pspec;
    play_selected_stream(user_data);
}

static void on_volume_changed(GtkRange *range, gpointer user_data)
{
    AppState *state = user_data;
    double volume = gtk_range_get_value(range);

    g_debug("mpv volume: %.0f", volume);
    check_mpv(mpv_set_property(state->mpv, "volume", MPV_FORMAT_DOUBLE, &volume), "set volume");
}

static void toggle_fullscreen(AppState *state)
{
    if (state->fullscreen) {
        g_message("fullscreen off");
        gtk_window_unfullscreen(GTK_WINDOW(state->window));
        gtk_widget_set_visible(state->bottom_panel, TRUE);
        state->fullscreen = FALSE;
    } else {
        g_message("fullscreen on");
        gtk_widget_set_visible(state->bottom_panel, FALSE);
        gtk_window_fullscreen(GTK_WINDOW(state->window));
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

    gtk_button_set_label(GTK_BUTTON(state->chat_toggle_button), visible ? "Chat schliessen" : "Chat oeffnen");
}

static void on_chat_toggle_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    AppState *state = user_data;
    set_chat_visible(state, !state->chat_visible);
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

    g_message("GL realize");
    gtk_gl_area_make_current(area);

    if (gtk_gl_area_get_error(area) != NULL) {
        g_warning("GTK GL area error: %s", gtk_gl_area_get_error(area)->message);
        set_status(state, "OpenGL konnte nicht initialisiert werden");
        return;
    }

    GdkGLContext *context = gtk_gl_area_get_context(area);
    if (context != NULL) {
        int major = 0;
        int minor = 0;
        gdk_gl_context_get_version(context, &major, &minor);
        g_message("GTK GL context created: api=%d version=%d.%d", gdk_gl_context_get_api(context), major, minor);
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

    g_message("mpv render context created");
    mpv_render_context_set_update_callback(state->mpv_gl, on_mpv_render_update, state);
}

static void on_gl_unrealize(GtkGLArea *area, gpointer user_data)
{
    AppState *state = user_data;

    gtk_gl_area_make_current(area);

    if (state->mpv_gl != NULL) {
        g_message("mpv render context destroyed");
        mpv_render_context_free(state->mpv_gl);
        state->mpv_gl = NULL;
    }
}

static GtkWidget *create_controls(AppState *state)
{
    GtkWidget *box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_add_css_class(box, "toolbar");
    gtk_widget_set_margin_top(box, 8);
    gtk_widget_set_margin_end(box, 8);
    gtk_widget_set_margin_bottom(box, 8);
    gtk_widget_set_margin_start(box, 8);

    GtkStringList *stream_names = gtk_string_list_new(NULL);
    for (guint i = 0; i < G_N_ELEMENTS(STREAMS); i++) {
        gtk_string_list_append(stream_names, STREAMS[i].label);
    }
    state->stream_combo = gtk_drop_down_new(G_LIST_MODEL(stream_names), NULL);
    g_object_unref(stream_names);
    gtk_drop_down_set_selected(GTK_DROP_DOWN(state->stream_combo), 0);
    gtk_widget_set_hexpand(state->stream_combo, TRUE);

    state->volume_scale = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, 0, 130, 1);
    gtk_range_set_value(GTK_RANGE(state->volume_scale), 80);
    gtk_widget_set_size_request(state->volume_scale, 140, -1);
    gtk_scale_set_draw_value(GTK_SCALE(state->volume_scale), FALSE);

    gtk_box_append(GTK_BOX(box), state->stream_combo);
    state->chat_toggle_button = gtk_button_new_with_label("Chat schliessen");
    gtk_box_append(GTK_BOX(box), state->chat_toggle_button);
    gtk_box_append(GTK_BOX(box), state->volume_scale);

    g_signal_connect(state->stream_combo, "notify::selected", G_CALLBACK(on_stream_changed), state);
    g_signal_connect(state->chat_toggle_button, "clicked", G_CALLBACK(on_chat_toggle_clicked), state);
    g_signal_connect(state->volume_scale, "value-changed", G_CALLBACK(on_volume_changed), state);

    return box;
}

static gboolean init_mpv(AppState *state)
{
    g_message("LC_NUMERIC before mpv_create: %s", setlocale(LC_NUMERIC, NULL));

    if (setlocale(LC_NUMERIC, "C") == NULL) {
        g_warning("LC_NUMERIC could not be set to C; libmpv may refuse to start");
    }

    g_message("LC_NUMERIC for mpv_create: %s", setlocale(LC_NUMERIC, NULL));
    g_message("libmpv client API version: %lu", mpv_client_api_version());

    state->mpv = mpv_create();
    if (state->mpv == NULL) {
        g_warning("mpv_create returned NULL");
        set_status(state, "mpv konnte nicht erstellt werden");
        return FALSE;
    }

    g_message("mpv handle created");
    check_mpv(mpv_set_option_string(state->mpv, "terminal", "no"), "set terminal");
    check_mpv(mpv_set_option_string(state->mpv, "config", "no"), "set config");
    check_mpv(mpv_set_option_string(state->mpv, "vo", "libmpv"), "set vo");
    check_mpv(mpv_set_option_string(state->mpv, "ytdl", "yes"), "set ytdl");
    check_mpv(mpv_set_option_string(state->mpv, "hwdec", "auto-safe"), "set hwdec");
    check_mpv(mpv_set_option_string(state->mpv, "volume", "80"), "set volume");
    check_mpv(mpv_request_log_messages(state->mpv, "info"), "request log messages");

    int status = mpv_initialize(state->mpv);
    if (status < 0) {
        g_warning("mpv init: %s", mpv_error_string(status));
        set_status(state, "mpv konnte nicht initialisiert werden");
        return FALSE;
    }

    g_message("mpv initialized");
    mpv_set_wakeup_callback(state->mpv, on_mpv_wakeup, state);
    return TRUE;
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
        g_message("no startup stream target provided");
        return;
    }

    g_message("startup stream target: %s -> %s", state->startup_target, url);
    load_stream_url(state, url, label, channel);
}

static void destroy_state(gpointer user_data)
{
    AppState *state = user_data;

    g_message("destroy app state");
    state->closing = TRUE;

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

    AppState *state = g_new0(AppState, 1);
    state->application = application;
    state->startup_target = config != NULL ? config->startup_target : NULL;

    g_message("activate app, startup target: %s", state->startup_target != NULL ? state->startup_target : "(none)");

    state->window = gtk_application_window_new(application);
    gtk_window_set_title(GTK_WINDOW(state->window), "Twitch Player");
    gtk_window_set_default_size(GTK_WINDOW(state->window), 1100, 680);

    GtkWidget *root = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_window_set_child(GTK_WINDOW(state->window), root);

    state->chat_width = 360;
    state->chat_paned_position = 740;
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
    gtk_paned_set_start_child(GTK_PANED(state->main_area), state->gl_area);

    state->chat_panel = chat_panel_new(state->chat_width);
    gtk_paned_set_end_child(GTK_PANED(state->main_area), state->chat_panel->widget);
    gtk_paned_set_position(GTK_PANED(state->main_area), state->chat_paned_position);

    GtkGesture *video_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(video_click), GDK_BUTTON_PRIMARY);
    g_signal_connect(video_click, "pressed", G_CALLBACK(on_video_pressed), state);
    gtk_widget_add_controller(state->gl_area, GTK_EVENT_CONTROLLER(video_click));

    state->bottom_panel = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_box_append(GTK_BOX(root), state->bottom_panel);
    gtk_box_append(GTK_BOX(state->bottom_panel), create_controls(state));

    state->status_label = gtk_label_new("Bereit");
    gtk_widget_set_halign(state->status_label, GTK_ALIGN_START);
    gtk_widget_set_margin_end(state->status_label, 8);
    gtk_widget_set_margin_bottom(state->status_label, 8);
    gtk_widget_set_margin_start(state->status_label, 8);
    gtk_box_append(GTK_BOX(state->bottom_panel), state->status_label);

    if (!init_mpv(state)) {
        gtk_widget_set_sensitive(state->stream_combo, FALSE);
    }

    g_signal_connect(state->gl_area, "realize", G_CALLBACK(on_gl_realize), state);
    g_signal_connect(state->gl_area, "unrealize", G_CALLBACK(on_gl_unrealize), state);
    g_signal_connect(state->gl_area, "render", G_CALLBACK(on_gl_render), state);
    g_object_set_data_full(G_OBJECT(state->window), "app-state", state, destroy_state);

    gtk_window_present(GTK_WINDOW(state->window));

    if (state->mpv != NULL && state->startup_target != NULL) {
        maybe_start_initial_stream(state);
    }
}

int main(int argc, char **argv)
{
    StartupConfig config = {
        .startup_target = argc > 1 ? argv[1] : NULL,
    };

    g_setenv("G_MESSAGES_DEBUG", G_LOG_DOMAIN, FALSE);
    g_message("twitch-player-2 starting");

    g_autoptr(GtkApplication) application = gtk_application_new(
        "dev.codex.twitch-player-2",
        G_APPLICATION_NON_UNIQUE
    );

    g_signal_connect(application, "activate", G_CALLBACK(on_activate), &config);

    return g_application_run(G_APPLICATION(application), 1, argv);
}
