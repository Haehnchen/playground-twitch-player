#define G_LOG_DOMAIN "channel-switcher-overlay"

#include "channel_switcher_overlay.h"

#include "player_icons.h"
#include "twitch_channel_list.h"
#include "twitch_stream_info.h"

#include <string.h>

#define PANEL_MARGIN 12
#define PANEL_TOP_SAFE_MARGIN 48
#define PANEL_BOTTOM_MARGIN 64
#define PANEL_EXTRA_VERTICAL_SPACE 36
#define LIVE_CHANNELS_CACHE_SECONDS 10
#define SEARCH_DEBOUNCE_MS 300
#define PANEL_MIN_WIDTH 430
#define PANEL_MAX_WIDTH 1300
#define PANEL_MAX_COLUMNS 4
#define PANEL_HORIZONTAL_PADDING 16
#define PANEL_SCROLLBAR_RESERVE_WIDTH 18
#define CARD_WIDTH 226
#define CARD_HORIZONTAL_PADDING 10
#define CARD_SPACING 6
#define PREVIEW_WIDTH 226
#define PREVIEW_HEIGHT 127
#define AVATAR_SIZE 24
#define CARD_OUTER_WIDTH (CARD_WIDTH + CARD_HORIZONTAL_PADDING)

struct _ChannelSwitcherOverlay {
    GtkOverlay *overlay;
    GtkWidget *backdrop;
    GtkWidget *panel;
    GtkWidget *grid;
    GtkWidget *scroller;
    GtkWidget *search_entry;
    GtkWidget *direct_channel_entry;
    AppSettings *settings;
    GPtrArray *previews;
    GPtrArray *preview_cards;
    guint preview_card_columns;
    int preview_card_width;
    int preview_width;
    int preview_height;
    GHashTable *image_cache;
    GHashTable *image_waiters;
    char *cached_channels_key;
    gint64 cached_at_us;
    GCancellable *cancel;
    guint search_debounce_source;
    guint generation;
    ChannelSwitcherActivateCallback activate_callback;
    gpointer user_data;
    ChannelSwitcherSettingsCallback settings_callback;
    gpointer settings_user_data;
};

typedef struct {
    ChannelSwitcherOverlay *switcher;
    guint generation;
    char *url;
    char *cache_key;
    int width;
    int height;
} RemoteImageData;

typedef struct {
    ChannelSwitcherOverlay *switcher;
    guint generation;
} LiveFetchCallbackData;

typedef struct {
    ChannelSwitcherOverlay *switcher;
    guint generation;
} ChannelListFetchCallbackData;

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
        "  min-width: 165px;"
        "  padding: 1px 7px;"
        "  border-radius: 4px;"
        "}"
        ".channel-switcher-header-separator {"
        "  color: rgba(255, 255, 255, 0.34);"
        "  font-size: 13px;"
        "  margin-left: 1px;"
        "  margin-right: 1px;"
        "}"
        ".channel-switcher-open-entry {"
        "  background: rgba(255, 255, 255, 0.08);"
        "  color: #ffffff;"
        "  border-color: rgba(255, 255, 255, 0.10);"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "  font-size: 12px;"
        "  min-height: 22px;"
        "  min-width: 165px;"
        "  padding: 1px 7px;"
        "  border-radius: 4px;"
        "}"
        ".channel-switcher-search selection {"
        "  background: rgba(145, 70, 255, 0.50);"
        "  color: #ffffff;"
        "}"
        ".channel-switcher-action,"
        ".channel-switcher-close {"
        "  background: transparent;"
        "  background-image: none;"
        "  color: #ffffff;"
        "  border-color: transparent;"
        "  outline-color: transparent;"
        "  box-shadow: none;"
        "  min-width: 24px;"
        "  min-height: 22px;"
        "  padding: 1px 4px;"
        "  border-radius: 4px;"
        "}"
        ".channel-switcher-action:hover {"
        "  background: rgba(255, 255, 255, 0.16);"
        "}"
        ".channel-switcher-close:hover {"
        "  background: rgba(170, 36, 36, 0.90);"
        "  background-image: none;"
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
        "  padding: 5px;"
        "}"
        ".channel-switcher-item:hover {"
        "  background: rgba(255, 255, 255, 0.13);"
        "}"
        ".channel-switcher-preview {"
        "  background: rgba(255, 255, 255, 0.08);"
        "  border-radius: 4px;"
        "  min-width: 226px;"
        "  min-height: 127px;"
        "}"
        ".channel-switcher-avatar {"
        "  background: rgba(0, 0, 0, 0.55);"
        "  border-radius: 999px;"
        "  min-width: 24px;"
        "  min-height: 24px;"
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

    g_free(data->url);
    g_free(data->cache_key);
    g_free(data);
}

