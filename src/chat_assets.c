#define G_LOG_DOMAIN "chat-assets"

#include "chat_assets.h"

#define CHAT_EMOTE_SIZE 18

struct ChatAssets {
    GHashTable *image_cache;
};

typedef struct {
    GtkPicture *picture;
    ChatAssets *assets;
    char *url;
} ImageLoadData;

typedef struct {
    guint start;
    guint end;
    char *id;
} EmoteRange;

static void image_load_data_free(ImageLoadData *data)
{
    if (data == NULL) {
        return;
    }

    g_clear_object(&data->picture);
    g_free(data->url);
    g_free(data);
}

static void on_image_loaded(GObject *source, GAsyncResult *result, gpointer user_data)
{
    ImageLoadData *data = user_data;
    g_autoptr(GError) error = NULL;
    char *contents = NULL;
    gsize length = 0;

    if (!g_file_load_contents_finish(G_FILE(source), result, &contents, &length, NULL, &error)) {
        g_debug("image load failed for %s: %s", data->url, error->message);
        image_load_data_free(data);
        return;
    }

    g_autoptr(GBytes) bytes = g_bytes_new_take(contents, length);
    g_autoptr(GdkTexture) texture = gdk_texture_new_from_bytes(bytes, &error);
    if (texture == NULL) {
        g_debug("image decode failed for %s: %s", data->url, error != NULL ? error->message : "unknown error");
        image_load_data_free(data);
        return;
    }

    g_hash_table_insert(data->assets->image_cache, g_strdup(data->url), g_object_ref(texture));
    gtk_picture_set_paintable(data->picture, GDK_PAINTABLE(texture));
    image_load_data_free(data);
}

static GtkWidget *create_inline_image(ChatAssets *assets, const char *url)
{
    GtkWidget *picture = gtk_picture_new();
    gtk_widget_add_css_class(picture, "chat-emote");
    gtk_widget_set_focusable(picture, FALSE);
    gtk_widget_set_size_request(picture, CHAT_EMOTE_SIZE, CHAT_EMOTE_SIZE);
    gtk_picture_set_content_fit(GTK_PICTURE(picture), GTK_CONTENT_FIT_CONTAIN);
    gtk_picture_set_can_shrink(GTK_PICTURE(picture), FALSE);

    GdkPaintable *cached = g_hash_table_lookup(assets->image_cache, url);
    if (cached != NULL) {
        gtk_picture_set_paintable(GTK_PICTURE(picture), cached);
        return picture;
    }

    ImageLoadData *data = g_new0(ImageLoadData, 1);
    data->picture = g_object_ref(GTK_PICTURE(picture));
    data->assets = assets;
    data->url = g_strdup(url);

    GFile *file = g_file_new_for_uri(url);
    g_file_load_contents_async(file, NULL, on_image_loaded, data);
    g_object_unref(file);

    return picture;
}

static void insert_emote(ChatAssets *assets, GtkTextBuffer *buffer, GtkTextView *view, GtkTextIter *iter, const char *url)
{
    GtkTextChildAnchor *anchor = gtk_text_child_anchor_new();

    gtk_text_buffer_insert_child_anchor(buffer, iter, anchor);
    gtk_text_view_add_child_at_anchor(view, create_inline_image(assets, url), anchor);
    g_object_unref(anchor);
}

static void emote_range_clear(gpointer data)
{
    EmoteRange *range = data;

    g_free(range->id);
}

static gint compare_emote_ranges(gconstpointer a, gconstpointer b)
{
    const EmoteRange *range_a = a;
    const EmoteRange *range_b = b;

    if (range_a->start == range_b->start) {
        return 0;
    }

    return range_a->start < range_b->start ? -1 : 1;
}

static GArray *parse_emote_ranges(const char *emotes)
{
    if (emotes == NULL || emotes[0] == '\0') {
        return NULL;
    }

    GArray *ranges = g_array_new(FALSE, FALSE, sizeof(EmoteRange));
    g_array_set_clear_func(ranges, emote_range_clear);

    g_auto(GStrv) emote_specs = g_strsplit(emotes, "/", -1);
    for (guint i = 0; emote_specs[i] != NULL; i++) {
        char *colon = strchr(emote_specs[i], ':');
        if (colon == NULL || colon == emote_specs[i] || colon[1] == '\0') {
            continue;
        }

        g_autofree char *id = g_strndup(emote_specs[i], colon - emote_specs[i]);
        g_auto(GStrv) positions = g_strsplit(colon + 1, ",", -1);
        for (guint j = 0; positions[j] != NULL; j++) {
            char *dash = strchr(positions[j], '-');
            if (dash == NULL || dash == positions[j] || dash[1] == '\0') {
                continue;
            }

            *dash = '\0';
            char *end_ptr = NULL;
            guint64 start = g_ascii_strtoull(positions[j], &end_ptr, 10);
            if (end_ptr == positions[j] || *end_ptr != '\0') {
                continue;
            }

            guint64 end = g_ascii_strtoull(dash + 1, &end_ptr, 10);
            if (end_ptr == dash + 1 || *end_ptr != '\0' || end < start || end > G_MAXUINT) {
                continue;
            }

            EmoteRange range = {
                .start = (guint)start,
                .end = (guint)end,
                .id = g_strdup(id),
            };
            g_array_append_val(ranges, range);
        }
    }

    if (ranges->len == 0) {
        g_array_unref(ranges);
        return NULL;
    }

    g_array_sort(ranges, compare_emote_ranges);
    return ranges;
}

static const char *utf8_offset_to_pointer_safe(const char *text, guint offset)
{
    const char *p = text;

    for (guint i = 0; i < offset; i++) {
        if (*p == '\0') {
            return NULL;
        }

        p = g_utf8_next_char(p);
    }

    return p;
}

ChatAssets *chat_assets_new(void)
{
    ChatAssets *assets = g_new0(ChatAssets, 1);

    assets->image_cache = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, g_object_unref);
    return assets;
}

void chat_assets_free(ChatAssets *assets)
{
    if (assets == NULL) {
        return;
    }

    g_clear_pointer(&assets->image_cache, g_hash_table_destroy);
    g_free(assets);
}

void chat_assets_insert_message_text(ChatAssets *assets, GtkTextBuffer *buffer, GtkTextView *view, GtkTextIter *iter, const char *message, const char *emotes)
{
    g_autoptr(GArray) ranges = parse_emote_ranges(emotes);

    if (message == NULL) {
        return;
    }

    if (ranges == NULL) {
        gtk_text_buffer_insert(buffer, iter, message, -1);
        return;
    }

    const char *cursor = message;
    for (guint i = 0; i < ranges->len; i++) {
        EmoteRange *range = &g_array_index(ranges, EmoteRange, i);
        const char *start = utf8_offset_to_pointer_safe(message, range->start);
        const char *end = utf8_offset_to_pointer_safe(message, range->end + 1);

        if (start == NULL || end == NULL || start < cursor || end < start) {
            continue;
        }

        if (start > cursor) {
            gtk_text_buffer_insert(buffer, iter, cursor, start - cursor);
        }

        g_autofree char *url = g_strdup_printf(
            "https://static-cdn.jtvnw.net/emoticons/v2/%s/default/dark/1.0",
            range->id
        );
        insert_emote(assets, buffer, view, iter, url);
        cursor = end;
    }

    if (*cursor != '\0') {
        gtk_text_buffer_insert(buffer, iter, cursor, -1);
    }
}
