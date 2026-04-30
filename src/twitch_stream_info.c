#define G_LOG_DOMAIN "twitch-stream-info"

#include "twitch_stream_info.h"

#include <json-glib/json-glib.h>
#include <libsoup/soup.h>
#include <string.h>

#define TWITCH_GQL_URI "https://gql.twitch.tv/gql"
#define TWITCH_GQL_CLIENT_ID "kimne78kx3ncx6brgo4mv6wki5h1ko"
#define TWITCH_GQL_QUERY "query($login:String!){user(login:$login){stream{title}}}"
#define TWITCH_GQL_LIVE_CHANNELS_QUERY "query($logins:[String!]!){users(logins:$logins){login displayName profileImageURL(width:70) stream{title viewersCount createdAt previewImageURL(width:240,height:135) game{name}}}}"

typedef struct {
    char *channel;
} FetchTitleData;

typedef struct {
    char **channels;
    guint channel_count;
} FetchLiveChannelsData;

static void fetch_title_data_free(FetchTitleData *data)
{
    if (data == NULL) {
        return;
    }

    g_free(data->channel);
    g_free(data);
}

static void fetch_live_channels_data_free(FetchLiveChannelsData *data)
{
    if (data == NULL) {
        return;
    }

    g_strfreev(data->channels);
    g_free(data);
}

void twitch_stream_preview_free(TwitchStreamPreview *preview)
{
    if (preview == NULL) {
        return;
    }

    g_free(preview->channel);
    g_free(preview->display_name);
    g_free(preview->title);
    g_free(preview->avatar_url);
    g_free(preview->preview_url);
    g_free(preview->started_at);
    g_free(preview->category_name);
    g_free(preview);
}

static char *build_stream_title_request_body(const char *channel)
{
    g_autoptr(JsonBuilder) builder = json_builder_new();

    /* Build via JsonBuilder so channel names are always escaped correctly. */
    json_builder_begin_object(builder);
    json_builder_set_member_name(builder, "query");
    json_builder_add_string_value(builder, TWITCH_GQL_QUERY);
    json_builder_set_member_name(builder, "variables");
    json_builder_begin_object(builder);
    json_builder_set_member_name(builder, "login");
    json_builder_add_string_value(builder, channel);
    json_builder_end_object(builder);
    json_builder_end_object(builder);

    g_autoptr(JsonNode) root = json_builder_get_root(builder);
    g_autoptr(JsonGenerator) generator = json_generator_new();
    json_generator_set_root(generator, root);
    return json_generator_to_data(generator, NULL);
}

static char *build_live_channels_request_body(const char * const *channels, guint channel_count)
{
    g_autoptr(JsonBuilder) builder = json_builder_new();

    json_builder_begin_object(builder);
    json_builder_set_member_name(builder, "query");
    json_builder_add_string_value(builder, TWITCH_GQL_LIVE_CHANNELS_QUERY);
    json_builder_set_member_name(builder, "variables");
    json_builder_begin_object(builder);
    json_builder_set_member_name(builder, "logins");
    json_builder_begin_array(builder);

    for (guint i = 0; i < channel_count; i++) {
        if (channels[i] != NULL && channels[i][0] != '\0') {
            json_builder_add_string_value(builder, channels[i]);
        }
    }

    json_builder_end_array(builder);
    json_builder_end_object(builder);
    json_builder_end_object(builder);

    g_autoptr(JsonNode) root = json_builder_get_root(builder);
    g_autoptr(JsonGenerator) generator = json_generator_new();
    json_generator_set_root(generator, root);
    return json_generator_to_data(generator, NULL);
}

static char *parse_stream_title_response(const char *json, gsize length, GError **error)
{
    g_autoptr(JsonParser) parser = json_parser_new();

    if (!json_parser_load_from_data(parser, json, length, error)) {
        return NULL;
    }

    JsonNode *root = json_parser_get_root(parser);
    if (root == NULL || !JSON_NODE_HOLDS_OBJECT(root)) {
        return NULL;
    }

    JsonObject *root_object = json_node_get_object(root);
    JsonNode *data_node = json_object_get_member(root_object, "data");
    if (data_node == NULL || !JSON_NODE_HOLDS_OBJECT(data_node)) {
        return NULL;
    }

    JsonObject *data = json_node_get_object(data_node);
    JsonNode *user_node = json_object_get_member(data, "user");
    if (user_node == NULL || JSON_NODE_HOLDS_NULL(user_node) || !JSON_NODE_HOLDS_OBJECT(user_node)) {
        /* Twitch returns a null user for unknown channels. */
        return NULL;
    }

    JsonObject *user = json_node_get_object(user_node);
    JsonNode *stream_node = json_object_get_member(user, "stream");
    if (stream_node == NULL || JSON_NODE_HOLDS_NULL(stream_node) || !JSON_NODE_HOLDS_OBJECT(stream_node)) {
        /* Known channel, but currently offline or without a live stream. */
        return NULL;
    }

    JsonObject *stream = json_node_get_object(stream_node);
    const char *title = json_object_get_string_member_with_default(stream, "title", NULL);
    return title != NULL ? g_strdup(title) : NULL;
}