static void remove_source_if_active(guint *source_id)
{
    if (source_id != NULL && *source_id != 0) {
        g_source_remove(*source_id);
        *source_id = 0;
    }
}

static void clear_grid(ChannelSwitcherOverlay *switcher);

static void clear_preview_cards(ChannelSwitcherOverlay *switcher)
{
    clear_grid(switcher);
    if (switcher->preview_cards != NULL) {
        g_ptr_array_set_size(switcher->preview_cards, 0);
    }
    switcher->preview_card_columns = 0;
    switcher->preview_card_width = 0;
    switcher->preview_width = 0;
    switcher->preview_height = 0;
}

static void clear_image_cache(ChannelSwitcherOverlay *switcher)
{
    if (switcher->image_cache != NULL) {
        g_hash_table_remove_all(switcher->image_cache);
    }
    if (switcher->image_waiters != NULL) {
        g_hash_table_remove_all(switcher->image_waiters);
    }
}

static void set_remote_image_texture(GtkWidget *image, GdkTexture *texture)
{
    if (!GTK_IS_PICTURE(image) || texture == NULL) {
        return;
    }

    gtk_picture_set_paintable(GTK_PICTURE(image), GDK_PAINTABLE(texture));
}

static char *build_image_cache_key(const char *url, int width, int height)
{
    return g_strdup_printf("%s\n%d:%d", url, width, height);
}

static GdkTexture *create_cover_texture_from_bytes(GBytes *bytes, int width, int height, GError **error)
{
    if (width <= 0 || height <= 0) {
        g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_ARGUMENT, "invalid image target size");
        return NULL;
    }

    g_autoptr(GInputStream) stream = g_memory_input_stream_new_from_bytes(bytes);
    g_autoptr(GdkPixbuf) source = gdk_pixbuf_new_from_stream(stream, NULL, error);
    if (source == NULL) {
        return NULL;
    }

    int source_width = gdk_pixbuf_get_width(source);
    int source_height = gdk_pixbuf_get_height(source);
    if (source_width <= 0 || source_height <= 0) {
        g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED, "invalid image dimensions");
        return NULL;
    }

    double scale = MAX(width / (double)source_width, height / (double)source_height);
    double scaled_width = source_width * scale;
    double scaled_height = source_height * scale;
    double offset_x = (width - scaled_width) / 2.0;
    double offset_y = (height - scaled_height) / 2.0;

    g_autoptr(GdkPixbuf) target = gdk_pixbuf_new(GDK_COLORSPACE_RGB, TRUE, 8, width, height);
    if (target == NULL) {
        g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED, "could not allocate image target");
        return NULL;
    }

    gdk_pixbuf_fill(target, 0x00000000);
    gdk_pixbuf_composite(
        source,
        target,
        0,
        0,
        width,
        height,
        offset_x,
        offset_y,
        scale,
        scale,
        GDK_INTERP_BILINEAR,
        255
    );

    gsize stride = (gsize)gdk_pixbuf_get_rowstride(target);
    gsize length = stride * (gsize)height;
    g_autoptr(GBytes) texture_bytes = g_bytes_new(gdk_pixbuf_get_pixels(target), length);

    return gdk_memory_texture_new(width, height, GDK_MEMORY_R8G8B8A8, texture_bytes, stride);
}

static GdkTexture *create_placeholder_texture(int width, int height)
{
    if (width <= 0 || height <= 0) {
        return NULL;
    }

    gsize stride = (gsize)width * 4;
    gsize length = stride * (gsize)height;
    g_autoptr(GBytes) bytes = g_bytes_new_take(g_malloc0(length), length);

    return gdk_memory_texture_new(width, height, GDK_MEMORY_R8G8B8A8, bytes, stride);
}

static void live_fetch_callback_data_free(LiveFetchCallbackData *data)
{
    g_free(data);
}

