#define G_LOG_DOMAIN "channel-switcher-overlay"

#include "channel_switcher_overlay.h"

#include "player_icons.h"
#include "twitch_stream_info.h"

#define PANEL_MARGIN 12
#define PANEL_TOP_SAFE_MARGIN 48
#define PANEL_BOTTOM_MARGIN 64
#define PANEL_EXTRA_VERTICAL_SPACE 36
#define LIVE_CHANNELS_CACHE_SECONDS 10
#define PANEL_MIN_WIDTH 430
#define PANEL_MAX_WIDTH 1200
#define PANEL_MIN_HEIGHT 150
#define PANEL_MAX_HEIGHT 520
#define CARD_WIDTH 226
#define CARD_SPACING 6
#define PREVIEW_WIDTH 210
#define PREVIEW_HEIGHT 118
#define AVATAR_SIZE 24

struct _ChannelSwitcherOverlay {
    GtkOverlay *overlay;
    GtkWidget *backdrop;
    GtkWidget *panel;
    GtkWidget *grid;
    GtkWidget *scroller;
    GtkWidget *search_entry;
    AppSettings *settings;
    GPtrArray *previews;
    char *cached_channels_key;
    gint64 cached_at_us;
    GCancellable *cancel;
    guint generation;
    ChannelSwitcherActivateCallback activate_callback;
    gpointer user_data;
};

typedef struct {
    ChannelSwitcherOverlay *switcher;
    guint generation;
    char *url;
    GtkWidget *image;
} RemoteImageData;

typedef struct {
    ChannelSwitcherOverlay *switcher;
    guint generation;
} LiveFetchCallbackData;

static gboolean css_installed = FALSE;

static void install_css(void)
{
    if (css_installed) {
        return;
    }

    GtkCssProvider *provider = gtk_css_provider_new();
    gtk_css_provider_load_from_string(
        provider,
        ".channel-switcher-panel {"
        "  background: rgba(12, 12, 14, 0.88);"
        "  color: #ffffff;"
        "  padding: 8px 8px 10px 8px;"
        "  border-radius: 6px;"
        "  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.45);"
        "}"
        ".channel-switcher-backdrop {"
        "  background: transparent;"
        "}"
        ".channel-switcher-header {"
        "  margin-bottom: 4px;"
        "}"
        ".channel-switcher-search {"
        "  background: rgba(255, 255, 255, 0.08);"
        "  color: #ffffff;"
        "  border-color: rgba(255, 255, 255, 0.10);"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "  font-size: 12px;"
        "  min-height: 22px;"
        "  min-width: 190px;"
        "  padding: 1px 7px;"
        "  border-radius: 4px;"
        "}"
        ".channel-switcher-search selection {"
        "  background: rgba(145, 70, 255, 0.50);"
        "  color: #ffffff;"
        "}"
        ".channel-switcher-close {"
        "  background: rgba(255, 255, 255, 0.08);"
        "  color: #ffffff;"
        "  border-color: transparent;"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "  min-width: 24px;"
        "  min-height: 22px;"
        "  padding: 1px 4px;"
        "  border-radius: 4px;"
        "}"
        ".channel-switcher-close:hover {"
        "  background: rgba(170, 36, 36, 0.90);"
        "}"
        ".channel-switcher-scroller,"
        ".channel-switcher-scroller viewport {"
        "  background: transparent;"
        "}"
        ".channel-switcher-grid {"
        "  background: transparent;"
        "}"
        ".channel-switcher-item {"
        "  background: rgba(255, 255, 255, 0.06);"
        "  color: #ffffff;"
        "  border-color: transparent;"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "  border-radius: 5px;"
        "  margin: 0;"
        "  padding: 7px;"
        "}"
        ".channel-switcher-item:hover {"
        "  background: rgba(255, 255, 255, 0.13);"
        "}"
        ".channel-switcher-preview {"
        "  background: rgba(255, 255, 255, 0.08);"
        "  border-radius: 4px;"
        "  min-width: 210px;"
        "  min-height: 118px;"
        "  max-width: 210px;"
        "  max-height: 118px;"
        "}"
        ".channel-switcher-avatar {"
        "  background: rgba(0, 0, 0, 0.55);"
        "  border-radius: 999px;"
        "  min-width: 24px;"
        "  min-height: 24px;"
        "  max-width: 24px;"
        "  max-height: 24px;"
        "  margin: 0;"
        "}"
        ".channel-switcher-name {"
        "  color: #ffffff;"
        "  font-weight: 700;"
        "  font-size: 13px;"
        "}"
        ".channel-switcher-title {"
        "  color: rgba(255, 255, 255, 0.86);"
        "  font-size: 12px;"
        "}"
        ".channel-switcher-meta {"
        "  color: rgba(255, 255, 255, 0.66);"
        "  font-size: 11px;"
        "}"
        ".channel-switcher-status {"
        "  color: rgba(255, 255, 255, 0.72);"
        "  padding: 12px;"
        "}"
    );
    gtk_style_context_add_provider_for_display(
        gdk_display_get_default(),
        GTK_STYLE_PROVIDER(provider),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION
    );
    g_object_unref(provider);
    css_installed = TRUE;
}

