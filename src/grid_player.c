#define G_LOG_DOMAIN "twitch-player-grid"

#include <gtk/gtk.h>
#include <epoxy/egl.h>
#include <epoxy/gl.h>
#include <mpv/render_gl.h>
#include <math.h>
#include <string.h>

#include "grid_player.h"
#include "player_icons.h"
#include "player_motion.h"
#include "player_defaults.h"
#include "player_volume.h"

#define MAX_TILES GRID_PLAYER_MAX_TILES
#define MPV_MAINLOOP_PRIORITY G_PRIORITY_HIGH
typedef struct _GridAppState GridAppState;

typedef struct {
    GridAppState *app;
    guint index;
    char *label;
    char *url;
    GtkWidget *container;
    GtkWidget *overlay;
    GtkWidget *gl_area;
    GtkWidget *footer;
    GtkWidget *channel_combo;
    GtkWidget *channel_label;
    GtkWidget *close_button;
    GtkWidget *empty_label;
    GtkWidget *focus_button;
    GtkWidget *stream_info_button;
    GtkWidget *volume_scale;
    PlayerSession *session;
    mpv_render_context *mpv_gl;
    int last_render_width;
    int last_render_height;
    gint render_queued;
    gint event_queued;
    guint render_warmup_source;
    int render_warmup_frames;
    gboolean owns_session;
} StreamTile;

struct _GridAppState {
    char *targets[MAX_TILES];
    guint target_count;
    GtkWidget *window;
    GtkWidget *root_overlay;
    GtkWidget *grid;
    GtkWidget *grid_items[MAX_TILES];
    GtkWidget *top_controls;
    StreamTile tiles[MAX_TILES];
    PlayerSession *primary_session;
    AppSettings *settings;
    StreamTile *visible_footer_tile;
    guint footer_hide_source;
    guint video_fullscreen_focus_source;
    guint focused_tile;
    guint video_fullscreen_pending_tile;
    PlayerMotionTracker motion_tracker;
    GridPlayerFullscreenCallback fullscreen_callback;
    gpointer fullscreen_user_data;
    double move_press_x;
    double move_press_y;
    gboolean move_pressed;
    gboolean closing;
    gboolean fullscreen;
    gboolean tile_focused;
    gboolean video_fullscreen_active;
    gboolean video_fullscreen_restore_app_fullscreen;
    gboolean video_fullscreen_restore_tile_focused;
    guint video_fullscreen_restore_focused_tile;
    gboolean started;
};

static gboolean create_mpv_render_context(StreamTile *tile);
static void schedule_footer_hide(GridAppState *state);
static void show_tile_overlay(StreamTile *tile);

static void set_tile_status(StreamTile *tile, const char *message)
{
    (void)tile;
    (void)message;
}

static void check_mpv(int status, const char *action)
{
    if (status < 0) {
        g_warning("%s: %s", action, mpv_error_string(status));
    }
}

static mpv_handle *tile_mpv(StreamTile *tile)
{
    return player_session_get_mpv(tile->session);
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
    StreamTile *tile = user_data;

    g_atomic_int_set(&tile->render_queued, 0);

    if (!tile->app->closing && tile->gl_area != NULL) {
        gtk_gl_area_queue_render(GTK_GL_AREA(tile->gl_area));
    }

    return G_SOURCE_REMOVE;
}

static gboolean warmup_tile_render(gpointer user_data)
{
    StreamTile *tile = user_data;

    if (tile->app->closing || tile->gl_area == NULL || tile->render_warmup_frames <= 0) {
        tile->render_warmup_source = 0;
        return G_SOURCE_REMOVE;
    }

    tile->render_warmup_frames--;
    gtk_gl_area_queue_render(GTK_GL_AREA(tile->gl_area));
    return G_SOURCE_CONTINUE;
}

static void start_render_warmup(StreamTile *tile)
{
    remove_source_if_active(&tile->render_warmup_source);
    tile->render_warmup_frames = 90;
    tile->render_warmup_source = g_timeout_add(16, warmup_tile_render, tile);
}

static void on_mpv_render_update(void *ctx)
{
    StreamTile *tile = ctx;

    if (g_atomic_int_compare_and_exchange(&tile->render_queued, 0, 1)) {
        g_idle_add_full(MPV_MAINLOOP_PRIORITY, queue_mpv_render, tile, NULL);
    }
}

