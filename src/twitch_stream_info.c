#define G_LOG_DOMAIN "twitch-stream-info"

#include "twitch_stream_info.h"

#include <json-glib/json-glib.h>
#include <libsoup/soup.h>
#include <string.h>

#define TWITCH_GQL_URI "https://gql.twitch.tv/gql"
#define TWITCH_GQL_CLIENT_ID "kimne78kx3ncx6brgo4mv6wki5h1ko"
#define TWITCH_GQL_QUERY "query($login:String!){user(login:$login){stream{title}}}"

typedef struct {
    char *channel;
} FetchTitleData;

static void fetch_title_data_free(FetchTitleData *data)
{
    if (data == NULL) {
        return;
    }

    g_free(data->channel);
    g_free(data);
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

static char *fetch_stream_title(const char *channel, GCancellable *cancel, GError **error)
{
    g_autoptr(SoupSession) session = soup_session_new();
    g_autoptr(SoupMessage) message = soup_message_new("POST", TWITCH_GQL_URI);
    g_autofree char *body = build_stream_title_request_body(channel);
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
    return parse_stream_title_response(response_data, response_size, error);
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