static void remote_image_data_free(RemoteImageData *data)
{
    if (data == NULL) {
        return;
    }

    g_clear_object(&data->image);
    g_free(data->url);
    g_free(data);
}

static void draw_remote_image(GtkDrawingArea *area, cairo_t *cr, int width, int height, gpointer user_data)
{
    (void)user_data;
    GdkPixbuf *pixbuf = g_object_get_data(G_OBJECT(area), "remote-image-pixbuf");

    cairo_set_source_rgba(cr, 1, 1, 1, 0.08);
    cairo_rectangle(cr, 0, 0, width, height);
    cairo_fill(cr);

    if (pixbuf == NULL || width <= 0 || height <= 0) {
        return;
    }

    int pixbuf_width = gdk_pixbuf_get_width(pixbuf);
    int pixbuf_height = gdk_pixbuf_get_height(pixbuf);
    if (pixbuf_width <= 0 || pixbuf_height <= 0) {
        return;
    }

    double scale = MAX(width / (double)pixbuf_width, height / (double)pixbuf_height);
    double draw_width = pixbuf_width * scale;
    double draw_height = pixbuf_height * scale;
    double offset_x = (width - draw_width) / 2.0;
    double offset_y = (height - draw_height) / 2.0;

    cairo_save(cr);
    cairo_rectangle(cr, 0, 0, width, height);
    cairo_clip(cr);
    cairo_translate(cr, offset_x, offset_y);
    cairo_scale(cr, scale, scale);
    gdk_cairo_set_source_pixbuf(cr, pixbuf, 0, 0);
    cairo_paint(cr);
    cairo_restore(cr);
}

static void live_fetch_callback_data_free(LiveFetchCallbackData *data)
{
    g_free(data);
}

static void on_remote_image_loaded(GObject *source, GAsyncResult *result, gpointer user_data)
{
    RemoteImageData *data = user_data;
    g_autoptr(GError) error = NULL;
    char *contents = NULL;
    gsize length = 0;

    if (data->generation != data->switcher->generation || data->switcher->panel == NULL) {
        remote_image_data_free(data);
        return;
    }

    if (!g_file_load_contents_finish(G_FILE(source), result, &contents, &length, NULL, &error)) {
        g_debug("image load failed for %s: %s", data->url, error != NULL ? error->message : "unknown error");
        remote_image_data_free(data);
        return;
    }

    g_autoptr(GBytes) bytes = g_bytes_new_take(contents, length);
    g_autoptr(GInputStream) stream = g_memory_input_stream_new_from_bytes(bytes);
    GdkPixbuf *pixbuf = gdk_pixbuf_new_from_stream(stream, NULL, &error);
    if (pixbuf != NULL && data->generation == data->switcher->generation) {
        g_object_set_data_full(G_OBJECT(data->image), "remote-image-pixbuf", pixbuf, g_object_unref);
        gtk_widget_queue_draw(data->image);
    } else {
        g_clear_object(&pixbuf);
    }

    remote_image_data_free(data);
}

static void load_remote_image(ChannelSwitcherOverlay *switcher, GtkWidget *image, const char *url)
{
    if (url == NULL || url[0] == '\0') {
        return;
    }

    RemoteImageData *data = g_new0(RemoteImageData, 1);
    data->switcher = switcher;
    data->generation = switcher->generation;
    data->image = g_object_ref(image);
    data->url = g_strdup(url);

    GFile *file = g_file_new_for_uri(url);
    g_file_load_contents_async(file, NULL, on_remote_image_loaded, data);
    g_object_unref(file);
}

static void clear_grid(ChannelSwitcherOverlay *switcher)
{
    if (switcher->grid == NULL) {
        return;
    }

    GtkWidget *child = gtk_widget_get_first_child(switcher->grid);
    while (child != NULL) {
        GtkWidget *next = gtk_widget_get_next_sibling(child);
        gtk_grid_remove(GTK_GRID(switcher->grid), child);
        child = next;
    }
}