static void on_remote_image_loaded(GObject *source, GAsyncResult *result, gpointer user_data)
{
    RemoteImageData *data = user_data;
    ChannelSwitcherOverlay *switcher = data->switcher;
    g_autoptr(GError) error = NULL;
    char *contents = NULL;
    gsize length = 0;

    if (switcher->image_waiters == NULL) {
        remote_image_data_free(data);
        return;
    }

    GPtrArray *waiters = g_hash_table_lookup(switcher->image_waiters, data->cache_key);
    if (waiters != NULL) {
        g_ptr_array_ref(waiters);
        g_hash_table_remove(switcher->image_waiters, data->cache_key);
    }

    if (!g_file_load_contents_finish(G_FILE(source), result, &contents, &length, NULL, &error)) {
        g_debug("image load failed for %s: %s", data->url, error != NULL ? error->message : "unknown error");
        if (waiters != NULL) {
            g_ptr_array_unref(waiters);
        }
        remote_image_data_free(data);
        return;
    }

    g_autoptr(GBytes) bytes = g_bytes_new_take(contents, length);
    if (data->generation != switcher->generation || switcher->panel == NULL) {
        if (waiters != NULL) {
            g_ptr_array_unref(waiters);
        }
        remote_image_data_free(data);
        return;
    }

    GdkTexture *texture = create_cover_texture_from_bytes(bytes, data->width, data->height, &error);
    if (texture != NULL) {
        if (switcher->image_cache != NULL) {
            g_hash_table_insert(switcher->image_cache, g_strdup(data->cache_key), g_object_ref(texture));
        }
        if (waiters != NULL) {
            for (guint i = 0; i < waiters->len; i++) {
                set_remote_image_texture(g_ptr_array_index(waiters, i), texture);
            }
        }
        g_object_unref(texture);
    } else if (error != NULL) {
        g_debug("image decode failed for %s: %s", data->url, error->message);
    }

    if (waiters != NULL) {
        g_ptr_array_unref(waiters);
    }

    remote_image_data_free(data);
}

