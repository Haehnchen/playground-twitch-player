#define G_LOG_DOMAIN "twitch-player-single"

#include <gio/gio.h>
#include <gtk/gtk.h>
#include <epoxy/egl.h>
#include <epoxy/gl.h>
#include <mpv/render_gl.h>
#include <math.h>
#include <string.h>

#include "single_player.h"

#include "chat_panel.h"
#include "player_defaults.h"
#include "player_icons.h"
#include "player_motion.h"
#include "player_volume.h"
#include "twitch_stream_info.h"

#define STREAM_TITLE_REFRESH_SECONDS 60
#define STREAM_DROPDOWN_WIDTH 170
#define DEFAULT_CHAT_WIDTH 280
#define MIN_CHAT_WIDTH 180
#define MIN_VIDEO_WIDTH 320
#define MPV_MAINLOOP_PRIORITY G_PRIORITY_HIGH

typedef struct {
    const char *label;
    const char *channel;
    const char *url;
} StreamEntry;

struct _SinglePlayer {
    const char *startup_target;
    GtkWidget *window;
    GtkWidget *video_overlay;
    GtkWidget *gl_area;
    GtkWidget *main_area;
    GtkWidget *chat_toggle_button;
    GtkWidget *bottom_panel;
    GtkWidget *footer_spacer;
    GtkWidget *stream_combo;
    GtkWidget *stream_title_label;
    GtkWidget *volume_scale;
    GtkWidget *status_label;
    StreamEntry *streams;
    guint stream_count;
    PlayerSession *session;
    mpv_render_context *mpv_gl;
    ChatPanel *chat_panel;
    AppSettings *settings;
    GCancellable *title_cancel;
    int chat_paned_position;
    int last_render_width;
    int last_render_height;
    gint render_queued;
    gint event_queued;
    guint render_warmup_source;
    int render_warmup_frames;
    guint active_stream;
    gboolean chat_visible;
    guint footer_hide_source;
    guint title_refresh_source;
    guint chat_position_source;
    guint title_generation;
    PlayerMotionTracker motion_tracker;
    double move_press_x;
    double move_press_y;
    gboolean move_pressed;
    gboolean closing;
    gboolean fullscreen;
    SinglePlayerFullscreenCallback fullscreen_callback;
    gpointer fullscreen_user_data;
    gboolean stream_playing;
    gboolean title_fetch_in_progress;
};

typedef struct {
    SinglePlayer *state;
    guint generation;
} StreamTitleCallbackData;

static void init_streams(SinglePlayer *state, const char *target);
static void free_streams(SinglePlayer *state);
static void rebuild_stream_menu(SinglePlayer *state);
static void update_stream_combo_label(SinglePlayer *state);

static int clamp_chat_paned_position(int position, int width)
{
    if (width <= 1) {
        return position;
    }

    int min_position = MIN(MIN_VIDEO_WIDTH, MAX(1, width - MIN_CHAT_WIDTH));
    int max_position = MAX(min_position, width - MIN_CHAT_WIDTH);
    return CLAMP(position, min_position, max_position);
}

static int get_default_chat_paned_position(int width)
{
    return width > 1 ? width - DEFAULT_CHAT_WIDTH : 0;
}

static int get_chat_paned_position_for_width(SinglePlayer *state, int width)
{
    int position = state->chat_paned_position > 0
        ? state->chat_paned_position
        : get_default_chat_paned_position(width);
    return clamp_chat_paned_position(position, width);
}

static gboolean apply_chat_position(gpointer user_data)
{
    SinglePlayer *state = user_data;

    if (state->closing || state->main_area == NULL) {
        state->chat_position_source = 0;
        return G_SOURCE_REMOVE;
    }

    int width = gtk_widget_get_width(state->main_area);
    if (width <= 1) {
        return G_SOURCE_CONTINUE;
    }

    state->chat_paned_position = get_chat_paned_position_for_width(state, width);
    gtk_paned_set_position(GTK_PANED(state->main_area), state->chat_paned_position);
    state->chat_position_source = 0;

    return G_SOURCE_REMOVE;
}

static void set_status(SinglePlayer *state, const char *message)
{
    if (state->status_label != NULL) {
        gtk_label_set_text(GTK_LABEL(state->status_label), message);
    }
}

static void set_stream_title(SinglePlayer *state, const char *title)
{
    if (state->stream_title_label == NULL) {
        return;
    }

    gtk_label_set_text(GTK_LABEL(state->stream_title_label), title != NULL ? title : "");
    gtk_widget_set_tooltip_text(state->stream_title_label, title != NULL && title[0] != '\0' ? title : NULL);
}

static const char *get_active_stream_channel(SinglePlayer *state)
{
    if (state->active_stream >= state->stream_count) {
        return NULL;
    }

    return state->streams[state->active_stream].channel;
}