static guint get_grid_columns(ChannelSwitcherOverlay *switcher)
{
    int panel_width = switcher->panel != NULL ? gtk_widget_get_width(switcher->panel) : 0;
    if (panel_width <= 1 && switcher->overlay != NULL) {
        int overlay_width = gtk_widget_get_width(GTK_WIDGET(switcher->overlay));
        panel_width = CLAMP(overlay_width - PANEL_MARGIN * 2, PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
    }

    int content_width = MAX(1, panel_width - 16);
    return MAX(1, (guint)((content_width + CARD_SPACING) / (CARD_WIDTH + CARD_SPACING)));
}

static void show_status(ChannelSwitcherOverlay *switcher, const char *message)
{
    if (switcher->grid == NULL) {
        return;
    }

    clear_grid(switcher);

    GtkWidget *label = gtk_label_new(message);
    gtk_widget_add_css_class(label, "channel-switcher-status");
    gtk_label_set_wrap(GTK_LABEL(label), TRUE);
    gtk_label_set_xalign(GTK_LABEL(label), 0.0);
    gtk_widget_set_hexpand(label, TRUE);
    gtk_grid_attach(GTK_GRID(switcher->grid), label, 0, 0, (int)get_grid_columns(switcher), 1);
}

static void position_panel(ChannelSwitcherOverlay *switcher)
{
    if (switcher->overlay == NULL || switcher->panel == NULL || switcher->scroller == NULL) {
        return;
    }

    int overlay_width = gtk_widget_get_width(GTK_WIDGET(switcher->overlay));
    int overlay_height = gtk_widget_get_height(GTK_WIDGET(switcher->overlay));
    double root_y = PANEL_TOP_SAFE_MARGIN;
    GtkRoot *root = gtk_widget_get_root(GTK_WIDGET(switcher->overlay));
    if (root != NULL && GTK_IS_WIDGET(root)) {
        graphene_point_t origin = GRAPHENE_POINT_INIT(0, 0);
        graphene_point_t root_point = GRAPHENE_POINT_INIT(0, 0);
        if (gtk_widget_compute_point(GTK_WIDGET(switcher->overlay), GTK_WIDGET(root), &origin, &root_point)) {
            root_y = root_point.y;
        }
    }
    /* Only reserve titlebar space when this video overlay starts under the
     * window controls. Grid tiles below the first row should start normally. */
    int top_margin = root_y < PANEL_TOP_SAFE_MARGIN
        ? PANEL_TOP_SAFE_MARGIN - (int)root_y
        : PANEL_MARGIN;
    int panel_width = CLAMP(overlay_width - PANEL_MARGIN * 2, PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
    int scroller_height = CLAMP(
        overlay_height - top_margin - PANEL_BOTTOM_MARGIN - PANEL_EXTRA_VERTICAL_SPACE,
        PANEL_MIN_HEIGHT,
        PANEL_MAX_HEIGHT
    );

    gtk_widget_set_size_request(switcher->panel, panel_width, -1);
    gtk_scrolled_window_set_max_content_height(GTK_SCROLLED_WINDOW(switcher->scroller), scroller_height);
    gtk_widget_set_margin_start(switcher->panel, PANEL_MARGIN);
    gtk_widget_set_margin_top(switcher->panel, top_margin);
}

static const AppSettingsChannel *find_settings_channel(ChannelSwitcherOverlay *switcher, const char *channel_name)
{
    guint channel_count = app_settings_get_channel_count(switcher->settings);

    for (guint i = 0; i < channel_count; i++) {
        const AppSettingsChannel *channel = app_settings_get_channel(switcher->settings, i);
        if (channel != NULL &&
            channel->channel != NULL &&
            channel_name != NULL &&
            g_ascii_strcasecmp(channel->channel, channel_name) == 0) {
            return channel;
        }
    }

    return NULL;
}

static void on_channel_button_clicked(GtkButton *button, gpointer user_data)
{
    ChannelSwitcherOverlay *switcher = user_data;
    const char *channel_name = g_object_get_data(G_OBJECT(button), "channel-name");
    const AppSettingsChannel *channel = find_settings_channel(switcher, channel_name);

    if (channel != NULL && switcher->activate_callback != NULL) {
        switcher->activate_callback(channel, switcher->user_data);
    }

    channel_switcher_overlay_hide(switcher);
}

static GtkWidget *create_image_picture(ChannelSwitcherOverlay *switcher, const char *url, int width, int height, const char *css_class)
{
    GtkWidget *image = gtk_drawing_area_new();
    gtk_widget_add_css_class(image, css_class);
    gtk_widget_set_size_request(image, width, height);
    gtk_drawing_area_set_content_width(GTK_DRAWING_AREA(image), width);
    gtk_drawing_area_set_content_height(GTK_DRAWING_AREA(image), height);
    gtk_widget_set_halign(image, GTK_ALIGN_START);
    gtk_widget_set_valign(image, GTK_ALIGN_START);
    gtk_widget_set_hexpand(image, FALSE);
    gtk_widget_set_vexpand(image, FALSE);
    gtk_widget_set_overflow(image, GTK_OVERFLOW_HIDDEN);
    gtk_drawing_area_set_draw_func(GTK_DRAWING_AREA(image), draw_remote_image, NULL, NULL);
    load_remote_image(switcher, image, url);
    return image;
}

static GtkWidget *create_fixed_picture_frame(GtkWidget *picture, int width, int height)
{
    GtkWidget *frame = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);

    gtk_widget_add_css_class(frame, "channel-switcher-image-frame");
    gtk_widget_set_size_request(frame, width, height);
    gtk_widget_set_halign(frame, GTK_ALIGN_START);
    gtk_widget_set_valign(frame, GTK_ALIGN_START);
    gtk_widget_set_hexpand(frame, FALSE);
    gtk_widget_set_vexpand(frame, FALSE);
    gtk_widget_set_overflow(frame, GTK_OVERFLOW_HIDDEN);
    gtk_box_append(GTK_BOX(frame), picture);

    return frame;
}

static char *format_viewer_count(guint viewer_count)
{
    if (viewer_count >= 1000000) {
        return g_strdup_printf("%.1fM viewers", viewer_count / 1000000.0);
    }
    if (viewer_count >= 1000) {
        return g_strdup_printf("%.1fK viewers", viewer_count / 1000.0);
    }
    return g_strdup_printf("%u viewers", viewer_count);
}

static char *format_live_duration(const char *started_at)
{
    if (started_at == NULL || started_at[0] == '\0') {
        return g_strdup("live");
    }

    g_autoptr(GDateTime) started = g_date_time_new_from_iso8601(started_at, NULL);
    if (started == NULL) {
        return g_strdup("live");
    }

    g_autoptr(GDateTime) now = g_date_time_new_now_utc();
    GTimeSpan span = g_date_time_difference(now, started);
    if (span < 0) {
        span = 0;
    }

    gint64 total_minutes = span / G_TIME_SPAN_MINUTE;
    gint64 hours = total_minutes / 60;
    gint64 minutes = total_minutes % 60;

    if (hours > 0) {
        return g_strdup_printf("live for %" G_GINT64_FORMAT "h %" G_GINT64_FORMAT "m", hours, minutes);
    }

    return g_strdup_printf("live for %" G_GINT64_FORMAT "m", minutes);
}

static char *format_meta_text(TwitchStreamPreview *preview)
{
    g_autofree char *viewers = format_viewer_count(preview->viewer_count);
    g_autofree char *duration = format_live_duration(preview->started_at);

    return g_strdup_printf("%s - %s", viewers, duration);
}

static GtkWidget *create_channel_card(ChannelSwitcherOverlay *switcher, TwitchStreamPreview *preview)
{
    const AppSettingsChannel *channel = find_settings_channel(switcher, preview->channel);
    const char *label = channel != NULL && channel->label != NULL && channel->label[0] != '\0'
        ? channel->label
        : preview->display_name;

    GtkWidget *button = gtk_button_new();
    gtk_widget_add_css_class(button, "channel-switcher-item");
    gtk_widget_set_halign(button, GTK_ALIGN_START);
    gtk_widget_set_hexpand(button, FALSE);
    gtk_widget_set_size_request(button, CARD_WIDTH, -1);
    g_object_set_data_full(G_OBJECT(button), "channel-name", g_strdup(preview->channel), g_free);
    g_signal_connect(button, "clicked", G_CALLBACK(on_channel_button_clicked), switcher);

    GtkWidget *card = gtk_box_new(GTK_ORIENTATION_VERTICAL, 7);
    gtk_widget_set_halign(card, GTK_ALIGN_START);
    gtk_widget_set_hexpand(card, FALSE);
    gtk_widget_set_size_request(card, CARD_WIDTH, -1);

    GtkWidget *preview_frame = create_fixed_picture_frame(
        create_image_picture(
            switcher,
            preview->preview_url,
            PREVIEW_WIDTH,
            PREVIEW_HEIGHT,
            "channel-switcher-preview"
        ),
        PREVIEW_WIDTH,
        PREVIEW_HEIGHT
    );
    gtk_widget_set_halign(preview_frame, GTK_ALIGN_CENTER);

    GtkWidget *text_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 4);
    gtk_widget_set_hexpand(text_box, TRUE);
    gtk_widget_set_valign(text_box, GTK_ALIGN_START);

    GtkWidget *details_row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 7);
    gtk_widget_set_halign(details_row, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand(details_row, TRUE);

    GtkWidget *avatar_frame = create_fixed_picture_frame(
        create_image_picture(
            switcher,
            preview->avatar_url,
            AVATAR_SIZE,
            AVATAR_SIZE,
            "channel-switcher-avatar"
        ),
        AVATAR_SIZE,
        AVATAR_SIZE
    );

    GtkWidget *title_label = gtk_label_new(preview->title != NULL ? preview->title : "");
    gtk_widget_add_css_class(title_label, "channel-switcher-title");
    gtk_label_set_xalign(GTK_LABEL(title_label), 0.0);
    gtk_label_set_single_line_mode(GTK_LABEL(title_label), TRUE);
    gtk_label_set_ellipsize(GTK_LABEL(title_label), PANGO_ELLIPSIZE_END);
    gtk_label_set_max_width_chars(GTK_LABEL(title_label), 26);
    gtk_widget_set_halign(title_label, GTK_ALIGN_FILL);

    GtkWidget *name_label = gtk_label_new(label);
    gtk_widget_add_css_class(name_label, "channel-switcher-name");
    gtk_label_set_xalign(GTK_LABEL(name_label), 0.0);
    gtk_label_set_ellipsize(GTK_LABEL(name_label), PANGO_ELLIPSIZE_END);
    gtk_label_set_max_width_chars(GTK_LABEL(name_label), 26);
    gtk_widget_set_halign(name_label, GTK_ALIGN_FILL);

    g_autofree char *meta_text = format_meta_text(preview);
    GtkWidget *meta_label = gtk_label_new(meta_text);
    gtk_widget_add_css_class(meta_label, "channel-switcher-meta");
    gtk_label_set_xalign(GTK_LABEL(meta_label), 0.0);
    gtk_label_set_ellipsize(GTK_LABEL(meta_label), PANGO_ELLIPSIZE_END);
    gtk_label_set_max_width_chars(GTK_LABEL(meta_label), 26);
    gtk_widget_set_halign(meta_label, GTK_ALIGN_FILL);

    gtk_box_append(GTK_BOX(text_box), title_label);
    gtk_box_append(GTK_BOX(text_box), name_label);
    gtk_box_append(GTK_BOX(text_box), meta_label);
    if (preview->category_name != NULL && preview->category_name[0] != '\0') {
        GtkWidget *category_label = gtk_label_new(preview->category_name);
        gtk_widget_add_css_class(category_label, "channel-switcher-meta");
        gtk_label_set_xalign(GTK_LABEL(category_label), 0.0);
        gtk_label_set_ellipsize(GTK_LABEL(category_label), PANGO_ELLIPSIZE_END);
        gtk_label_set_max_width_chars(GTK_LABEL(category_label), 26);
        gtk_widget_set_halign(category_label, GTK_ALIGN_FILL);
        gtk_box_append(GTK_BOX(text_box), category_label);
    }
    gtk_box_append(GTK_BOX(details_row), avatar_frame);
    gtk_box_append(GTK_BOX(details_row), text_box);
    gtk_box_append(GTK_BOX(card), preview_frame);
    gtk_box_append(GTK_BOX(card), details_row);
    gtk_button_set_child(GTK_BUTTON(button), card);

    return button;
}