static void load_remote_image(ChannelSwitcherOverlay *switcher, GtkWidget *image, const char *url, int width, int height)
{
    if (url == NULL || url[0] == '\0') {
        return;
    }

    if (switcher->image_cache == NULL || switcher->image_waiters == NULL) {
        return;
    }

    g_autofree char *cache_key = build_image_cache_key(url, width, height);
    GdkTexture *cached = g_hash_table_lookup(switcher->image_cache, cache_key);
    if (cached != NULL) {
        set_remote_image_texture(image, cached);
        return;
    }

    GPtrArray *waiters = g_hash_table_lookup(switcher->image_waiters, cache_key);
    if (waiters != NULL) {
        g_ptr_array_add(waiters, g_object_ref(image));
        return;
    }

    waiters = g_ptr_array_new_with_free_func(g_object_unref);
    g_ptr_array_add(waiters, g_object_ref(image));
    g_hash_table_insert(switcher->image_waiters, g_strdup(cache_key), waiters);

    RemoteImageData *data = g_new0(RemoteImageData, 1);
    data->switcher = switcher;
    data->generation = switcher->generation;
    data->url = g_strdup(url);
    data->cache_key = g_strdup(cache_key);
    data->width = width;
    data->height = height;

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

static int calculate_panel_width(int overlay_width)
{
    int available_width = MAX(1, overlay_width - PANEL_MARGIN * 2);
    int max_panel_width = MIN(PANEL_MAX_WIDTH, available_width);
    return CLAMP(max_panel_width, MIN(PANEL_MIN_WIDTH, available_width), max_panel_width);
}

static int calculate_grid_width(int panel_width)
{
    return MAX(1, panel_width - PANEL_HORIZONTAL_PADDING - PANEL_SCROLLBAR_RESERVE_WIDTH);
}

static guint calculate_grid_columns(int grid_width)
{
    guint columns = MIN(
        (guint)PANEL_MAX_COLUMNS,
        MAX(1, (guint)((grid_width + CARD_SPACING) / (CARD_OUTER_WIDTH + CARD_SPACING)))
    );

    return columns;
}

static int calculate_card_width(int grid_width, guint columns)
{
    int spacing_width = (int)(columns > 0 ? columns - 1 : 0) * CARD_SPACING;
    int card_outer_width = (grid_width - spacing_width) / (int)MAX(1, columns);

    return MAX(CARD_WIDTH, card_outer_width - CARD_HORIZONTAL_PADDING);
}

static int calculate_preview_height(int preview_width)
{
    return MAX(1, (preview_width * 9 + 8) / 16);
}

static void calculate_card_layout(
    ChannelSwitcherOverlay *switcher,
    guint *columns,
    int *card_width,
    int *preview_width,
    int *preview_height
)
{
    int panel_width = 0;

    if (switcher->overlay != NULL) {
        panel_width = calculate_panel_width(gtk_widget_get_width(GTK_WIDGET(switcher->overlay)));
    } else if (switcher->panel != NULL) {
        panel_width = gtk_widget_get_width(switcher->panel);
    }

    int grid_width = calculate_grid_width(panel_width);
    guint calculated_columns = calculate_grid_columns(grid_width);
    int calculated_card_width = calculate_card_width(grid_width, calculated_columns);

    if (columns != NULL) {
        *columns = calculated_columns;
    }
    if (card_width != NULL) {
        *card_width = calculated_card_width;
    }
    if (preview_width != NULL) {
        *preview_width = calculated_card_width;
    }
    if (preview_height != NULL) {
        *preview_height = calculate_preview_height(calculated_card_width);
    }
}

static guint get_grid_columns(ChannelSwitcherOverlay *switcher)
{
    guint columns = 1;

    calculate_card_layout(switcher, &columns, NULL, NULL, NULL);
    return columns;
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
    int panel_width = calculate_panel_width(overlay_width);
    int grid_width = calculate_grid_width(panel_width);
    int scroller_height = MAX(
        1,
        overlay_height - top_margin - PANEL_BOTTOM_MARGIN - PANEL_EXTRA_VERTICAL_SPACE
    );

    gtk_widget_set_size_request(switcher->panel, panel_width, -1);
    gtk_scrolled_window_set_min_content_width(GTK_SCROLLED_WINDOW(switcher->scroller), grid_width);
    gtk_scrolled_window_set_max_content_width(GTK_SCROLLED_WINDOW(switcher->scroller), grid_width);
    gtk_scrolled_window_set_max_content_height(GTK_SCROLLED_WINDOW(switcher->scroller), scroller_height);
    gtk_widget_set_margin_start(switcher->panel, PANEL_MARGIN);
    gtk_widget_set_margin_end(switcher->panel, PANEL_MARGIN);
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

static char *extract_twitch_channel_name(const char *value)
{
    if (value == NULL) {
        return NULL;
    }

    g_autofree char *trimmed = g_strdup(value);
    g_strstrip(trimmed);
    if (trimmed[0] == '\0') {
        return NULL;
    }

    const char *start = strstr(trimmed, "twitch.tv/");
    if (start != NULL) {
        start += strlen("twitch.tv/");
    } else {
        start = trimmed;
    }

    while (*start == '/' || *start == '@') {
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

static void on_channel_button_clicked(GtkButton *button, gpointer user_data)
{
    ChannelSwitcherOverlay *switcher = user_data;
    const char *channel_name = g_object_get_data(G_OBJECT(button), "channel-name");
    const AppSettingsChannel *channel = find_settings_channel(switcher, channel_name);

    if (channel != NULL && switcher->activate_callback != NULL) {
        switcher->activate_callback(channel, switcher->user_data);
    } else if (channel_name != NULL && channel_name[0] != '\0' && switcher->activate_callback != NULL) {
        AppSettingsChannel dynamic_channel = {0};
        dynamic_channel.channel = (char *)channel_name;
        dynamic_channel.label = (char *)g_object_get_data(G_OBJECT(button), "channel-label");
        dynamic_channel.url = g_strdup_printf("https://www.twitch.tv/%s", channel_name);
        if (dynamic_channel.label == NULL || dynamic_channel.label[0] == '\0') {
            dynamic_channel.label = dynamic_channel.channel;
        }
        switcher->activate_callback(&dynamic_channel, switcher->user_data);
        g_free(dynamic_channel.url);
    }

    channel_switcher_overlay_hide(switcher);
}

static void activate_dynamic_channel(ChannelSwitcherOverlay *switcher, const char *channel_name)
{
    if (switcher == NULL ||
        switcher->activate_callback == NULL ||
        channel_name == NULL ||
        channel_name[0] == '\0') {
        return;
    }

    const AppSettingsChannel *configured_channel = find_settings_channel(switcher, channel_name);
    if (configured_channel != NULL) {
        switcher->activate_callback(configured_channel, switcher->user_data);
        channel_switcher_overlay_hide(switcher);
        return;
    }

    AppSettingsChannel dynamic_channel = {0};
    dynamic_channel.channel = (char *)channel_name;
    dynamic_channel.label = (char *)channel_name;
    dynamic_channel.url = g_strdup_printf("https://www.twitch.tv/%s", channel_name);

    switcher->activate_callback(&dynamic_channel, switcher->user_data);
    g_free(dynamic_channel.url);
    channel_switcher_overlay_hide(switcher);
}

static void open_direct_channel(ChannelSwitcherOverlay *switcher)
{
    if (switcher == NULL || switcher->direct_channel_entry == NULL) {
        return;
    }

    const char *text = gtk_editable_get_text(GTK_EDITABLE(switcher->direct_channel_entry));
    g_autofree char *channel = extract_twitch_channel_name(text);
    activate_dynamic_channel(switcher, channel);
}

static void on_direct_channel_activate(GtkEntry *entry, gpointer user_data)
{
    (void)entry;
    open_direct_channel(user_data);
}

static void on_direct_channel_icon_pressed(GtkEntry *entry, GtkEntryIconPosition icon_pos, gpointer user_data)
{
    (void)entry;

    if (icon_pos == GTK_ENTRY_ICON_SECONDARY) {
        open_direct_channel(user_data);
    }
}

static GtkWidget *create_image_picture(ChannelSwitcherOverlay *switcher, const char *url, int width, int height, const char *css_class)
{
    GtkWidget *image = gtk_picture_new();
    g_autoptr(GdkTexture) placeholder = create_placeholder_texture(width, height);

    gtk_widget_add_css_class(image, css_class);
    gtk_widget_set_focusable(image, FALSE);
    gtk_widget_set_size_request(image, width, height);
    gtk_widget_set_halign(image, GTK_ALIGN_START);
    gtk_widget_set_valign(image, GTK_ALIGN_START);
    gtk_widget_set_hexpand(image, FALSE);
    gtk_widget_set_vexpand(image, FALSE);
    gtk_widget_set_overflow(image, GTK_OVERFLOW_HIDDEN);
    gtk_picture_set_content_fit(GTK_PICTURE(image), GTK_CONTENT_FIT_COVER);
    gtk_picture_set_can_shrink(GTK_PICTURE(image), TRUE);
    if (placeholder != NULL) {
        gtk_picture_set_paintable(GTK_PICTURE(image), GDK_PAINTABLE(placeholder));
    }
    load_remote_image(switcher, image, url, width, height);
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
    g_autofree char *count = twitch_stream_info_format_viewer_count(viewer_count);
    return g_strdup_printf("%s viewers", count);
}

static char *format_meta_text(TwitchStreamPreview *preview)
{
    g_autofree char *viewers = format_viewer_count(preview->viewer_count);
    g_autofree char *duration = twitch_stream_info_format_live_duration(preview->started_at);

    return g_strdup_printf("%s • %s", viewers, duration != NULL ? duration : "live");
}

static GtkWidget *create_channel_card(
    ChannelSwitcherOverlay *switcher,
    TwitchStreamPreview *preview,
    int card_width,
    int preview_width,
    int preview_height
)
{
    const AppSettingsChannel *channel = find_settings_channel(switcher, preview->channel);
    const char *label = channel != NULL && channel->label != NULL && channel->label[0] != '\0'
        ? channel->label
        : preview->display_name;

    GtkWidget *button = gtk_button_new();
    gtk_widget_add_css_class(button, "channel-switcher-item");
    gtk_widget_set_halign(button, GTK_ALIGN_START);
    gtk_widget_set_hexpand(button, FALSE);
    gtk_widget_set_size_request(button, card_width, -1);
    g_object_set_data_full(G_OBJECT(button), "channel-name", g_strdup(preview->channel), g_free);
    g_object_set_data_full(G_OBJECT(button), "channel-label", g_strdup(label), g_free);
    g_signal_connect(button, "clicked", G_CALLBACK(on_channel_button_clicked), switcher);

    GtkWidget *card = gtk_box_new(GTK_ORIENTATION_VERTICAL, 5);
    gtk_widget_set_halign(card, GTK_ALIGN_START);
    gtk_widget_set_hexpand(card, FALSE);
    gtk_widget_set_size_request(card, card_width, -1);

    GtkWidget *preview_frame = create_fixed_picture_frame(
        create_image_picture(
            switcher,
            preview->preview_url,
            preview_width,
            preview_height,
            "channel-switcher-preview"
        ),
        preview_width,
        preview_height
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

static void ensure_preview_cards(ChannelSwitcherOverlay *switcher)
{
    guint columns = 1;
    int card_width = CARD_WIDTH;
    int preview_width = PREVIEW_WIDTH;
    int preview_height = PREVIEW_HEIGHT;

    calculate_card_layout(switcher, &columns, &card_width, &preview_width, &preview_height);

    if (switcher->preview_cards == NULL) {
        switcher->preview_cards = g_ptr_array_new_with_free_func(g_object_unref);
    }

    if (switcher->previews == NULL) {
        return;
    }

    if (switcher->preview_cards->len == switcher->previews->len &&
        switcher->preview_card_columns == columns &&
        switcher->preview_card_width == card_width &&
        switcher->preview_width == preview_width &&
        switcher->preview_height == preview_height) {
        return;
    }

    clear_preview_cards(switcher);
    switcher->preview_card_columns = columns;
    switcher->preview_card_width = card_width;
    switcher->preview_width = preview_width;
    switcher->preview_height = preview_height;
    for (guint i = 0; i < switcher->previews->len; i++) {
        TwitchStreamPreview *preview = g_ptr_array_index(switcher->previews, i);
        GtkWidget *card = create_channel_card(switcher, preview, card_width, preview_width, preview_height);
        g_ptr_array_add(switcher->preview_cards, g_object_ref_sink(card));
    }
}

static void render_live_channels(ChannelSwitcherOverlay *switcher)
{
    if (switcher->previews == NULL || switcher->previews->len == 0) {
        show_status(switcher, "No configured channels are live");
        return;
    }

    ensure_preview_cards(switcher);

    const char *filter = switcher->search_entry != NULL
        ? gtk_editable_get_text(GTK_EDITABLE(switcher->search_entry))
        : "";
    guint visible_count = 0;

    clear_grid(switcher);
    guint columns = switcher->preview_card_columns > 0 ? switcher->preview_card_columns : get_grid_columns(switcher);
    for (guint i = 0; i < switcher->previews->len && i < switcher->preview_cards->len; i++) {
        TwitchStreamPreview *preview = g_ptr_array_index(switcher->previews, i);
        if (!preview_matches_filter(preview, filter)) {
            continue;
        }

        gtk_grid_attach(
            GTK_GRID(switcher->grid),
            g_ptr_array_index(switcher->preview_cards, i),
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

static gboolean apply_search_filter(gpointer user_data)
{
    ChannelSwitcherOverlay *switcher = user_data;

    switcher->search_debounce_source = 0;
    render_live_channels(switcher);

    return G_SOURCE_REMOVE;
}

static void on_search_changed(GtkEditable *editable, gpointer user_data)
{
    (void)editable;
    ChannelSwitcherOverlay *switcher = user_data;

    remove_source_if_active(&switcher->search_debounce_source);
    if (switcher->panel == NULL || !gtk_widget_get_visible(switcher->panel) || switcher->previews == NULL) {
        return;
    }

    switcher->search_debounce_source = g_timeout_add(SEARCH_DEBOUNCE_MS, apply_search_filter, switcher);
}

static void activate_first_visible_channel(ChannelSwitcherOverlay *switcher)
{
    if (switcher == NULL || switcher->grid == NULL) {
        return;
    }

    for (GtkWidget *child = gtk_widget_get_first_child(switcher->grid);
         child != NULL;
         child = gtk_widget_get_next_sibling(child)) {
        if (GTK_IS_BUTTON(child)) {
            g_signal_emit_by_name(child, "clicked");
            return;
        }
    }
}

static void on_search_activate(GtkSearchEntry *entry, gpointer user_data)
{
    (void)entry;
    ChannelSwitcherOverlay *switcher = user_data;

    remove_source_if_active(&switcher->search_debounce_source);
    render_live_channels(switcher);
    activate_first_visible_channel(switcher);
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
    clear_preview_cards(switcher);
    switcher->previews = previews != NULL ? g_ptr_array_ref(previews) : NULL;
    switcher->cached_at_us = g_get_monotonic_time();
    render_live_channels(switcher);

    live_fetch_callback_data_free(data);
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

static void start_live_channel_fetch(ChannelSwitcherOverlay *switcher, char **channels, guint channel_count, gboolean allow_cache)
{
    g_autofree char *channels_key = build_channels_cache_key(channels, channel_count);

    if (channel_count == 0) {
        if (switcher->previews != NULL) {
            g_ptr_array_unref(switcher->previews);
            switcher->previews = NULL;
        }
        clear_preview_cards(switcher);
        clear_image_cache(switcher);
        g_clear_pointer(&switcher->cached_channels_key, g_free);
        switcher->cached_at_us = 0;
        show_status(switcher, "No channels configured");
        return;
    }

    if (allow_cache && has_fresh_cache(switcher, channels_key)) {
        render_live_channels(switcher);
        return;
    }

    if (switcher->previews != NULL) {
        g_ptr_array_unref(switcher->previews);
        switcher->previews = NULL;
    }
    clear_preview_cards(switcher);
    clear_image_cache(switcher);
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

static void on_channel_list_fetched(GObject *source_object, GAsyncResult *result, gpointer user_data)
{
    (void)source_object;
    ChannelListFetchCallbackData *data = user_data;
    ChannelSwitcherOverlay *switcher = data->switcher;
    g_autoptr(GError) error = NULL;
    g_auto(GStrv) channels = NULL;
    guint channel_count = 0;

    channels = twitch_channel_list_fetch_finish(result, &channel_count, &error);

    if (data->generation != switcher->generation || switcher->panel == NULL) {
        g_free(data);
        return;
    }

    g_clear_object(&switcher->cancel);

    if (error != NULL) {
        if (!g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            g_debug("channel list fetch failed: %s", error->message);
            show_status(switcher, error->message);
        }
        g_free(data);
        return;
    }

    start_live_channel_fetch(switcher, channels, channel_count, TRUE);

    g_free(data);
}

static void on_close_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    channel_switcher_overlay_hide(user_data);
}

static void on_settings_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    ChannelSwitcherOverlay *switcher = user_data;
    ChannelSwitcherSettingsCallback callback = switcher->settings_callback;
    gpointer callback_data = switcher->settings_user_data;

    channel_switcher_overlay_hide(switcher);
    if (callback != NULL) {
        callback(callback_data);
    }
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
    gpointer user_data,
    ChannelSwitcherSettingsCallback settings_callback,
    gpointer settings_user_data
)
{
    install_css();

    ChannelSwitcherOverlay *switcher = g_new0(ChannelSwitcherOverlay, 1);
    switcher->overlay = overlay;
    g_object_add_weak_pointer(G_OBJECT(switcher->overlay), (gpointer *)&switcher->overlay);
    switcher->settings = settings;
    switcher->activate_callback = activate_callback;
    switcher->user_data = user_data;
    switcher->settings_callback = settings_callback;
    switcher->settings_user_data = settings_user_data;
    switcher->preview_cards = g_ptr_array_new_with_free_func(g_object_unref);
    switcher->image_cache = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, g_object_unref);
    switcher->image_waiters = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, (GDestroyNotify)g_ptr_array_unref);

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
    gtk_widget_set_halign(switcher->panel, GTK_ALIGN_CENTER);
    gtk_widget_set_valign(switcher->panel, GTK_ALIGN_START);
    gtk_widget_set_hexpand(switcher->panel, FALSE);
    gtk_widget_set_visible(switcher->panel, FALSE);

    GtkWidget *header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    gtk_widget_add_css_class(header, "channel-switcher-header");
    switcher->search_entry = gtk_search_entry_new();
    g_object_add_weak_pointer(G_OBJECT(switcher->search_entry), (gpointer *)&switcher->search_entry);
    gtk_widget_add_css_class(switcher->search_entry, "channel-switcher-search");
    gtk_search_entry_set_placeholder_text(GTK_SEARCH_ENTRY(switcher->search_entry), "Filter live channels");
    g_signal_connect(switcher->search_entry, "changed", G_CALLBACK(on_search_changed), switcher);
    g_signal_connect(switcher->search_entry, "activate", G_CALLBACK(on_search_activate), switcher);
    switcher->direct_channel_entry = gtk_entry_new();
    g_object_add_weak_pointer(G_OBJECT(switcher->direct_channel_entry), (gpointer *)&switcher->direct_channel_entry);
    gtk_widget_add_css_class(switcher->direct_channel_entry, "channel-switcher-open-entry");
    gtk_entry_set_placeholder_text(GTK_ENTRY(switcher->direct_channel_entry), "Channel or Twitch URL");
    gtk_entry_set_icon_from_icon_name(GTK_ENTRY(switcher->direct_channel_entry), GTK_ENTRY_ICON_SECONDARY, "media-playback-start-symbolic");
    gtk_entry_set_icon_tooltip_text(GTK_ENTRY(switcher->direct_channel_entry), GTK_ENTRY_ICON_SECONDARY, "Open channel");
    gtk_widget_set_tooltip_text(switcher->direct_channel_entry, "Enter a channel name or Twitch URL");
    g_signal_connect(switcher->direct_channel_entry, "activate", G_CALLBACK(on_direct_channel_activate), switcher);
    g_signal_connect(switcher->direct_channel_entry, "icon-press", G_CALLBACK(on_direct_channel_icon_pressed), switcher);
    GtkWidget *header_spacer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_set_hexpand(header_spacer, TRUE);
    GtkWidget *settings_button = gtk_button_new();
    gtk_button_set_child(GTK_BUTTON(settings_button), player_settings_icon_new());
    gtk_widget_add_css_class(settings_button, "channel-switcher-action");
    gtk_widget_set_tooltip_text(settings_button, "Edit channels");
    g_signal_connect(settings_button, "clicked", G_CALLBACK(on_settings_clicked), switcher);
    GtkWidget *close_button = gtk_button_new();
    gtk_button_set_child(GTK_BUTTON(close_button), player_window_icon_new(PLAYER_WINDOW_ICON_CLOSE));
    gtk_widget_add_css_class(close_button, "channel-switcher-close");
    gtk_widget_set_tooltip_text(close_button, "Close");
    g_signal_connect(close_button, "clicked", G_CALLBACK(on_close_clicked), switcher);
    GtkWidget *input_separator = gtk_label_new("|");
    gtk_widget_add_css_class(input_separator, "channel-switcher-header-separator");
    gtk_box_append(GTK_BOX(header), switcher->search_entry);
    gtk_box_append(GTK_BOX(header), input_separator);
    gtk_box_append(GTK_BOX(header), switcher->direct_channel_entry);
    gtk_box_append(GTK_BOX(header), header_spacer);
    gtk_box_append(GTK_BOX(header), settings_button);
    gtk_box_append(GTK_BOX(header), close_button);
    gtk_box_append(GTK_BOX(switcher->panel), header);

    switcher->scroller = gtk_scrolled_window_new();
    g_object_add_weak_pointer(G_OBJECT(switcher->scroller), (gpointer *)&switcher->scroller);
    gtk_widget_add_css_class(switcher->scroller, "channel-switcher-scroller");
    gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(switcher->scroller), GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_scrolled_window_set_propagate_natural_width(GTK_SCROLLED_WINDOW(switcher->scroller), TRUE);
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
    clear_preview_cards(switcher);
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
    remove_source_if_active(&switcher->search_debounce_source);
    if (switcher->search_entry != NULL) {
        gtk_editable_set_text(GTK_EDITABLE(switcher->search_entry), "");
    }
    if (switcher->direct_channel_entry != NULL) {
        gtk_editable_set_text(GTK_EDITABLE(switcher->direct_channel_entry), "");
    }
    remove_source_if_active(&switcher->search_debounce_source);
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
    switcher->cancel = g_cancellable_new();
    ChannelListFetchCallbackData *data = g_new0(ChannelListFetchCallbackData, 1);
    data->switcher = switcher;
    data->generation = switcher->generation;
    twitch_channel_list_fetch_async(
        switcher->settings,
        switcher->cancel,
        on_channel_list_fetched,
        data
    );
}

void channel_switcher_overlay_hide(ChannelSwitcherOverlay *switcher)
{
    if (switcher == NULL) {
        return;
    }

    switcher->generation++;
    remove_source_if_active(&switcher->search_debounce_source);
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
    if (switcher->direct_channel_entry != NULL) {
        gtk_editable_set_text(GTK_EDITABLE(switcher->direct_channel_entry), "");
    }
    clear_grid(switcher);
    clear_image_cache(switcher);
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
    clear_preview_cards(switcher);
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
    switcher->direct_channel_entry = NULL;
    switcher->overlay = NULL;
    /* Async image/network callbacks can still carry this pointer briefly after
     * cancellation. Keep the tiny shell allocated, matching the player lifetime
     * pattern used for queued mpv callbacks. */
}