static void on_stream_title_fetched(GObject *source_object, GAsyncResult *result, gpointer user_data)
{
    (void)source_object;
    StreamTitleCallbackData *data = user_data;
    SinglePlayer *state = data->state;
    g_autoptr(GError) error = NULL;
    g_autofree char *title = twitch_stream_info_fetch_title_finish(result, &error);

    if (data->generation != state->title_generation) {
        g_free(data);
        return;
    }

    state->title_fetch_in_progress = FALSE;
    g_clear_object(&state->title_cancel);

    if (state->closing || !state->stream_playing) {
        g_free(data);
        return;
    }

    if (error != NULL) {
        if (!g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            g_debug("stream title fetch failed: %s", error->message);
        }
        g_free(data);
        return;
    }

    set_stream_title(state, title);
    g_free(data);
}

static void request_stream_title_update(SinglePlayer *state, gboolean force)
{
    const char *channel = get_active_stream_channel(state);

    if (state->closing || !state->stream_playing || channel == NULL || channel[0] == '\0') {
        return;
    }
    if (state->title_fetch_in_progress && !force) {
        return;
    }

    if (force) {
        state->title_generation++;
        if (state->title_cancel != NULL) {
            g_cancellable_cancel(state->title_cancel);
            g_clear_object(&state->title_cancel);
        }
        state->title_fetch_in_progress = FALSE;
    }

    StreamTitleCallbackData *data = g_new0(StreamTitleCallbackData, 1);
    data->state = state;
    data->generation = ++state->title_generation;

    state->title_cancel = g_cancellable_new();
    state->title_fetch_in_progress = TRUE;

    twitch_stream_info_fetch_title_async(channel, state->title_cancel, on_stream_title_fetched, data);
}

static gboolean refresh_stream_title(gpointer user_data)
{
    SinglePlayer *state = user_data;

    if (state->closing) {
        state->title_refresh_source = 0;
        return G_SOURCE_REMOVE;
    }

    request_stream_title_update(state, FALSE);
    return G_SOURCE_CONTINUE;
}

static void reset_stream_title(SinglePlayer *state)
{
    state->title_generation++;
    if (state->title_cancel != NULL) {
        g_cancellable_cancel(state->title_cancel);
        g_clear_object(&state->title_cancel);
    }
    state->title_fetch_in_progress = FALSE;
    set_stream_title(state, "");
}

static void start_chat(SinglePlayer *state, const char *channel)
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

static mpv_handle *get_mpv(SinglePlayer *state)
{
    return player_session_get_mpv(state->session);
}

static void remove_source_if_active(guint *source_id)
{
    if (*source_id == 0) {
        return;
    }

    GSource *source = g_main_context_find_source_by_id(NULL, *source_id);
    if (source != NULL) {
        g_source_destroy(source);
    }
    *source_id = 0;
}

static void *get_proc_address(void *ctx, const char *name)
{
    (void)ctx;
    return (void *)eglGetProcAddress(name);
}

static gboolean queue_mpv_render(gpointer user_data)
{
    SinglePlayer *state = user_data;

    g_atomic_int_set(&state->render_queued, 0);

    if (!state->closing && state->gl_area != NULL) {
        gtk_gl_area_queue_render(GTK_GL_AREA(state->gl_area));
    }

    return G_SOURCE_REMOVE;
}

static gboolean warmup_video_render(gpointer user_data)
{
    SinglePlayer *state = user_data;

    if (state->closing || state->gl_area == NULL || state->render_warmup_frames <= 0) {
        state->render_warmup_source = 0;
        return G_SOURCE_REMOVE;
    }

    state->render_warmup_frames--;
    gtk_gl_area_queue_render(GTK_GL_AREA(state->gl_area));
    return G_SOURCE_CONTINUE;
}

static void start_render_warmup(SinglePlayer *state)
{
    remove_source_if_active(&state->render_warmup_source);
    state->render_warmup_frames = 90;
    state->render_warmup_source = g_timeout_add(16, warmup_video_render, state);
}

static void on_mpv_render_update(void *ctx)
{
    SinglePlayer *state = ctx;

    if (g_atomic_int_compare_and_exchange(&state->render_queued, 0, 1)) {
        g_idle_add_full(MPV_MAINLOOP_PRIORITY, queue_mpv_render, state, NULL);
    }
}