static gboolean string_contains_casefold(const char *haystack, const char *needle)
{
    if (needle == NULL || needle[0] == '\0') {
        return TRUE;
    }
    if (haystack == NULL || haystack[0] == '\0') {
        return FALSE;
    }

    g_autofree char *haystack_folded = g_utf8_casefold(haystack, -1);
    g_autofree char *needle_folded = g_utf8_casefold(needle, -1);
    return strstr(haystack_folded, needle_folded) != NULL;
}

static gboolean preview_matches_filter(TwitchStreamPreview *preview, const char *filter)
{
    return string_contains_casefold(preview->channel, filter) ||
        string_contains_casefold(preview->display_name, filter) ||
        string_contains_casefold(preview->title, filter) ||
        string_contains_casefold(preview->category_name, filter);
}

static void render_live_channels(ChannelSwitcherOverlay *switcher)
{
    if (switcher->previews == NULL || switcher->previews->len == 0) {
        show_status(switcher, "No configured channels are live");
        return;
    }

    const char *filter = switcher->search_entry != NULL
        ? gtk_editable_get_text(GTK_EDITABLE(switcher->search_entry))
        : "";
    guint visible_count = 0;

    clear_grid(switcher);
    guint columns = get_grid_columns(switcher);
    for (guint i = 0; i < switcher->previews->len; i++) {
        TwitchStreamPreview *preview = g_ptr_array_index(switcher->previews, i);
        if (!preview_matches_filter(preview, filter)) {
            continue;
        }

        gtk_grid_attach(
            GTK_GRID(switcher->grid),
            create_channel_card(switcher, preview),
            (int)(visible_count % columns),
            (int)(visible_count / columns),
            1,
            1
        );
        visible_count++;
    }

    if (visible_count == 0) {
        show_status(switcher, "No live channels match the filter");
    }
}