static char *post_twitch_gql_request(const char *body, GCancellable *cancel, GError **error)
{
    g_autoptr(SoupSession) session = soup_session_new();
    g_autoptr(SoupMessage) message = soup_message_new("POST", TWITCH_GQL_URI);
    g_autoptr(GBytes) body_bytes = g_bytes_new_static(body, strlen(body));

    g_object_set(session, "timeout", 15, NULL);
    SoupMessageHeaders *request_headers = soup_message_get_request_headers(message);
    soup_message_headers_append(request_headers, "Client-ID", TWITCH_GQL_CLIENT_ID);
    soup_message_headers_append(request_headers, "Accept", "application/json");
    soup_message_set_request_body_from_bytes(message, "application/json", body_bytes);

    g_autoptr(GBytes) response = soup_session_send_and_read(session, message, cancel, error);
    if (response == NULL) {
        return NULL;
    }

    guint status = soup_message_get_status(message);
    if (status < 200 || status >= 300) {
        g_set_error(
            error,
            G_IO_ERROR,
            G_IO_ERROR_FAILED,
            "Twitch returned HTTP %u",
            status
        );
        return NULL;
    }

    gsize response_size = 0;
    const char *response_data = g_bytes_get_data(response, &response_size);
    return g_strndup(response_data, response_size);
}

static char *fetch_stream_title(const char *channel, GCancellable *cancel, GError **error)
{
    g_autofree char *body = build_stream_title_request_body(channel);
    g_autofree char *response = post_twitch_gql_request(body, cancel, error);

    if (response == NULL) {
        return NULL;
    }

    return parse_stream_title_response(response, strlen(response), error);
}

static const char *json_object_get_string_or_null(JsonObject *object, const char *member_name)
{
    JsonNode *node = json_object_get_member(object, member_name);

    if (node == NULL || JSON_NODE_HOLDS_NULL(node) || !JSON_NODE_HOLDS_VALUE(node)) {
        return NULL;
    }

    return json_node_get_string(node);
}

static guint json_object_get_uint_or_zero(JsonObject *object, const char *member_name)
{
    JsonNode *node = json_object_get_member(object, member_name);

    if (node == NULL || JSON_NODE_HOLDS_NULL(node) || !JSON_NODE_HOLDS_VALUE(node)) {
        return 0;
    }

    gint64 value = json_node_get_int(node);
    return value > 0 && value <= G_MAXUINT ? (guint)value : 0;
}

static gint compare_stream_previews_by_viewers(gconstpointer a, gconstpointer b)
{
    const TwitchStreamPreview *preview_a = *(TwitchStreamPreview * const *)a;
    const TwitchStreamPreview *preview_b = *(TwitchStreamPreview * const *)b;

    if (preview_a->viewer_count == preview_b->viewer_count) {
        return g_ascii_strcasecmp(preview_a->display_name, preview_b->display_name);
    }

    return preview_a->viewer_count > preview_b->viewer_count ? -1 : 1;
}

static GPtrArray *parse_live_channels_response(const char *json, gsize length, GError **error)
{
    g_autoptr(JsonParser) parser = json_parser_new();

    if (!json_parser_load_from_data(parser, json, length, error)) {
        return NULL;
    }

    GPtrArray *previews = g_ptr_array_new_with_free_func((GDestroyNotify)twitch_stream_preview_free);
    JsonNode *root = json_parser_get_root(parser);
    if (root == NULL || !JSON_NODE_HOLDS_OBJECT(root)) {
        return previews;
    }

    JsonObject *root_object = json_node_get_object(root);
    JsonNode *data_node = json_object_get_member(root_object, "data");
    if (data_node == NULL || !JSON_NODE_HOLDS_OBJECT(data_node)) {
        return previews;
    }

    JsonObject *data = json_node_get_object(data_node);
    JsonNode *users_node = json_object_get_member(data, "users");
    if (users_node == NULL || !JSON_NODE_HOLDS_ARRAY(users_node)) {
        return previews;
    }

    JsonArray *users = json_node_get_array(users_node);
    guint users_length = json_array_get_length(users);
    for (guint i = 0; i < users_length; i++) {
        JsonNode *user_node = json_array_get_element(users, i);
        if (user_node == NULL || JSON_NODE_HOLDS_NULL(user_node) || !JSON_NODE_HOLDS_OBJECT(user_node)) {
            continue;
        }

        JsonObject *user = json_node_get_object(user_node);
        JsonNode *stream_node = json_object_get_member(user, "stream");
        if (stream_node == NULL || JSON_NODE_HOLDS_NULL(stream_node) || !JSON_NODE_HOLDS_OBJECT(stream_node)) {
            continue;
        }

        const char *login = json_object_get_string_or_null(user, "login");
        if (login == NULL || login[0] == '\0') {
            continue;
        }

        JsonObject *stream = json_node_get_object(stream_node);
        const char *title = json_object_get_string_or_null(stream, "title");
        const char *preview_url = json_object_get_string_or_null(stream, "previewImageURL");
        const char *started_at = json_object_get_string_or_null(stream, "createdAt");
        const char *display_name = json_object_get_string_or_null(user, "displayName");
        const char *avatar_url = json_object_get_string_or_null(user, "profileImageURL");
        const char *category_name = NULL;
        JsonNode *game_node = json_object_get_member(stream, "game");
        if (game_node != NULL && JSON_NODE_HOLDS_OBJECT(game_node)) {
            category_name = json_object_get_string_or_null(json_node_get_object(game_node), "name");
        }

        TwitchStreamPreview *preview = g_new0(TwitchStreamPreview, 1);
        preview->channel = g_ascii_strdown(login, -1);
        preview->display_name = g_strdup(display_name != NULL && display_name[0] != '\0' ? display_name : login);
        preview->title = title != NULL ? g_strdup(title) : g_strdup("");
        preview->avatar_url = avatar_url != NULL ? g_strdup(avatar_url) : NULL;
        preview->preview_url = preview_url != NULL ? g_strdup(preview_url) : NULL;
        preview->started_at = started_at != NULL ? g_strdup(started_at) : NULL;
        preview->category_name = category_name != NULL ? g_strdup(category_name) : NULL;
        preview->viewer_count = json_object_get_uint_or_zero(stream, "viewersCount");
        g_ptr_array_add(previews, preview);
    }

    g_ptr_array_sort(previews, compare_stream_previews_by_viewers);
    return previews;
}