static gboolean process_mpv_events(gpointer user_data)
{
    SinglePlayer *state = user_data;

    g_atomic_int_set(&state->event_queued, 0);

    mpv_handle *mpv = get_mpv(state);
    if (state->closing || mpv == NULL) {
        return G_SOURCE_REMOVE;
    }

    while (true) {
        mpv_event *event = mpv_wait_event(mpv, 0);

        if (event->event_id == MPV_EVENT_NONE) {
            break;
        }

        switch (event->event_id) {
        case MPV_EVENT_START_FILE:
            set_status(state, "Loading stream");
            break;
        case MPV_EVENT_FILE_LOADED:
            set_status(state, "Playback running");
            break;
        case MPV_EVENT_END_FILE: {
            mpv_event_end_file *end = event->data;
            if (end != NULL && end->reason == MPV_END_FILE_REASON_ERROR) {
                set_status(state, "Stream could not be played");
            } else {
                set_status(state, "Stopped");
            }
            break;
        }
        case MPV_EVENT_VIDEO_RECONFIG:
            /* After a Twitch ad the stream resumes with a huge PTS jump, causing
             * an internal mpv playback reset. The video output needs a warmup so
             * the GL renderer actively polls for new frames again. */
            start_render_warmup(state);
            break;
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
    SinglePlayer *state = ctx;

    if (g_atomic_int_compare_and_exchange(&state->event_queued, 0, 1)) {
        g_idle_add_full(MPV_MAINLOOP_PRIORITY, process_mpv_events, state, NULL);
    }
}

static void load_stream_url(SinglePlayer *state, const char *url, const char *label, const char *channel)
{
    set_status(state, PLAYER_STARTING_STREAM_STATUS);
    state->stream_playing = TRUE;
    update_stream_combo_label(state);
    reset_stream_title(state);
    start_chat(state, channel);
    request_stream_title_update(state, TRUE);
    player_session_load_stream(state->session, url, label, channel);
}

static void play_selected_stream(SinglePlayer *state)
{
    if (!player_session_is_ready(state->session)) {
        g_warning("play requested, but mpv is not available");
        return;
    }

    guint active = state->active_stream;

    if (active >= state->stream_count) {
        set_status(state, "No stream selected");
        return;
    }

    load_stream_url(state, state->streams[active].url, state->streams[active].label, state->streams[active].channel);
}

static void on_volume_changed(GtkRange *range, gpointer user_data)
{
    SinglePlayer *state = user_data;

    player_volume_sync_session_from_range(state->session, range);
}

static void toggle_mute(SinglePlayer *state)
{
    mpv_handle *mpv = get_mpv(state);
    if (mpv == NULL) {
        return;
    }

    const char *cmd[] = {
        "cycle",
        "mute",
        NULL,
    };

    check_mpv(mpv_command(mpv, cmd), "toggle mute");
}

static void schedule_footer_hide(SinglePlayer *state);

static GtkWidget *create_overlay_button(GtkWidget *icon, const char *tooltip)
{
    GtkWidget *button = gtk_button_new();
    gtk_button_set_child(GTK_BUTTON(button), icon);
    gtk_widget_add_css_class(button, "overlay-icon-button");
    gtk_widget_set_tooltip_text(button, tooltip);
    return button;
}

static gboolean is_stream_menu_open(SinglePlayer *state)
{
    if (state->stream_combo == NULL) {
        return FALSE;
    }

    GtkPopover *popover = gtk_menu_button_get_popover(GTK_MENU_BUTTON(state->stream_combo));
    return popover != NULL && gtk_widget_get_mapped(GTK_WIDGET(popover));
}

static gboolean hide_footer(gpointer user_data)
{
    SinglePlayer *state = user_data;

    state->footer_hide_source = 0;

    if (is_stream_menu_open(state)) {
        schedule_footer_hide(state);
        return G_SOURCE_REMOVE;
    }

    if (!state->closing) {
        if (state->bottom_panel != NULL) {
            gtk_widget_set_visible(state->bottom_panel, FALSE);
        }
        if (state->chat_toggle_button != NULL) {
            gtk_widget_set_visible(state->chat_toggle_button, FALSE);
        }
    }

    return G_SOURCE_REMOVE;
}

static void schedule_footer_hide(SinglePlayer *state)
{
    remove_source_if_active(&state->footer_hide_source);

    state->footer_hide_source = g_timeout_add(1800, hide_footer, state);
}

static void show_footer(SinglePlayer *state)
{
    if (state->closing) {
        return;
    }

    if (state->bottom_panel != NULL) {
        gtk_widget_set_visible(state->bottom_panel, TRUE);
    }
    if (state->chat_toggle_button != NULL) {
        gtk_widget_set_visible(state->chat_toggle_button, TRUE);
    }
    schedule_footer_hide(state);
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

static void begin_window_move_from_event(SinglePlayer *state, GdkEvent *event, guint button)
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

static void on_video_motion(GtkEventControllerMotion *controller, double x, double y, gpointer user_data)
{
    (void)controller;
    SinglePlayer *state = user_data;

    if (player_motion_tracker_ignore_stationary(&state->motion_tracker, state, x, y)) {
        return;
    }

    show_footer(state);
}

static void request_fullscreen_toggle(SinglePlayer *state)
{
    if (state->fullscreen_callback != NULL) {
        state->fullscreen_callback(state->fullscreen_user_data);
    }
    show_footer(state);
}

static void on_video_pressed(GtkGestureClick *gesture, int n_press, double x, double y, gpointer user_data)
{
    (void)gesture;
    (void)x;
    (void)y;

    if (n_press == 2) {
        request_fullscreen_toggle(user_data);
    }
}

static gboolean on_video_legacy_event(GtkEventControllerLegacy *controller, GdkEvent *event, gpointer user_data)
{
    (void)controller;
    SinglePlayer *state = user_data;
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

static gboolean on_video_scroll(GtkEventControllerScroll *controller, double dx, double dy, gpointer user_data)
{
    (void)controller;
    SinglePlayer *state = user_data;

    if (!player_volume_apply_scroll(state->volume_scale, dx, dy)) {
        return GDK_EVENT_PROPAGATE;
    }

    show_footer(state);

    return GDK_EVENT_STOP;
}

static void set_chat_visible(SinglePlayer *state, gboolean visible)
{
    if (!GTK_IS_PANED(state->main_area) || state->chat_panel == NULL) {
        return;
    }

    if (visible) {
        state->chat_visible = TRUE;
        gtk_paned_set_end_child(GTK_PANED(state->main_area), state->chat_panel->widget);
        int width = gtk_widget_get_width(state->main_area);
        if (width > 1) {
            state->chat_paned_position = get_chat_paned_position_for_width(state, width);
            gtk_paned_set_position(GTK_PANED(state->main_area), state->chat_paned_position);
        } else if (state->chat_position_source == 0) {
            state->chat_position_source = g_timeout_add(50, apply_chat_position, state);
        }
    } else {
        remove_source_if_active(&state->chat_position_source);
        int position = gtk_paned_get_position(GTK_PANED(state->main_area));
        if (position > 0) {
            state->chat_paned_position = position;
        }
        state->chat_visible = FALSE;
        g_object_ref(state->chat_panel->widget);
        gtk_paned_set_end_child(GTK_PANED(state->main_area), NULL);
    }

    gtk_widget_set_tooltip_text(state->chat_toggle_button, visible ? "Close chat" : "Open chat");
    gtk_button_set_child(
        GTK_BUTTON(state->chat_toggle_button),
        player_chat_icon_new(visible ? PLAYER_CHAT_ICON_CLOSE : PLAYER_CHAT_ICON_OPEN)
    );
}

static void on_chat_toggle_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    SinglePlayer *state = user_data;
    set_chat_visible(state, !state->chat_visible);
    show_footer(state);
}

static void on_stream_info_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    SinglePlayer *state = user_data;

    mpv_handle *mpv = get_mpv(state);
    if (mpv == NULL) {
        return;
    }

    const char *stats_cmd[] = {
        "script-binding",
        "stats/display-stats-toggle",
        NULL,
    };

    int status = mpv_command(mpv, stats_cmd);
    if (status < 0) {
        const char *keypress_cmd[] = {
            "keypress",
            "i",
            NULL,
        };
        check_mpv(mpv_command(mpv, keypress_cmd), "toggle stream info");
    }

    show_footer(state);
}

static void on_stream_menu_clicked(GtkButton *button, gpointer user_data)
{
    SinglePlayer *state = user_data;
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

static void update_stream_combo_label(SinglePlayer *state)
{
    if (state->stream_combo == NULL) {
        return;
    }

    GtkWidget *child = gtk_menu_button_get_child(GTK_MENU_BUTTON(state->stream_combo));
    if (!GTK_IS_LABEL(child)) {
        return;
    }

    if (!state->stream_playing || state->stream_count == 0 || state->active_stream >= state->stream_count) {
        gtk_label_set_text(GTK_LABEL(child), PLAYER_EMPTY_STREAM_LABEL);
        return;
    }

    gtk_label_set_text(GTK_LABEL(child), state->streams[state->active_stream].label);
}

static void rebuild_stream_menu(SinglePlayer *state)
{
    GtkWidget *stream_menu = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class(stream_menu, "stream-menu");
    gtk_widget_set_size_request(stream_menu, STREAM_DROPDOWN_WIDTH, -1);

    for (guint i = 0; i < state->stream_count; i++) {
        GtkWidget *item = gtk_button_new();
        GtkWidget *item_content = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
        GtkWidget *item_label = gtk_label_new(state->streams[i].label);

        gtk_label_set_xalign(GTK_LABEL(item_label), 0.0);
        gtk_widget_set_halign(item_label, GTK_ALIGN_START);
        gtk_widget_set_hexpand(item_label, TRUE);
        gtk_widget_set_halign(item_content, GTK_ALIGN_FILL);
        gtk_widget_set_hexpand(item_content, TRUE);
        gtk_box_append(GTK_BOX(item_content), item_label);
        gtk_button_set_child(GTK_BUTTON(item), item_content);
        gtk_widget_add_css_class(item, "stream-menu-item");
        gtk_widget_set_halign(item, GTK_ALIGN_FILL);
        gtk_widget_set_hexpand(item, TRUE);
        gtk_widget_set_size_request(item, STREAM_DROPDOWN_WIDTH, -1);
        gtk_widget_set_margin_start(item, 0);
        gtk_widget_set_margin_end(item, 0);
        gtk_widget_set_margin_top(item, 0);
        gtk_widget_set_margin_bottom(item, 0);
        g_object_set_data(G_OBJECT(item), "stream-index", GUINT_TO_POINTER(i));
        g_signal_connect(item, "clicked", G_CALLBACK(on_stream_menu_clicked), state);
        gtk_box_append(GTK_BOX(stream_menu), item);
    }

    GtkWidget *stream_popover = gtk_popover_new();
    gtk_widget_add_css_class(stream_popover, "stream-popover");
    gtk_widget_set_size_request(stream_popover, STREAM_DROPDOWN_WIDTH, -1);
    gtk_popover_set_position(GTK_POPOVER(stream_popover), GTK_POS_TOP);
    gtk_popover_set_has_arrow(GTK_POPOVER(stream_popover), FALSE);
    gtk_popover_set_child(GTK_POPOVER(stream_popover), stream_menu);
    gtk_menu_button_set_popover(GTK_MENU_BUTTON(state->stream_combo), stream_popover);
    gtk_widget_set_sensitive(state->stream_combo, state->stream_count > 0);
    update_stream_combo_label(state);
}

static gboolean on_gl_render(GtkGLArea *area, GdkGLContext *context, gpointer user_data)
{
    (void)context;
    SinglePlayer *state = user_data;

    if (state->mpv_gl == NULL) {
        return TRUE;
    }

    int scale = gtk_widget_get_scale_factor(GTK_WIDGET(area));
    int width = gtk_widget_get_width(GTK_WIDGET(area)) * scale;
    int height = gtk_widget_get_height(GTK_WIDGET(area)) * scale;

    if (width <= 0 || height <= 0) {
        return TRUE;
    }

    uint64_t update_flags = mpv_render_context_update(state->mpv_gl);
    gboolean size_changed = width != state->last_render_width || height != state->last_render_height;
    gboolean warming_up = state->render_warmup_frames > 0;

    if ((update_flags & MPV_RENDER_UPDATE_FRAME) == 0 && !size_changed && !warming_up) {
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

    int status = mpv_render_context_render(state->mpv_gl, params);
    if (status < 0) {
        g_warning("mpv render: %s", mpv_error_string(status));
    } else {
        state->last_render_width = width;
        state->last_render_height = height;
    }

    return TRUE;
}

static void on_gl_realize(GtkGLArea *area, gpointer user_data)
{
    SinglePlayer *state = user_data;

    mpv_handle *mpv = get_mpv(state);
    if (mpv == NULL) {
        g_debug("GL realize skipped: mpv is not available");
        return;
    }

    gtk_gl_area_make_current(area);

    if (gtk_gl_area_get_error(area) != NULL) {
        g_warning("GTK GL area error: %s", gtk_gl_area_get_error(area)->message);
        set_status(state, "OpenGL could not be initialized");
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

    int status = mpv_render_context_create(&state->mpv_gl, mpv, params);
    if (status < 0) {
        g_warning("mpv render context: %s", mpv_error_string(status));
        set_status(state, "mpv rendering could not be started");
        return;
    }

    mpv_render_context_set_update_callback(state->mpv_gl, on_mpv_render_update, state);
    player_session_reenable_video(state->session);
    start_render_warmup(state);
    gtk_gl_area_queue_render(area);
}

static void clear_mpv_render_context(SinglePlayer *state)
{
    if (state->gl_area != NULL && gtk_widget_get_realized(state->gl_area)) {
        gtk_gl_area_make_current(GTK_GL_AREA(state->gl_area));
    }

    if (state->mpv_gl != NULL) {
        mpv_render_context_set_update_callback(state->mpv_gl, NULL, NULL);
        mpv_render_context_free(state->mpv_gl);
        state->mpv_gl = NULL;
    }
    remove_source_if_active(&state->render_warmup_source);

    state->last_render_width = 0;
    state->last_render_height = 0;
    state->render_warmup_frames = 0;
}

static void on_gl_unrealize(GtkGLArea *area, gpointer user_data)
{
    SinglePlayer *state = user_data;

    gtk_gl_area_make_current(area);
    clear_mpv_render_context(state);
}

static GtkWidget *create_controls(SinglePlayer *state)
{
    GtkWidget *box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_add_css_class(box, "video-footer");

    state->stream_combo = gtk_menu_button_new();
    gtk_widget_add_css_class(state->stream_combo, "stream-dropdown");
    GtkWidget *stream_label = gtk_label_new("");
    gtk_widget_add_css_class(stream_label, "stream-button-label");
    gtk_widget_set_halign(stream_label, GTK_ALIGN_START);
    gtk_label_set_xalign(GTK_LABEL(stream_label), 0.0);
    gtk_menu_button_set_child(GTK_MENU_BUTTON(state->stream_combo), stream_label);
    gtk_widget_set_halign(state->stream_combo, GTK_ALIGN_START);
    gtk_menu_button_set_direction(GTK_MENU_BUTTON(state->stream_combo), GTK_ARROW_UP);
    gtk_menu_button_set_always_show_arrow(GTK_MENU_BUTTON(state->stream_combo), FALSE);
    gtk_widget_set_size_request(state->stream_combo, STREAM_DROPDOWN_WIDTH, -1);
    gtk_widget_set_hexpand(state->stream_combo, FALSE);
    rebuild_stream_menu(state);

    state->stream_title_label = gtk_label_new("");
    gtk_widget_add_css_class(state->stream_title_label, "stream-title-label");
    gtk_widget_set_halign(state->stream_title_label, GTK_ALIGN_START);
    gtk_widget_set_valign(state->stream_title_label, GTK_ALIGN_CENTER);
    gtk_widget_set_hexpand(state->stream_title_label, TRUE);
    gtk_label_set_xalign(GTK_LABEL(state->stream_title_label), 0.0);
    gtk_label_set_ellipsize(GTK_LABEL(state->stream_title_label), PANGO_ELLIPSIZE_END);
    gtk_label_set_single_line_mode(GTK_LABEL(state->stream_title_label), TRUE);

    state->volume_scale = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, PLAYER_VOLUME_MIN, PLAYER_VOLUME_MAX, 1);
    gtk_range_set_value(GTK_RANGE(state->volume_scale), player_session_get_volume(state->session));
    gtk_widget_set_size_request(state->volume_scale, 140, -1);
    gtk_scale_set_draw_value(GTK_SCALE(state->volume_scale), FALSE);

    gtk_box_append(GTK_BOX(box), state->stream_combo);
    gtk_box_append(GTK_BOX(box), state->stream_title_label);
    state->footer_spacer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_set_hexpand(state->footer_spacer, FALSE);
    gtk_box_append(GTK_BOX(box), state->footer_spacer);
    gtk_box_append(GTK_BOX(box), state->volume_scale);

    GtkWidget *stream_info_button = create_overlay_button(player_info_icon_new(), PLAYER_STREAM_INFO_TOOLTIP);
    gtk_box_append(GTK_BOX(box), stream_info_button);

    state->chat_toggle_button = create_overlay_button(player_chat_icon_new(PLAYER_CHAT_ICON_OPEN), "Open chat");
    gtk_widget_add_css_class(state->chat_toggle_button, "chat-toggle");
    gtk_box_append(GTK_BOX(box), state->chat_toggle_button);

    g_signal_connect(stream_info_button, "clicked", G_CALLBACK(on_stream_info_clicked), state);
    g_signal_connect(state->chat_toggle_button, "clicked", G_CALLBACK(on_chat_toggle_clicked), state);
    g_signal_connect(state->volume_scale, "value-changed", G_CALLBACK(on_volume_changed), state);

    return box;
}

static char *extract_twitch_channel(const char *target)
{
    if (target == NULL || target[0] == '\0') {
        return NULL;
    }

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

static gboolean target_matches_stream_values(
    const char *target,
    const char *target_channel,
    const char *label,
    const char *channel,
    const char *url
)
{
    if (target == NULL || target[0] == '\0') {
        return FALSE;
    }

    return g_ascii_strcasecmp(target, label) == 0 ||
        g_ascii_strcasecmp(target, channel) == 0 ||
        g_ascii_strcasecmp(target, url) == 0 ||
        (target_channel != NULL && g_ascii_strcasecmp(target_channel, channel) == 0);
}

static void init_streams(SinglePlayer *state, const char *target)
{
    guint settings_count = app_settings_get_channel_count(state->settings);
    guint base_count = settings_count;
    guint extra_count = 0;
    guint active_stream = 0;
    gboolean startup_known = FALSE;
    g_autofree char *startup_channel = extract_twitch_channel(target);
    char *extra_label = NULL;
    char *extra_channel = NULL;
    char *extra_url = NULL;

    if (target != NULL && target[0] != '\0') {
        for (guint i = 0; i < base_count; i++) {
            const AppSettingsChannel *channel = app_settings_get_channel(state->settings, i);
            if (target_matches_stream_values(target, startup_channel, channel->label, channel->channel, channel->url)) {
                active_stream = i;
                startup_known = TRUE;
                break;
            }
        }
    }

    if (!startup_known && target != NULL && target[0] != '\0') {
        extra_count = 1;

        if (startup_channel != NULL) {
            extra_label = g_strdup(startup_channel);
            extra_channel = g_strdup(startup_channel);
        } else if (g_str_has_prefix(target, "http://") || g_str_has_prefix(target, "https://")) {
            extra_label = g_strdup(target);
        } else {
            extra_label = g_strdup(target);
            extra_channel = g_ascii_strdown(target, -1);
        }

        if (g_str_has_prefix(target, "http://") || g_str_has_prefix(target, "https://")) {
            extra_url = g_strdup(target);
        } else {
            extra_url = g_strdup_printf("https://www.twitch.tv/%s", target);
        }

        active_stream = base_count;
    }

    state->stream_count = base_count + extra_count;
    state->streams = g_new0(StreamEntry, state->stream_count);

    for (guint i = 0; i < base_count; i++) {
        const AppSettingsChannel *channel = app_settings_get_channel(state->settings, i);
        state->streams[i].label = g_strdup(channel->label);
        state->streams[i].channel = g_strdup(channel->channel);
        state->streams[i].url = g_strdup(channel->url);
    }

    if (extra_count == 1) {
        state->streams[base_count].label = extra_label;
        state->streams[base_count].channel = extra_channel;
        state->streams[base_count].url = extra_url;
    }

    state->active_stream = active_stream;
}

static void free_streams(SinglePlayer *state)
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

static void maybe_start_initial_stream(SinglePlayer *state)
{
    if (state->startup_target == NULL || state->startup_target[0] == '\0') {
        return;
    }

    play_selected_stream(state);
}

static void single_player_destroy(SinglePlayer *state)
{
    if (state == NULL) {
        return;
    }

    state->closing = TRUE;

    remove_source_if_active(&state->footer_hide_source);

    remove_source_if_active(&state->title_refresh_source);

    if (state->title_cancel != NULL) {
        g_cancellable_cancel(state->title_cancel);
        g_clear_object(&state->title_cancel);
    }

    remove_source_if_active(&state->chat_position_source);
    remove_source_if_active(&state->render_warmup_source);

    clear_mpv_render_context(state);

    player_session_set_wakeup_callback(state->session, NULL, NULL);
    state->session = NULL;

    if (GTK_IS_PANED(state->main_area)) {
        int position = gtk_paned_get_position(GTK_PANED(state->main_area));
        if (position > 0) {
            state->chat_paned_position = position;
        }
        gtk_paned_set_end_child(GTK_PANED(state->main_area), NULL);
    }

    chat_panel_free(state->chat_panel);
    state->chat_panel = NULL;
    state->gl_area = NULL;
    state->video_overlay = NULL;
    state->main_area = NULL;
    state->chat_toggle_button = NULL;
    state->bottom_panel = NULL;
    state->stream_combo = NULL;
    state->stream_title_label = NULL;
    state->volume_scale = NULL;
    state->status_label = NULL;
    free_streams(state);
    state->settings = NULL;
}

SinglePlayer *single_player_new(
    GtkWindow *window,
    AppSettings *settings,
    PlayerSession *session,
    const char *startup_target,
    gboolean auto_start,
    int chat_paned_position,
    SinglePlayerFullscreenCallback fullscreen_callback,
    gpointer fullscreen_user_data
)
{
    SinglePlayer *state = g_new0(SinglePlayer, 1);
    state->startup_target = startup_target;
    state->session = session;
    state->window = GTK_WIDGET(window);
    state->chat_paned_position = chat_paned_position;
    state->active_stream = 0;
    state->chat_visible = FALSE;
    state->settings = settings;
    state->fullscreen_callback = fullscreen_callback;
    state->fullscreen_user_data = fullscreen_user_data;
    const char *session_target = player_session_get_url(state->session);
    init_streams(state, session_target != NULL && session_target[0] != '\0' ? session_target : state->startup_target);
    state->stream_playing = player_session_is_playing(state->session);

    state->main_area = gtk_paned_new(GTK_ORIENTATION_HORIZONTAL);
    g_object_add_weak_pointer(G_OBJECT(state->main_area), (gpointer *)&state->main_area);
    gtk_widget_add_css_class(state->main_area, "main-area");
    gtk_widget_set_hexpand(state->main_area, TRUE);
    gtk_widget_set_vexpand(state->main_area, TRUE);
    gtk_paned_set_wide_handle(GTK_PANED(state->main_area), FALSE);
    gtk_paned_set_resize_start_child(GTK_PANED(state->main_area), TRUE);
    gtk_paned_set_shrink_start_child(GTK_PANED(state->main_area), FALSE);
    gtk_paned_set_resize_end_child(GTK_PANED(state->main_area), FALSE);
    gtk_paned_set_shrink_end_child(GTK_PANED(state->main_area), FALSE);

    state->gl_area = gtk_gl_area_new();
    g_object_add_weak_pointer(G_OBJECT(state->gl_area), (gpointer *)&state->gl_area);
    gtk_gl_area_set_auto_render(GTK_GL_AREA(state->gl_area), FALSE);
    gtk_widget_set_hexpand(state->gl_area, TRUE);
    gtk_widget_set_vexpand(state->gl_area, TRUE);

    state->video_overlay = gtk_overlay_new();
    g_object_add_weak_pointer(G_OBJECT(state->video_overlay), (gpointer *)&state->video_overlay);
    gtk_widget_set_hexpand(state->video_overlay, TRUE);
    gtk_widget_set_vexpand(state->video_overlay, TRUE);
    gtk_overlay_set_child(GTK_OVERLAY(state->video_overlay), state->gl_area);
    gtk_paned_set_start_child(GTK_PANED(state->main_area), state->video_overlay);

    state->chat_panel = chat_panel_new(DEFAULT_CHAT_WIDTH);

    GtkGesture *video_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(video_click), GDK_BUTTON_PRIMARY);
    g_signal_connect(video_click, "pressed", G_CALLBACK(on_video_pressed), state);
    gtk_widget_add_controller(state->gl_area, GTK_EVENT_CONTROLLER(video_click));

    GtkEventController *video_legacy = gtk_event_controller_legacy_new();
    g_signal_connect(video_legacy, "event", G_CALLBACK(on_video_legacy_event), state);
    gtk_widget_add_controller(state->gl_area, video_legacy);

    GtkEventController *video_scroll = gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_VERTICAL);
    gtk_event_controller_set_propagation_phase(video_scroll, GTK_PHASE_CAPTURE);
    g_signal_connect(video_scroll, "scroll", G_CALLBACK(on_video_scroll), state);
    gtk_widget_add_controller(state->video_overlay, video_scroll);

    state->bottom_panel = create_controls(state);
    gtk_widget_set_halign(state->bottom_panel, GTK_ALIGN_FILL);
    gtk_widget_set_valign(state->bottom_panel, GTK_ALIGN_END);
    gtk_overlay_add_overlay(GTK_OVERLAY(state->video_overlay), state->bottom_panel);
    state->title_refresh_source = g_timeout_add_seconds(STREAM_TITLE_REFRESH_SECONDS, refresh_stream_title, state);

    GtkEventController *video_motion = gtk_event_controller_motion_new();
    gtk_event_controller_set_propagation_phase(video_motion, GTK_PHASE_CAPTURE);
    g_signal_connect(video_motion, "motion", G_CALLBACK(on_video_motion), state);
    gtk_widget_add_controller(state->video_overlay, video_motion);

    if (!player_session_is_ready(state->session)) {
        set_status(state, "mpv could not be initialized");
        gtk_widget_set_sensitive(state->stream_combo, FALSE);
    } else {
        player_session_set_wakeup_callback(state->session, on_mpv_wakeup, state);
    }

    g_signal_connect(state->gl_area, "realize", G_CALLBACK(on_gl_realize), state);
    g_signal_connect(state->gl_area, "unrealize", G_CALLBACK(on_gl_unrealize), state);
    g_signal_connect(state->gl_area, "render", G_CALLBACK(on_gl_render), state);

    schedule_footer_hide(state);

    if (player_session_is_playing(state->session)) {
        update_stream_combo_label(state);
        set_status(state, "Playback running");
        start_chat(state, get_active_stream_channel(state));
        request_stream_title_update(state, TRUE);
    } else if (player_session_is_ready(state->session) && auto_start) {
        maybe_start_initial_stream(state);
    }

    return state;
}

GtkWidget *single_player_get_widget(SinglePlayer *player)
{
    return player != NULL && GTK_IS_WIDGET(player->main_area) ? player->main_area : NULL;
}

char *single_player_dup_current_target(SinglePlayer *player)
{
    if (player == NULL || !player_session_is_playing(player->session)) {
        return NULL;
    }

    return player_session_dup_url(player->session);
}

int single_player_get_chat_paned_position(SinglePlayer *player)
{
    if (player == NULL) {
        return 0;
    }

    if (GTK_IS_PANED(player->main_area)) {
        int position = gtk_paned_get_position(GTK_PANED(player->main_area));
        if (position > 0) {
            player->chat_paned_position = position;
        }
    }

    return player->chat_paned_position;
}

void single_player_set_fullscreen(SinglePlayer *player, gboolean fullscreen)
{
    if (player != NULL) {
        player->fullscreen = fullscreen;
    }
}

void single_player_show_overlay(SinglePlayer *player)
{
    if (player != NULL) {
        show_footer(player);
    }
}

gboolean single_player_handle_key(SinglePlayer *player, guint keyval, GdkModifierType modifiers)
{
    if (player == NULL || (modifiers & GDK_CONTROL_MASK) != 0) {
        return GDK_EVENT_PROPAGATE;
    }

    if (keyval != GDK_KEY_m && keyval != GDK_KEY_M) {
        return GDK_EVENT_PROPAGATE;
    }

    toggle_mute(player);
    show_footer(player);
    return GDK_EVENT_STOP;
}

void single_player_set_settings(SinglePlayer *player, AppSettings *settings)
{
    if (player == NULL) {
        return;
    }

    g_autofree char *current_channel = NULL;
    if (player->active_stream < player->stream_count) {
        current_channel = g_strdup(player->streams[player->active_stream].channel);
    }

    player->settings = settings;
    free_streams(player);
    init_streams(player, current_channel);
    rebuild_stream_menu(player);
    show_footer(player);
}

void single_player_free(SinglePlayer *player)
{
    single_player_destroy(player);
    /* mpv may still have queued idle callbacks that carry this pointer. */
}