static void on_search_changed(GtkEditable *editable, gpointer user_data)
{
    (void)editable;
    render_live_channels(user_data);
}

static void on_live_channels_fetched(GObject *source_object, GAsyncResult *result, gpointer user_data)
{
    (void)source_object;
    LiveFetchCallbackData *data = user_data;
    ChannelSwitcherOverlay *switcher = data->switcher;
    g_autoptr(GError) error = NULL;
    g_autoptr(GPtrArray) previews = twitch_stream_info_fetch_live_channels_finish(result, &error);

    if (data->generation != switcher->generation || switcher->panel == NULL) {
        live_fetch_callback_data_free(data);
        return;
    }

    g_clear_object(&switcher->cancel);

    if (error != NULL) {
        if (!g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            g_debug("live channel fetch failed: %s", error->message);
            show_status(switcher, "Live channels could not be loaded");
        }
        live_fetch_callback_data_free(data);
        return;
    }

    if (switcher->previews != NULL) {
        g_ptr_array_unref(switcher->previews);
    }
    switcher->previews = previews != NULL ? g_ptr_array_ref(previews) : NULL;
    switcher->cached_at_us = g_get_monotonic_time();
    render_live_channels(switcher);

    live_fetch_callback_data_free(data);
}

static void collect_settings_channels(ChannelSwitcherOverlay *switcher, char ***channels_out, guint *channel_count_out)
{
    guint channel_count = app_settings_get_channel_count(switcher->settings);
    char **channels = g_new0(char *, channel_count + 1);
    guint out = 0;

    for (guint i = 0; i < channel_count; i++) {
        const AppSettingsChannel *channel = app_settings_get_channel(switcher->settings, i);
        if (channel == NULL || channel->channel == NULL || channel->channel[0] == '\0') {
            continue;
        }

        channels[out++] = g_strdup(channel->channel);
    }

    *channels_out = channels;
    *channel_count_out = out;
}