static gboolean process_mpv_events(gpointer user_data)
{
    StreamTile *tile = user_data;

    g_atomic_int_set(&tile->event_queued, 0);

    mpv_handle *mpv = tile_mpv(tile);
    if (tile->app->closing || mpv == NULL) {
        return G_SOURCE_REMOVE;
    }

    while (TRUE) {
        mpv_event *event = mpv_wait_event(mpv, 0);

        if (event->event_id == MPV_EVENT_NONE) {
            break;
        }

        switch (event->event_id) {
        case MPV_EVENT_START_FILE:
            set_tile_status(tile, "Loading");
            break;
        case MPV_EVENT_FILE_LOADED:
            set_tile_status(tile, "Playback running");
            break;
        case MPV_EVENT_END_FILE: {
            mpv_event_end_file *end = event->data;
            if (end != NULL && end->reason == MPV_END_FILE_REASON_ERROR) {
                set_tile_status(tile, "Stream could not be played");
            } else {
                set_tile_status(tile, "Stopped");
            }
            break;
        }
        case MPV_EVENT_VIDEO_RECONFIG:
            /* After a Twitch ad the stream resumes with a huge PTS jump, causing
             * an internal mpv playback reset. The video output needs a warmup so
             * the GL renderer actively polls for new frames again. */
            start_render_warmup(tile);
            break;
        case MPV_EVENT_LOG_MESSAGE: {
            mpv_event_log_message *log = event->data;
            if (log != NULL && log->prefix != NULL && log->text != NULL) {
                g_debug("mpv[%s][%u]: %s", log->prefix, tile->index, log->text);
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
    StreamTile *tile = ctx;

    if (g_atomic_int_compare_and_exchange(&tile->event_queued, 0, 1)) {
        g_idle_add_full(MPV_MAINLOOP_PRIORITY, process_mpv_events, tile, NULL);
    }
}

static char *target_to_label(const char *target)
{
    if (target == NULL || target[0] == '\0') {
        return NULL;
    }

    const char *prefix = "twitch.tv/";
    const char *channel = strstr(target, prefix);

    if (channel != NULL) {
        channel += strlen(prefix);
    } else {
        channel = target;
    }

    while (*channel == '/') {
        channel++;
    }

    const char *end = channel;
    while (g_ascii_isalnum(*end) || *end == '_') {
        end++;
    }

    if (end > channel) {
        return g_strndup(channel, end - channel);
    }

    return g_strdup(target);
}

static char *target_to_url(const char *target)
{
    if (target == NULL || target[0] == '\0') {
        return NULL;
    }

    if (g_str_has_prefix(target, "http://") || g_str_has_prefix(target, "https://")) {
        return g_strdup(target);
    }

    return g_strdup_printf("https://www.twitch.tv/%s", target);
}

static void update_tile_channel_label(StreamTile *tile)
{
    if (tile->channel_label == NULL) {
        return;
    }

    const char *label = tile->label != NULL && tile->label[0] != '\0' ? tile->label : PLAYER_EMPTY_STREAM_LABEL;
    gtk_label_set_text(GTK_LABEL(tile->channel_label), label);
    gtk_widget_set_tooltip_text(tile->channel_label, tile->label != NULL && tile->label[0] != '\0' ? tile->label : NULL);
}

static void sync_tile_from_session(StreamTile *tile)
{
    if (!player_session_is_playing(tile->session)) {
        return;
    }

    const char *label = player_session_get_label(tile->session);
    const char *url = player_session_get_url(tile->session);

    g_free(tile->label);
    g_free(tile->url);
    tile->label = g_strdup(label != NULL && label[0] != '\0' ? label : url);
    tile->url = g_strdup(url);
}

static void update_tile_empty_state(StreamTile *tile)
{
    gboolean has_stream = tile->url != NULL && tile->url[0] != '\0';

    if (tile->empty_label != NULL) {
        gtk_widget_set_visible(tile->empty_label, !has_stream);
    }
    if (tile->close_button != NULL) {
        gtk_widget_set_sensitive(tile->close_button, has_stream);
    }
    if (tile->stream_info_button != NULL) {
        gtk_widget_set_sensitive(tile->stream_info_button, has_stream && player_session_is_ready(tile->session));
    }
    if (tile->volume_scale != NULL) {
        gtk_widget_set_sensitive(tile->volume_scale, has_stream && player_session_is_ready(tile->session));
    }

    update_tile_channel_label(tile);
}

static void load_tile_stream(StreamTile *tile)
{
    if (!player_session_is_ready(tile->session) || tile->url == NULL) {
        return;
    }

    set_tile_status(tile, PLAYER_STARTING_STREAM_STATUS);
    player_session_load_stream(tile->session, tile->url, tile->label, NULL);
}

static void clear_tile_render_context(StreamTile *tile)
{
    if (tile->gl_area != NULL && gtk_widget_get_realized(tile->gl_area)) {
        gtk_gl_area_make_current(GTK_GL_AREA(tile->gl_area));
    }

    if (tile->mpv_gl != NULL) {
        mpv_render_context_set_update_callback(tile->mpv_gl, NULL, NULL);
        mpv_render_context_free(tile->mpv_gl);
        tile->mpv_gl = NULL;
    }
    remove_source_if_active(&tile->render_warmup_source);
    tile->last_render_width = 0;
    tile->last_render_height = 0;
    tile->render_warmup_frames = 0;
}

static void reset_owned_tile_session(StreamTile *tile)
{
    clear_tile_render_context(tile);
    player_session_set_wakeup_callback(tile->session, NULL, NULL);
    if (tile->owns_session) {
        player_session_free(tile->session);
        tile->session = player_session_new();
    } else {
        player_session_stop(tile->session);
    }
}

static void stop_tile_stream(StreamTile *tile)
{
    reset_owned_tile_session(tile);
    g_clear_pointer(&tile->label, g_free);
    g_clear_pointer(&tile->url, g_free);
    update_tile_empty_state(tile);

    if (tile->gl_area != NULL) {
        gtk_gl_area_queue_render(GTK_GL_AREA(tile->gl_area));
    }
}

static gboolean ensure_tile_session(StreamTile *tile)
{
    if (tile->session == NULL) {
        tile->session = player_session_new();
        tile->owns_session = TRUE;
    }

    if (!player_session_is_ready(tile->session)) {
        update_tile_empty_state(tile);
        return FALSE;
    }

    player_session_set_wakeup_callback(tile->session, on_mpv_wakeup, tile);
    if (tile->gl_area != NULL && gtk_widget_get_realized(tile->gl_area) && !create_mpv_render_context(tile)) {
        update_tile_empty_state(tile);
        return FALSE;
    }

    update_tile_empty_state(tile);
    return TRUE;
}

static void set_tile_channel(StreamTile *tile, const AppSettingsChannel *channel)
{
    if (channel == NULL || channel->url == NULL || channel->url[0] == '\0') {
        return;
    }

    g_free(tile->label);
    g_free(tile->url);
    tile->label = g_strdup(channel->label);
    tile->url = g_strdup(channel->url);

    if (!ensure_tile_session(tile)) {
        return;
    }

    load_tile_stream(tile);
}

static void on_volume_changed(GtkRange *range, gpointer user_data)
{
    StreamTile *tile = user_data;

    player_volume_sync_session_from_range(tile->session, range);
}

static GtkWidget *create_overlay_button(GtkWidget *icon, const char *tooltip)
{
    GtkWidget *button = gtk_button_new();
    gtk_button_set_child(GTK_BUTTON(button), icon);
    gtk_widget_add_css_class(button, "overlay-icon-button");
    gtk_widget_set_tooltip_text(button, tooltip);
    return button;
}

static void on_tile_close_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    StreamTile *tile = user_data;

    stop_tile_stream(tile);
    show_tile_overlay(tile);
}

static void on_stream_info_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    StreamTile *tile = user_data;

    mpv_handle *mpv = tile_mpv(tile);
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

    show_tile_overlay(tile);
}

static void on_channel_menu_clicked(GtkButton *button, gpointer user_data)
{
    StreamTile *tile = user_data;
    gpointer index_data = g_object_get_data(G_OBJECT(button), "channel-index");
    guint index = GPOINTER_TO_UINT(index_data);
    const AppSettingsChannel *channel = app_settings_get_channel(tile->app->settings, index);

    set_tile_channel(tile, channel);

    GtkPopover *popover = gtk_menu_button_get_popover(GTK_MENU_BUTTON(tile->channel_combo));
    if (popover != NULL) {
        gtk_popover_popdown(popover);
    }

    show_tile_overlay(tile);
}

static GtkWidget *create_channel_menu(StreamTile *tile)
{
    GtkWidget *menu = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class(menu, "channel-menu");
    gtk_widget_set_size_request(menu, 170, -1);

    guint channel_count = app_settings_get_channel_count(tile->app->settings);
    if (channel_count == 0) {
        GtkWidget *item = gtk_button_new_with_label("No channels");
        gtk_widget_add_css_class(item, "channel-menu-item");
        gtk_widget_set_sensitive(item, FALSE);
        gtk_box_append(GTK_BOX(menu), item);
        return menu;
    }

    for (guint i = 0; i < channel_count; i++) {
        const AppSettingsChannel *channel = app_settings_get_channel(tile->app->settings, i);
        GtkWidget *item = gtk_button_new();
        GtkWidget *item_content = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
        GtkWidget *item_label = gtk_label_new(channel != NULL ? channel->label : "");

        gtk_label_set_xalign(GTK_LABEL(item_label), 0.0);
        gtk_widget_set_halign(item_label, GTK_ALIGN_START);
        gtk_widget_set_hexpand(item_label, TRUE);
        gtk_box_append(GTK_BOX(item_content), item_label);
        gtk_button_set_child(GTK_BUTTON(item), item_content);
        gtk_widget_add_css_class(item, "channel-menu-item");
        gtk_widget_set_halign(item, GTK_ALIGN_FILL);
        gtk_widget_set_hexpand(item, TRUE);
        g_object_set_data(G_OBJECT(item), "channel-index", GUINT_TO_POINTER(i));
        g_signal_connect(item, "clicked", G_CALLBACK(on_channel_menu_clicked), tile);
        gtk_box_append(GTK_BOX(menu), item);
    }

    return menu;
}

static void rebuild_tile_channel_popover(StreamTile *tile)
{
    GtkWidget *channel_popover = gtk_popover_new();
    gtk_widget_add_css_class(channel_popover, "channel-popover");
    gtk_popover_set_position(GTK_POPOVER(channel_popover), GTK_POS_TOP);
    gtk_popover_set_has_arrow(GTK_POPOVER(channel_popover), FALSE);
    gtk_popover_set_child(GTK_POPOVER(channel_popover), create_channel_menu(tile));
    gtk_menu_button_set_popover(GTK_MENU_BUTTON(tile->channel_combo), channel_popover);
    gtk_widget_set_sensitive(tile->channel_combo, app_settings_get_channel_count(tile->app->settings) > 0);
}

static gboolean is_channel_menu_open(StreamTile *tile)
{
    if (tile->channel_combo == NULL) {
        return FALSE;
    }

    GtkPopover *popover = gtk_menu_button_get_popover(GTK_MENU_BUTTON(tile->channel_combo));
    return popover != NULL && gtk_widget_get_mapped(GTK_WIDGET(popover));
}

static gboolean hide_footers(gpointer user_data)
{
    GridAppState *state = user_data;

    state->footer_hide_source = 0;

    for (guint i = 0; i < MAX_TILES; i++) {
        if (is_channel_menu_open(&state->tiles[i])) {
            schedule_footer_hide(state);
            return G_SOURCE_REMOVE;
        }
    }

    state->visible_footer_tile = NULL;

    if (!state->closing) {
        if (state->top_controls != NULL) {
            gtk_widget_set_visible(state->top_controls, FALSE);
        }
        for (guint i = 0; i < MAX_TILES; i++) {
            if (state->tiles[i].footer != NULL) {
                gtk_widget_set_visible(state->tiles[i].footer, FALSE);
            }
        }
    }

    return G_SOURCE_REMOVE;
}

static void schedule_footer_hide(GridAppState *state)
{
    remove_source_if_active(&state->footer_hide_source);

    state->footer_hide_source = g_timeout_add(1800, hide_footers, state);
}

static void show_tile_overlay(StreamTile *tile)
{
    GridAppState *state = tile->app;

    if (state->closing) {
        return;
    }

    if (state->top_controls != NULL) {
        gtk_widget_set_visible(state->top_controls, TRUE);
    }
    for (guint i = 0; i < MAX_TILES; i++) {
        if (state->tiles[i].footer != NULL) {
            gtk_widget_set_visible(state->tiles[i].footer, &state->tiles[i] == tile);
        }
    }
    state->visible_footer_tile = tile;

    schedule_footer_hide(state);
}

static GtkGridLayoutChild *get_grid_layout_child(GridAppState *state, GtkWidget *child)
{
    GtkLayoutManager *layout = gtk_widget_get_layout_manager(state->grid);
    GtkLayoutChild *layout_child = gtk_layout_manager_get_layout_child(layout, child);

    if (!GTK_IS_GRID_LAYOUT_CHILD(layout_child)) {
        return NULL;
    }

    return GTK_GRID_LAYOUT_CHILD(layout_child);
}

static void set_grid_item_layout(GridAppState *state, GtkWidget *widget, int column, int row, int column_span, int row_span)
{
    GtkGridLayoutChild *child = get_grid_layout_child(state, widget);

    if (child == NULL) {
        return;
    }

    gtk_grid_layout_child_set_column(child, column);
    gtk_grid_layout_child_set_row(child, row);
    gtk_grid_layout_child_set_column_span(child, column_span);
    gtk_grid_layout_child_set_row_span(child, row_span);
}

static void restore_grid_layout(GridAppState *state)
{
    for (guint i = 0; i < MAX_TILES; i++) {
        if (state->grid_items[i] == NULL) {
            continue;
        }

        set_grid_item_layout(state, state->grid_items[i], i % 2, i / 2, 1, 1);
        gtk_widget_set_visible(state->grid_items[i], TRUE);
    }

    state->tile_focused = FALSE;
}

static gboolean is_tile_focused(StreamTile *tile)
{
    GridAppState *state = tile->app;

    return state->tile_focused && state->focused_tile == tile->index;
}

static void update_tile_focus_buttons(GridAppState *state)
{
    for (guint i = 0; i < MAX_TILES; i++) {
        StreamTile *tile = &state->tiles[i];
        if (tile->focus_button == NULL) {
            continue;
        }

        gboolean focused = is_tile_focused(tile);
        gtk_button_set_child(
            GTK_BUTTON(tile->focus_button),
            player_tile_focus_icon_new(focused ? PLAYER_TILE_FOCUS_ICON_RESTORE : PLAYER_TILE_FOCUS_ICON_EXPAND)
        );
        gtk_widget_set_tooltip_text(tile->focus_button, focused ? "Restore grid" : "Focus tile");
    }
}

static void focus_tile(StreamTile *tile)
{
    GridAppState *state = tile->app;

    for (guint i = 0; i < MAX_TILES; i++) {
        if (state->grid_items[i] != NULL) {
            gtk_widget_set_visible(state->grid_items[i], i == tile->index);
        }
    }

    set_grid_item_layout(state, tile->container, 0, 0, 2, 2);
    state->focused_tile = tile->index;
    state->tile_focused = TRUE;
}

static void toggle_tile_focus(StreamTile *tile)
{
    GridAppState *state = tile->app;

    if (state->tile_focused && state->focused_tile == tile->index) {
        restore_grid_layout(state);
    } else {
        focus_tile(tile);
    }

    update_tile_focus_buttons(state);
    show_tile_overlay(tile);
}

static gboolean apply_pending_video_fullscreen_focus(gpointer user_data)
{
    GridAppState *state = user_data;

    state->video_fullscreen_focus_source = 0;

    if (state->closing || state->video_fullscreen_pending_tile >= MAX_TILES) {
        return G_SOURCE_REMOVE;
    }

    StreamTile *tile = &state->tiles[state->video_fullscreen_pending_tile];
    if (!is_tile_focused(tile)) {
        focus_tile(tile);
        update_tile_focus_buttons(state);
    }
    show_tile_overlay(tile);

    return G_SOURCE_REMOVE;
}

static void schedule_video_fullscreen_focus(StreamTile *tile)
{
    GridAppState *state = tile->app;

    remove_source_if_active(&state->video_fullscreen_focus_source);
    state->video_fullscreen_pending_tile = tile->index;
    state->video_fullscreen_focus_source = g_timeout_add(50, apply_pending_video_fullscreen_focus, state);
}

static void request_tile_fullscreen_toggle(StreamTile *tile)
{
    GridAppState *state = tile->app;

    if (state->video_fullscreen_active) {
        remove_source_if_active(&state->video_fullscreen_focus_source);

        if (!state->video_fullscreen_restore_app_fullscreen &&
            state->fullscreen &&
            state->fullscreen_callback != NULL) {
            state->fullscreen_callback(state->fullscreen_user_data);
        }

        if (state->video_fullscreen_restore_tile_focused &&
            state->video_fullscreen_restore_focused_tile < MAX_TILES &&
            state->grid_items[state->video_fullscreen_restore_focused_tile] != NULL) {
            focus_tile(&state->tiles[state->video_fullscreen_restore_focused_tile]);
        } else {
            restore_grid_layout(state);
        }
        state->video_fullscreen_active = FALSE;
        update_tile_focus_buttons(state);
        show_tile_overlay(tile);
        return;
    }

    state->video_fullscreen_restore_app_fullscreen = state->fullscreen;
    state->video_fullscreen_restore_tile_focused = state->tile_focused;
    state->video_fullscreen_restore_focused_tile = state->focused_tile;
    state->video_fullscreen_active = TRUE;

    if (!state->fullscreen && state->fullscreen_callback != NULL) {
        state->fullscreen_callback(state->fullscreen_user_data);
    }

    schedule_video_fullscreen_focus(tile);
}

static void on_tile_focus_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    toggle_tile_focus(user_data);
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

static void begin_window_move_from_event(GridAppState *state, GdkEvent *event, guint button)
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

static void on_tile_motion(GtkEventControllerMotion *controller, double x, double y, gpointer user_data)
{
    (void)controller;
    StreamTile *tile = user_data;
    GridAppState *state = tile->app;

    if (player_motion_tracker_ignore_stationary(&state->motion_tracker, tile, x, y)) {
        return;
    }

    show_tile_overlay(tile);
}

static void on_video_pressed(GtkGestureClick *gesture, int n_press, double x, double y, gpointer user_data)
{
    (void)gesture;
    (void)x;
    (void)y;

    if (n_press == 2) {
        request_tile_fullscreen_toggle(user_data);
    }
}

static gboolean on_video_legacy_event(GtkEventControllerLegacy *controller, GdkEvent *event, gpointer user_data)
{
    (void)controller;
    StreamTile *tile = user_data;
    GridAppState *state = tile->app;
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

static gboolean on_tile_scroll(GtkEventControllerScroll *controller, double dx, double dy, gpointer user_data)
{
    (void)controller;
    StreamTile *tile = user_data;

    if (tile->volume_scale == NULL ||
        !gtk_widget_get_sensitive(tile->volume_scale) ||
        !player_volume_apply_scroll(tile->volume_scale, dx, dy)) {
        return GDK_EVENT_PROPAGATE;
    }

    show_tile_overlay(tile);

    return GDK_EVENT_STOP;
}

static gboolean on_gl_render(GtkGLArea *area, GdkGLContext *context, gpointer user_data)
{
    (void)context;
    StreamTile *tile = user_data;

    if (tile->mpv_gl == NULL) {
        gtk_gl_area_attach_buffers(area);
        glClearColor(0.02f, 0.02f, 0.02f, 1.0f);
        glClear(GL_COLOR_BUFFER_BIT);
        return TRUE;
    }

    int scale = gtk_widget_get_scale_factor(GTK_WIDGET(area));
    int width = gtk_widget_get_width(GTK_WIDGET(area)) * scale;
    int height = gtk_widget_get_height(GTK_WIDGET(area)) * scale;

    if (width <= 0 || height <= 0) {
        return TRUE;
    }

    uint64_t update_flags = mpv_render_context_update(tile->mpv_gl);
    gboolean size_changed = width != tile->last_render_width || height != tile->last_render_height;
    gboolean warming_up = tile->render_warmup_frames > 0;

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

    int status = mpv_render_context_render(tile->mpv_gl, params);
    if (status < 0) {
        g_warning("mpv render: %s", mpv_error_string(status));
    } else {
        tile->last_render_width = width;
        tile->last_render_height = height;
    }

    return TRUE;
}

static gboolean create_mpv_render_context(StreamTile *tile)
{
    mpv_handle *mpv = tile_mpv(tile);
    if (mpv == NULL || tile->gl_area == NULL) {
        return FALSE;
    }

    gtk_gl_area_make_current(GTK_GL_AREA(tile->gl_area));

    if (gtk_gl_area_get_error(GTK_GL_AREA(tile->gl_area)) != NULL) {
        g_warning("GTK GL area error: %s", gtk_gl_area_get_error(GTK_GL_AREA(tile->gl_area))->message);
        return FALSE;
    }

    if (tile->mpv_gl != NULL) {
        mpv_render_context_set_update_callback(tile->mpv_gl, NULL, NULL);
        mpv_render_context_free(tile->mpv_gl);
        tile->mpv_gl = NULL;
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

    int status = mpv_render_context_create(&tile->mpv_gl, mpv, params);
    if (status < 0) {
        g_warning("mpv render context: %s", mpv_error_string(status));
        return FALSE;
    }

    mpv_render_context_set_update_callback(tile->mpv_gl, on_mpv_render_update, tile);
    player_session_reenable_video(tile->session);
    start_render_warmup(tile);
    gtk_gl_area_queue_render(GTK_GL_AREA(tile->gl_area));
    return TRUE;
}

static void on_gl_realize(GtkGLArea *area, gpointer user_data)
{
    (void)area;
    StreamTile *tile = user_data;

    if (tile_mpv(tile) != NULL && !create_mpv_render_context(tile)) {
        set_tile_status(tile, "Render error");
    }
}

static void on_gl_unrealize(GtkGLArea *area, gpointer user_data)
{
    StreamTile *tile = user_data;

    gtk_gl_area_make_current(area);

    if (tile->mpv_gl != NULL) {
        mpv_render_context_set_update_callback(tile->mpv_gl, NULL, NULL);
        mpv_render_context_free(tile->mpv_gl);
        tile->mpv_gl = NULL;
    }
}

static GtkWidget *create_tile_footer(StreamTile *tile)
{
    GtkWidget *box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_add_css_class(box, "tile-footer");

    tile->channel_combo = gtk_menu_button_new();
    gtk_widget_add_css_class(tile->channel_combo, "channel-dropdown");
    tile->channel_label = gtk_label_new("");
    gtk_widget_add_css_class(tile->channel_label, "channel-button-label");
    gtk_widget_set_halign(tile->channel_label, GTK_ALIGN_START);
    gtk_label_set_xalign(GTK_LABEL(tile->channel_label), 0.0);
    gtk_label_set_ellipsize(GTK_LABEL(tile->channel_label), PANGO_ELLIPSIZE_END);
    gtk_menu_button_set_child(GTK_MENU_BUTTON(tile->channel_combo), tile->channel_label);
    gtk_menu_button_set_direction(GTK_MENU_BUTTON(tile->channel_combo), GTK_ARROW_UP);
    gtk_menu_button_set_always_show_arrow(GTK_MENU_BUTTON(tile->channel_combo), FALSE);
    gtk_widget_set_size_request(tile->channel_combo, 170, -1);
    gtk_widget_set_hexpand(tile->channel_combo, FALSE);

    rebuild_tile_channel_popover(tile);

    tile->close_button = create_overlay_button(player_window_icon_new(PLAYER_WINDOW_ICON_CLOSE), "Clear slot");
    gtk_widget_add_css_class(tile->close_button, "tile-close-button");
    g_signal_connect(tile->close_button, "clicked", G_CALLBACK(on_tile_close_clicked), tile);

    GtkWidget *spacer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_set_hexpand(spacer, TRUE);

    tile->volume_scale = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, PLAYER_VOLUME_MIN, PLAYER_VOLUME_MAX, 1);
    gtk_range_set_value(GTK_RANGE(tile->volume_scale), player_session_get_volume(tile->session));
    gtk_scale_set_draw_value(GTK_SCALE(tile->volume_scale), FALSE);
    gtk_widget_set_size_request(tile->volume_scale, 120, -1);
    g_signal_connect(tile->volume_scale, "value-changed", G_CALLBACK(on_volume_changed), tile);

    tile->focus_button = create_overlay_button(player_tile_focus_icon_new(PLAYER_TILE_FOCUS_ICON_EXPAND), "Focus tile");
    g_signal_connect(tile->focus_button, "clicked", G_CALLBACK(on_tile_focus_clicked), tile);

    tile->stream_info_button = create_overlay_button(player_info_icon_new(), PLAYER_STREAM_INFO_TOOLTIP);
    g_signal_connect(tile->stream_info_button, "clicked", G_CALLBACK(on_stream_info_clicked), tile);

    gtk_box_append(GTK_BOX(box), tile->channel_combo);
    gtk_box_append(GTK_BOX(box), tile->close_button);
    gtk_box_append(GTK_BOX(box), spacer);
    gtk_box_append(GTK_BOX(box), tile->volume_scale);
    gtk_box_append(GTK_BOX(box), tile->focus_button);
    gtk_box_append(GTK_BOX(box), tile->stream_info_button);
    update_tile_empty_state(tile);

    return box;
}

static GtkWidget *create_stream_tile(GridAppState *state, guint index, const char *target)
{
    StreamTile *tile = &state->tiles[index];
    tile->app = state;
    tile->index = index;
    tile->label = target_to_label(target);
    tile->url = target_to_url(target);
    if (index == 0 && state->primary_session != NULL) {
        tile->session = state->primary_session;
    } else if (tile->url != NULL && tile->url[0] != '\0') {
        tile->session = player_session_new();
    }
    tile->owns_session = tile->session != NULL && tile->session != state->primary_session;
    sync_tile_from_session(tile);

    tile->container = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    g_object_add_weak_pointer(G_OBJECT(tile->container), (gpointer *)&tile->container);
    gtk_widget_add_css_class(tile->container, "tile-container");
    if (index % 2 == 0) {
        gtk_widget_add_css_class(tile->container, "tile-left");
    }
    if (index / 2 == 0) {
        gtk_widget_add_css_class(tile->container, "tile-top");
    }
    gtk_widget_set_hexpand(tile->container, TRUE);
    gtk_widget_set_vexpand(tile->container, TRUE);

    tile->overlay = gtk_overlay_new();
    g_object_add_weak_pointer(G_OBJECT(tile->overlay), (gpointer *)&tile->overlay);
    gtk_widget_set_hexpand(tile->overlay, TRUE);
    gtk_widget_set_vexpand(tile->overlay, TRUE);
    gtk_box_append(GTK_BOX(tile->container), tile->overlay);

    tile->gl_area = gtk_gl_area_new();
    g_object_add_weak_pointer(G_OBJECT(tile->gl_area), (gpointer *)&tile->gl_area);
    gtk_gl_area_set_auto_render(GTK_GL_AREA(tile->gl_area), FALSE);
    gtk_widget_set_hexpand(tile->gl_area, TRUE);
    gtk_widget_set_vexpand(tile->gl_area, TRUE);
    gtk_overlay_set_child(GTK_OVERLAY(tile->overlay), tile->gl_area);

    tile->empty_label = gtk_label_new("Empty");
    gtk_widget_add_css_class(tile->empty_label, "empty-label");
    gtk_widget_set_halign(tile->empty_label, GTK_ALIGN_CENTER);
    gtk_widget_set_valign(tile->empty_label, GTK_ALIGN_CENTER);
    gtk_overlay_add_overlay(GTK_OVERLAY(tile->overlay), tile->empty_label);

    tile->footer = create_tile_footer(tile);
    gtk_widget_set_halign(tile->footer, GTK_ALIGN_FILL);
    gtk_widget_set_valign(tile->footer, GTK_ALIGN_END);
    gtk_widget_set_visible(tile->footer, FALSE);
    gtk_overlay_add_overlay(GTK_OVERLAY(tile->overlay), tile->footer);

    GtkGesture *video_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(video_click), GDK_BUTTON_PRIMARY);
    g_signal_connect(video_click, "pressed", G_CALLBACK(on_video_pressed), tile);
    gtk_widget_add_controller(tile->gl_area, GTK_EVENT_CONTROLLER(video_click));

    GtkEventController *video_legacy = gtk_event_controller_legacy_new();
    g_signal_connect(video_legacy, "event", G_CALLBACK(on_video_legacy_event), tile);
    gtk_widget_add_controller(tile->gl_area, video_legacy);

    GtkEventController *video_motion = gtk_event_controller_motion_new();
    gtk_event_controller_set_propagation_phase(video_motion, GTK_PHASE_CAPTURE);
    g_signal_connect(video_motion, "motion", G_CALLBACK(on_tile_motion), tile);
    gtk_widget_add_controller(tile->overlay, video_motion);

    GtkEventController *tile_scroll = gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_VERTICAL);
    gtk_event_controller_set_propagation_phase(tile_scroll, GTK_PHASE_CAPTURE);
    g_signal_connect(tile_scroll, "scroll", G_CALLBACK(on_tile_scroll), tile);
    gtk_widget_add_controller(tile->overlay, tile_scroll);

    g_signal_connect(tile->gl_area, "realize", G_CALLBACK(on_gl_realize), tile);
    g_signal_connect(tile->gl_area, "unrealize", G_CALLBACK(on_gl_unrealize), tile);
    g_signal_connect(tile->gl_area, "render", G_CALLBACK(on_gl_render), tile);

    update_tile_empty_state(tile);

    return tile->container;
}

static void install_css(void)
{
    GtkCssProvider *provider = gtk_css_provider_new();

    gtk_css_provider_load_from_string(
        provider,
        ".grid-root {"
        "  background: #050505;"
        "}"
        ".stream-grid {"
        "  background: #050505;"
        "}"
        ".tile-container {"
        "  background: #050505;"
        "  border: none;"
        "}"
        ".tile-left {"
        "  border-right: 1px solid rgba(255, 255, 255, 0.12);"
        "}"
        ".tile-top {"
        "  border-bottom: 1px solid rgba(255, 255, 255, 0.12);"
        "}"
        ".empty-label {"
        "  color: rgba(255, 255, 255, 0.35);"
        "  font-size: 18px;"
        "}"
        ".tile-footer {"
        "  background: rgba(0, 0, 0, 0.62);"
        "  color: white;"
        "  padding: 7px 9px;"
        "}"
        ".tile-footer button,"
        ".tile-footer menubutton,"
        ".tile-footer menubutton > button,"
        ".tile-footer popover,"
        ".tile-footer scale {"
        "  color: white;"
        "}"
        ".tile-footer button,"
        ".tile-footer menubutton > button {"
        "  background: rgba(30, 30, 30, 0.82);"
        "  color: white;"
        "  border-color: transparent;"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "}"
        ".tile-footer button:hover,"
        ".tile-footer menubutton > button:hover {"
        "  background: rgba(54, 54, 54, 0.90);"
        "}"
        ".channel-dropdown {"
        "  min-width: 170px;"
        "}"
        ".channel-dropdown > button {"
        "  padding-left: 10px;"
        "  padding-right: 8px;"
        "}"
        ".channel-button-label {"
        "  color: white;"
        "}"
        ".channel-popover contents {"
        "  background: rgba(28, 28, 28, 0.98);"
        "  padding: 0;"
        "  margin: 0;"
        "  border: none;"
        "  border-radius: 4px;"
        "  box-shadow: none;"
        "}"
        ".channel-popover {"
        "  padding: 0;"
        "  margin: 0;"
        "  border: none;"
        "  border-radius: 4px;"
        "  box-shadow: none;"
        "}"
        ".channel-menu {"
        "  background: rgba(28, 28, 28, 0.98);"
        "  padding: 2px 0;"
        "  margin: 0;"
        "}"
        ".channel-menu-item {"
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
        ".channel-menu-item box {"
        "  padding: 0;"
        "  margin: 0;"
        "}"
        ".channel-menu-item label {"
        "  color: white;"
        "  padding: 0;"
        "  margin: 0;"
        "}"
        ".channel-menu-item:hover {"
        "  background: rgba(74, 74, 74, 0.98);"
        "  color: white;"
        "}"
        ".tile-close-button:hover {"
        "  background: rgba(170, 36, 36, 0.90);"
        "}"
        ".tile-footer scale trough {"
        "  background: rgba(255, 255, 255, 0.20);"
        "}"
        ".tile-footer scale highlight {"
        "  background: rgba(255, 255, 255, 0.72);"
        "}"
        ".tile-footer scale slider {"
        "  background: rgba(255, 255, 255, 0.95);"
        "  border-color: rgba(0, 0, 0, 0.45);"
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
        ".close-button:hover {"
        "  background: rgba(170, 36, 36, 0.90);"
        "}"
    );

    gtk_style_context_add_provider_for_display(
        gdk_display_get_default(),
        GTK_STYLE_PROVIDER(provider),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    g_object_unref(provider);
}

static guint get_target_count(GridAppState *state)
{
    return MIN(state->target_count, MAX_TILES);
}

static const char *get_target_at(GridAppState *state, guint index)
{
    return index < state->target_count ? state->targets[index] : NULL;
}

void grid_player_free(GridPlayer *player)
{
    GridAppState *state = player;

    if (state == NULL) {
        return;
    }

    state->closing = TRUE;

    remove_source_if_active(&state->footer_hide_source);
    remove_source_if_active(&state->video_fullscreen_focus_source);

    for (guint i = 0; i < MAX_TILES; i++) {
        StreamTile *tile = &state->tiles[i];

        clear_tile_render_context(tile);
        player_session_set_wakeup_callback(tile->session, NULL, NULL);
        if (tile->owns_session) {
            player_session_free(tile->session);
        }
        tile->session = NULL;

        g_clear_pointer(&tile->label, g_free);
        g_clear_pointer(&tile->url, g_free);
        tile->container = NULL;
        tile->overlay = NULL;
        tile->gl_area = NULL;
        tile->footer = NULL;
        tile->channel_combo = NULL;
        tile->channel_label = NULL;
        tile->close_button = NULL;
        tile->empty_label = NULL;
        tile->stream_info_button = NULL;
        tile->volume_scale = NULL;
    }

    for (guint i = 0; i < MAX_TILES; i++) {
        g_clear_pointer(&state->targets[i], g_free);
    }

    state->root_overlay = NULL;
    state->grid = NULL;
    state->primary_session = NULL;
    state->settings = NULL;
    /* mpv may already have queued idle callbacks that still carry tile pointers. */
}

GridPlayer *grid_player_new(
    GtkWindow *window,
    AppSettings *settings,
    PlayerSession *primary_session,
    const char * const *targets,
    guint target_count,
    GridPlayerFullscreenCallback fullscreen_callback,
    gpointer fullscreen_user_data
)
{
    install_css();

    GridAppState *state = g_new0(GridAppState, 1);
    state->window = GTK_WIDGET(window);
    state->primary_session = primary_session;
    state->target_count = targets != NULL ? MIN(target_count, (guint)MAX_TILES) : 0;
    for (guint i = 0; i < state->target_count; i++) {
        state->targets[i] = g_strdup(targets[i]);
    }
    state->settings = settings;
    state->fullscreen_callback = fullscreen_callback;
    state->fullscreen_user_data = fullscreen_user_data;

    state->root_overlay = gtk_overlay_new();
    g_object_add_weak_pointer(G_OBJECT(state->root_overlay), (gpointer *)&state->root_overlay);
    gtk_widget_add_css_class(state->root_overlay, "grid-root");
    gtk_widget_set_hexpand(state->root_overlay, TRUE);
    gtk_widget_set_vexpand(state->root_overlay, TRUE);

    state->grid = gtk_grid_new();
    g_object_add_weak_pointer(G_OBJECT(state->grid), (gpointer *)&state->grid);
    gtk_widget_add_css_class(state->grid, "stream-grid");
    gtk_widget_set_hexpand(state->grid, TRUE);
    gtk_widget_set_vexpand(state->grid, TRUE);
    gtk_grid_set_row_homogeneous(GTK_GRID(state->grid), TRUE);
    gtk_grid_set_column_homogeneous(GTK_GRID(state->grid), TRUE);
    gtk_overlay_set_child(GTK_OVERLAY(state->root_overlay), state->grid);

    guint initial_target_count = get_target_count(state);
    for (guint i = 0; i < MAX_TILES; i++) {
        GtkWidget *tile_widget = create_stream_tile(
            state,
            i,
            i < initial_target_count ? get_target_at(state, i) : NULL
        );

        gtk_grid_attach(GTK_GRID(state->grid), tile_widget, i % 2, i / 2, 1, 1);
        state->grid_items[i] = tile_widget;
    }

    schedule_footer_hide(state);

    return state;
}

GtkWidget *grid_player_get_widget(GridPlayer *player)
{
    return player != NULL ? player->root_overlay : NULL;
}

char *grid_player_dup_first_target(GridPlayer *player)
{
    if (player == NULL) {
        return NULL;
    }

    for (guint i = 0; i < MAX_TILES; i++) {
        const char *url = player->tiles[i].url;
        if (url != NULL && url[0] != '\0') {
            return g_strdup(url);
        }
    }

    return NULL;
}

PlayerSession *grid_player_take_first_session(GridPlayer *player)
{
    if (player == NULL) {
        return NULL;
    }

    for (guint i = 0; i < MAX_TILES; i++) {
        StreamTile *tile = &player->tiles[i];
        if (!player_session_is_playing(tile->session)) {
            continue;
        }

        PlayerSession *session = tile->session;
        clear_tile_render_context(tile);
        player_session_set_wakeup_callback(session, NULL, NULL);
        tile->session = NULL;
        tile->owns_session = FALSE;
        return session;
    }

    return NULL;
}

void grid_player_start(GridPlayer *player)
{
    if (player == NULL || player->started) {
        return;
    }

    player->started = TRUE;
    for (guint i = 0; i < MAX_TILES; i++) {
        StreamTile *tile = &player->tiles[i];
        if ((tile->url == NULL || tile->url[0] == '\0') && !player_session_is_playing(tile->session)) {
            continue;
        }

        if (ensure_tile_session(tile)) {
            if (player_session_is_playing(tile->session)) {
                sync_tile_from_session(tile);
                update_tile_empty_state(tile);
                set_tile_status(tile, "Playback running");
                continue;
            }
            load_tile_stream(&player->tiles[i]);
        }
    }
}

void grid_player_set_fullscreen(GridPlayer *player, gboolean fullscreen)
{
    if (player != NULL) {
        player->fullscreen = fullscreen;
        if (!fullscreen) {
            player->video_fullscreen_active = FALSE;
            remove_source_if_active(&player->video_fullscreen_focus_source);
        }
    }
}

void grid_player_set_settings(GridPlayer *player, AppSettings *settings)
{
    if (player == NULL) {
        return;
    }

    player->settings = settings;
    for (guint i = 0; i < MAX_TILES; i++) {
        if (player->tiles[i].channel_combo != NULL) {
            rebuild_tile_channel_popover(&player->tiles[i]);
        }
    }
}