static GPtrArray *fetch_live_channels(FetchLiveChannelsData *data, GCancellable *cancel, GError **error)
{
    g_autofree char *body = build_live_channels_request_body((const char * const *)data->channels, data->channel_count);
    g_autofree char *response = post_twitch_gql_request(body, cancel, error);

    if (response == NULL) {
        return NULL;
    }

    return parse_live_channels_response(response, strlen(response), error);
}

static void fetch_title_worker(GTask *task, gpointer source_object, gpointer task_data, GCancellable *cancel)
{
    (void)source_object;
    FetchTitleData *data = task_data;
    g_autoptr(GError) error = NULL;
    char *title = fetch_stream_title(data->channel, cancel, &error);

    if (error != NULL) {
        g_task_return_error(task, g_steal_pointer(&error));
        return;
    }

    g_task_return_pointer(task, title, g_free);
}

static void fetch_live_channels_worker(GTask *task, gpointer source_object, gpointer task_data, GCancellable *cancel)
{
    (void)source_object;
    FetchLiveChannelsData *data = task_data;
    g_autoptr(GError) error = NULL;
    GPtrArray *previews = fetch_live_channels(data, cancel, &error);

    if (error != NULL) {
        g_task_return_error(task, g_steal_pointer(&error));
        return;
    }

    g_task_return_pointer(task, previews, (GDestroyNotify)g_ptr_array_unref);
}

void twitch_stream_info_fetch_title_async(
    const char *channel,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
)
{
    g_return_if_fail(channel != NULL);
    g_return_if_fail(channel[0] != '\0');

    FetchTitleData *data = g_new0(FetchTitleData, 1);
    data->channel = g_ascii_strdown(channel, -1);

    GTask *task = g_task_new(NULL, cancel, callback, user_data);
    g_task_set_task_data(task, data, (GDestroyNotify)fetch_title_data_free);
    /* Network I/O runs off the GTK main thread. */
    g_task_run_in_thread(task, fetch_title_worker);
    g_object_unref(task);
}

char *twitch_stream_info_fetch_title_finish(GAsyncResult *result, GError **error)
{
    g_return_val_if_fail(g_task_is_valid(result, NULL), NULL);

    return g_task_propagate_pointer(G_TASK(result), error);
}

void twitch_stream_info_fetch_live_channels_async(
    const char * const *channels,
    guint channel_count,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
)
{
    FetchLiveChannelsData *data = g_new0(FetchLiveChannelsData, 1);
    data->channel_count = channel_count;
    data->channels = g_new0(char *, channel_count + 1);

    for (guint i = 0; i < channel_count; i++) {
        data->channels[i] = channels[i] != NULL ? g_ascii_strdown(channels[i], -1) : g_strdup("");
    }

    GTask *task = g_task_new(NULL, cancel, callback, user_data);
    g_task_set_task_data(task, data, (GDestroyNotify)fetch_live_channels_data_free);
    g_task_run_in_thread(task, fetch_live_channels_worker);
    g_object_unref(task);
}

GPtrArray *twitch_stream_info_fetch_live_channels_finish(GAsyncResult *result, GError **error)
{
    g_return_val_if_fail(g_task_is_valid(result, NULL), NULL);

    return g_task_propagate_pointer(G_TASK(result), error);
}