static char *build_channels_cache_key(char **channels, guint channel_count)
{
    GString *key = g_string_new(NULL);

    for (guint i = 0; i < channel_count; i++) {
        if (i > 0) {
            g_string_append_c(key, '\n');
        }
        g_string_append(key, channels[i] != NULL ? channels[i] : "");
    }

    return g_string_free(key, FALSE);
}

static gboolean has_fresh_cache(ChannelSwitcherOverlay *switcher, const char *channels_key)
{
    gint64 now_us = g_get_monotonic_time();

    return switcher->previews != NULL &&
        switcher->cached_channels_key != NULL &&
        channels_key != NULL &&
        g_strcmp0(switcher->cached_channels_key, channels_key) == 0 &&
        now_us - switcher->cached_at_us < LIVE_CHANNELS_CACHE_SECONDS * G_USEC_PER_SEC;
}

static void on_close_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    channel_switcher_overlay_hide(user_data);
}

static void on_backdrop_pressed(GtkGestureClick *gesture, int n_press, double x, double y, gpointer user_data)
{
    (void)gesture;
    (void)x;
    (void)y;

    if (n_press == 1) {
        channel_switcher_overlay_hide(user_data);
    }
}

ChannelSwitcherOverlay *channel_switcher_overlay_new(
    GtkOverlay *overlay,
    AppSettings *settings,
    ChannelSwitcherActivateCallback activate_callback,
    gpointer user_data
)
{
    install_css();

    ChannelSwitcherOverlay *switcher = g_new0(ChannelSwitcherOverlay, 1);
    switcher->overlay = overlay;
    g_object_add_weak_pointer(G_OBJECT(switcher->overlay), (gpointer *)&switcher->overlay);
    switcher->settings = settings;
    switcher->activate_callback = activate_callback;
    switcher->user_data = user_data;

    switcher->backdrop = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    g_object_add_weak_pointer(G_OBJECT(switcher->backdrop), (gpointer *)&switcher->backdrop);
    gtk_widget_add_css_class(switcher->backdrop, "channel-switcher-backdrop");
    gtk_widget_set_halign(switcher->backdrop, GTK_ALIGN_FILL);
    gtk_widget_set_valign(switcher->backdrop, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand(switcher->backdrop, TRUE);
    gtk_widget_set_vexpand(switcher->backdrop, TRUE);
    gtk_widget_set_visible(switcher->backdrop, FALSE);
    GtkGesture *backdrop_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(backdrop_click), GDK_BUTTON_PRIMARY);
    g_signal_connect(backdrop_click, "pressed", G_CALLBACK(on_backdrop_pressed), switcher);
    gtk_widget_add_controller(switcher->backdrop, GTK_EVENT_CONTROLLER(backdrop_click));
    gtk_overlay_add_overlay(overlay, switcher->backdrop);

    switcher->panel = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    g_object_add_weak_pointer(G_OBJECT(switcher->panel), (gpointer *)&switcher->panel);
    gtk_widget_add_css_class(switcher->panel, "channel-switcher-panel");
    gtk_widget_set_halign(switcher->panel, GTK_ALIGN_START);
    gtk_widget_set_valign(switcher->panel, GTK_ALIGN_START);
    gtk_widget_set_visible(switcher->panel, FALSE);

    GtkWidget *header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_add_css_class(header, "channel-switcher-header");
    switcher->search_entry = gtk_search_entry_new();
    g_object_add_weak_pointer(G_OBJECT(switcher->search_entry), (gpointer *)&switcher->search_entry);
    gtk_widget_add_css_class(switcher->search_entry, "channel-switcher-search");
    gtk_search_entry_set_placeholder_text(GTK_SEARCH_ENTRY(switcher->search_entry), "Filter live channels");
    g_signal_connect(switcher->search_entry, "changed", G_CALLBACK(on_search_changed), switcher);
    GtkWidget *header_spacer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_set_hexpand(header_spacer, TRUE);
    GtkWidget *close_button = gtk_button_new();
    gtk_button_set_child(GTK_BUTTON(close_button), player_window_icon_new(PLAYER_WINDOW_ICON_CLOSE));
    gtk_widget_add_css_class(close_button, "channel-switcher-close");
    gtk_widget_set_tooltip_text(close_button, "Close");
    g_signal_connect(close_button, "clicked", G_CALLBACK(on_close_clicked), switcher);
    gtk_box_append(GTK_BOX(header), switcher->search_entry);
    gtk_box_append(GTK_BOX(header), header_spacer);
    gtk_box_append(GTK_BOX(header), close_button);
    gtk_box_append(GTK_BOX(switcher->panel), header);

    switcher->scroller = gtk_scrolled_window_new();
    g_object_add_weak_pointer(G_OBJECT(switcher->scroller), (gpointer *)&switcher->scroller);
    gtk_widget_add_css_class(switcher->scroller, "channel-switcher-scroller");
    gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(switcher->scroller), GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_scrolled_window_set_propagate_natural_height(GTK_SCROLLED_WINDOW(switcher->scroller), TRUE);

    switcher->grid = gtk_grid_new();
    g_object_add_weak_pointer(G_OBJECT(switcher->grid), (gpointer *)&switcher->grid);
    gtk_widget_add_css_class(switcher->grid, "channel-switcher-grid");
    gtk_widget_set_halign(switcher->grid, GTK_ALIGN_START);
    gtk_widget_set_hexpand(switcher->grid, FALSE);
    gtk_grid_set_column_spacing(GTK_GRID(switcher->grid), CARD_SPACING);
    gtk_grid_set_row_spacing(GTK_GRID(switcher->grid), CARD_SPACING);
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(switcher->scroller), switcher->grid);
    gtk_box_append(GTK_BOX(switcher->panel), switcher->scroller);
    gtk_overlay_add_overlay(overlay, switcher->panel);

    return switcher;
}

void channel_switcher_overlay_set_settings(ChannelSwitcherOverlay *switcher, AppSettings *settings)
{
    if (switcher == NULL) {
        return;
    }

    switcher->settings = settings;
    if (switcher->previews != NULL) {
        g_ptr_array_unref(switcher->previews);
        switcher->previews = NULL;
    }
    g_clear_pointer(&switcher->cached_channels_key, g_free);
    switcher->cached_at_us = 0;
}

void channel_switcher_overlay_show_at(ChannelSwitcherOverlay *switcher, double x, double y)
{
    (void)x;
    (void)y;

    if (switcher == NULL || switcher->settings == NULL || switcher->panel == NULL) {
        return;
    }

    switcher->generation++;
    if (switcher->search_entry != NULL) {
        gtk_editable_set_text(GTK_EDITABLE(switcher->search_entry), "");
    }
    if (switcher->backdrop != NULL) {
        gtk_widget_set_visible(switcher->backdrop, TRUE);
    }
    gtk_widget_set_visible(switcher->panel, TRUE);
    position_panel(switcher);
    if (switcher->search_entry != NULL) {
        gtk_widget_grab_focus(switcher->search_entry);
    }
    show_status(switcher, "Loading live channels");

    g_clear_object(&switcher->cancel);
    g_auto(GStrv) channels = NULL;
    guint channel_count = 0;
    collect_settings_channels(switcher, &channels, &channel_count);
    g_autofree char *channels_key = build_channels_cache_key(channels, channel_count);

    if (channel_count == 0) {
        if (switcher->previews != NULL) {
            g_ptr_array_unref(switcher->previews);
            switcher->previews = NULL;
        }
        g_clear_pointer(&switcher->cached_channels_key, g_free);
        switcher->cached_at_us = 0;
        show_status(switcher, "No channels configured");
        return;
    }

    if (has_fresh_cache(switcher, channels_key)) {
        render_live_channels(switcher);
        return;
    }

    if (switcher->previews != NULL) {
        g_ptr_array_unref(switcher->previews);
        switcher->previews = NULL;
    }
    g_free(switcher->cached_channels_key);
    switcher->cached_channels_key = g_strdup(channels_key);
    switcher->cached_at_us = 0;

    switcher->cancel = g_cancellable_new();
    LiveFetchCallbackData *data = g_new0(LiveFetchCallbackData, 1);
    data->switcher = switcher;
    data->generation = switcher->generation;
    twitch_stream_info_fetch_live_channels_async(
        (const char * const *)channels,
        channel_count,
        switcher->cancel,
        on_live_channels_fetched,
        data
    );
}

void channel_switcher_overlay_hide(ChannelSwitcherOverlay *switcher)
{
    if (switcher == NULL) {
        return;
    }

    switcher->generation++;
    if (switcher->cancel != NULL) {
        g_cancellable_cancel(switcher->cancel);
        g_clear_object(&switcher->cancel);
    }
    if (switcher->panel != NULL) {
        gtk_widget_set_visible(switcher->panel, FALSE);
    }
    if (switcher->backdrop != NULL) {
        gtk_widget_set_visible(switcher->backdrop, FALSE);
    }
    clear_grid(switcher);
}

gboolean channel_switcher_overlay_is_visible(ChannelSwitcherOverlay *switcher)
{
    return switcher != NULL &&
        switcher->panel != NULL &&
        gtk_widget_get_visible(switcher->panel);
}

void channel_switcher_overlay_free(ChannelSwitcherOverlay *switcher)
{
    if (switcher == NULL) {
        return;
    }

    channel_switcher_overlay_hide(switcher);
    if (switcher->previews != NULL) {
        g_ptr_array_unref(switcher->previews);
        switcher->previews = NULL;
    }
    g_clear_pointer(&switcher->cached_channels_key, g_free);
    switcher->cached_at_us = 0;
    if (switcher->panel != NULL && switcher->overlay != NULL) {
        gtk_overlay_remove_overlay(switcher->overlay, switcher->panel);
    }
    if (switcher->backdrop != NULL && switcher->overlay != NULL) {
        gtk_overlay_remove_overlay(switcher->overlay, switcher->backdrop);
    }
    switcher->panel = NULL;
    switcher->backdrop = NULL;
    switcher->grid = NULL;
    switcher->scroller = NULL;
    switcher->search_entry = NULL;
    switcher->overlay = NULL;
    /* Async image/network callbacks can still carry this pointer briefly after
     * cancellation. Keep the tiny shell allocated, matching the player lifetime
     * pattern used for queued mpv callbacks. */
}
