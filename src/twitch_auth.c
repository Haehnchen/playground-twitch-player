#define G_LOG_DOMAIN "twitch-auth"

#include "twitch_auth.h"

#include <json-glib/json-glib.h>
#include <libsoup/soup.h>
#include <string.h>

#define TWITCH_DEVICE_URI "https://id.twitch.tv/oauth2/device"
#define TWITCH_TOKEN_URI "https://id.twitch.tv/oauth2/token"
#define TWITCH_FOLLOWS_SCOPE "user:read:follows"
#define TWITCH_DEVICE_GRANT "urn:ietf:params:oauth:grant-type:device_code"

typedef struct {
    char *client_id;
} DeviceCodeData;

typedef struct {
    char *client_id;
    char *device_code;
    guint expires_in;
    guint interval;
} PollTokenData;

typedef struct {
    guint status;
    char *body;
} AuthResponse;

static void device_code_data_free(DeviceCodeData *data)
{
    if (data == NULL) {
        return;
    }

    g_free(data->client_id);
    g_free(data);
}

static void poll_token_data_free(PollTokenData *data)
{
    if (data == NULL) {
        return;
    }

    g_free(data->client_id);
    g_free(data->device_code);
    g_free(data);
}

static void auth_response_free(AuthResponse *response)
{
    if (response == NULL) {
        return;
    }

    g_free(response->body);
    g_free(response);
}

G_DEFINE_AUTOPTR_CLEANUP_FUNC(AuthResponse, auth_response_free)

void twitch_auth_device_code_free(TwitchAuthDeviceCode *code)
{
    if (code == NULL) {
        return;
    }

    g_free(code->device_code);
    g_free(code->user_code);
    g_free(code->verification_uri);
    g_free(code);
}

void twitch_auth_token_free(TwitchAuthToken *token)
{
    if (token == NULL) {
        return;
    }

    g_free(token->access_token);
    g_free(token->refresh_token);
    g_free(token);
}

static void append_form_pair(GString *form, const char *name, const char *value)
{
    if (form->len > 0) {
        g_string_append_c(form, '&');
    }

    g_autofree char *escaped_name = g_uri_escape_string(name, NULL, TRUE);
    g_autofree char *escaped_value = g_uri_escape_string(value != NULL ? value : "", NULL, TRUE);
    g_string_append_printf(form, "%s=%s", escaped_name, escaped_value);
}

static AuthResponse *post_auth_form(const char *uri, const char *body, GCancellable *cancel, GError **error)
{
    g_autoptr(SoupSession) session = soup_session_new();
    g_autoptr(SoupMessage) message = soup_message_new("POST", uri);
    g_autoptr(GBytes) body_bytes = g_bytes_new_static(body, strlen(body));

    g_object_set(session, "timeout", 15, NULL);
    SoupMessageHeaders *request_headers = soup_message_get_request_headers(message);
    soup_message_headers_append(request_headers, "Accept", "application/json");
    soup_message_set_request_body_from_bytes(message, "application/x-www-form-urlencoded", body_bytes);

    g_autoptr(GBytes) response_bytes = soup_session_send_and_read(session, message, cancel, error);
    if (response_bytes == NULL) {
        return NULL;
    }

    gsize response_size = 0;
    const char *response_data = g_bytes_get_data(response_bytes, &response_size);

    AuthResponse *response = g_new0(AuthResponse, 1);
    response->status = soup_message_get_status(message);
    response->body = g_strndup(response_data, response_size);
    return response;
}

static JsonObject *parse_json_object(JsonParser *parser, const char *json, GError **error)
{
    if (!json_parser_load_from_data(parser, json, -1, error)) {
        return NULL;
    }

    JsonNode *root = json_parser_get_root(parser);
    if (root == NULL || !JSON_NODE_HOLDS_OBJECT(root)) {
        g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED, "Twitch returned invalid JSON");
        return NULL;
    }

    return json_node_get_object(root);
}

static const char *json_string_or_null(JsonObject *object, const char *name)
{
    JsonNode *node = json_object_get_member(object, name);
    if (node == NULL || JSON_NODE_HOLDS_NULL(node) || !JSON_NODE_HOLDS_VALUE(node)) {
        return NULL;
    }

    return json_node_get_string(node);
}

static guint json_uint_or_zero(JsonObject *object, const char *name)
{
    JsonNode *node = json_object_get_member(object, name);
    if (node == NULL || JSON_NODE_HOLDS_NULL(node) || !JSON_NODE_HOLDS_VALUE(node)) {
        return 0;
    }

    gint64 value = json_node_get_int(node);
    return value > 0 && value <= G_MAXUINT ? (guint)value : 0;
}

static char *parse_auth_error_message(const char *json)
{
    g_autoptr(JsonParser) parser = json_parser_new();
    g_autoptr(GError) error = NULL;
    JsonObject *object = parse_json_object(parser, json, &error);
    if (object == NULL) {
        return g_strdup("");
    }

    const char *message = json_string_or_null(object, "message");
    if (message != NULL && message[0] != '\0') {
        return g_strdup(message);
    }

    const char *error_name = json_string_or_null(object, "error");
    return error_name != NULL ? g_strdup(error_name) : g_strdup("");
}

static TwitchAuthDeviceCode *parse_device_code_response(const char *json, GError **error)
{
    g_autoptr(JsonParser) parser = json_parser_new();
    JsonObject *object = parse_json_object(parser, json, error);
    if (object == NULL) {
        return NULL;
    }

    const char *device_code = json_string_or_null(object, "device_code");
    const char *user_code = json_string_or_null(object, "user_code");
    const char *verification_uri = json_string_or_null(object, "verification_uri");
    if (device_code == NULL || user_code == NULL || verification_uri == NULL) {
        g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED, "Twitch did not return a device code");
        return NULL;
    }

    TwitchAuthDeviceCode *code = g_new0(TwitchAuthDeviceCode, 1);
    code->device_code = g_strdup(device_code);
    code->user_code = g_strdup(user_code);
    code->verification_uri = g_strdup(verification_uri);
    code->expires_in = json_uint_or_zero(object, "expires_in");
    code->interval = json_uint_or_zero(object, "interval");
    if (code->interval == 0) {
        code->interval = 5;
    }
    return code;
}

static TwitchAuthToken *parse_token_response(const char *json, GError **error)
{
    g_autoptr(JsonParser) parser = json_parser_new();
    JsonObject *object = parse_json_object(parser, json, error);
    if (object == NULL) {
        return NULL;
    }

    const char *access_token = json_string_or_null(object, "access_token");
    if (access_token == NULL || access_token[0] == '\0') {
        g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED, "Twitch did not return an access token");
        return NULL;
    }

    TwitchAuthToken *token = g_new0(TwitchAuthToken, 1);
    token->access_token = g_strdup(access_token);
    token->refresh_token = g_strdup(json_string_or_null(object, "refresh_token"));
    token->expires_in = json_uint_or_zero(object, "expires_in");
    return token;
}

static gboolean sleep_poll_interval(guint interval, GCancellable *cancel, GError **error)
{
    guint remaining_ms = MAX(interval, 1) * 1000;
    while (remaining_ms > 0) {
        if (g_cancellable_set_error_if_cancelled(cancel, error)) {
            return FALSE;
        }

        guint chunk_ms = MIN(remaining_ms, 100);
        g_usleep(chunk_ms * 1000);
        remaining_ms -= chunk_ms;
    }

    return TRUE;
}

static TwitchAuthDeviceCode *request_device_code(DeviceCodeData *data, GCancellable *cancel, GError **error)
{
    g_autoptr(GString) form = g_string_new(NULL);
    append_form_pair(form, "client_id", data->client_id);
    append_form_pair(form, "scopes", TWITCH_FOLLOWS_SCOPE);

    g_autoptr(AuthResponse) response = post_auth_form(TWITCH_DEVICE_URI, form->str, cancel, error);
    if (response == NULL) {
        return NULL;
    }

    if (response->status < 200 || response->status >= 300) {
        g_autofree char *message = parse_auth_error_message(response->body);
        g_set_error(
            error,
            G_IO_ERROR,
            G_IO_ERROR_FAILED,
            "Twitch auth returned HTTP %u%s%s",
            response->status,
            message[0] != '\0' ? ": " : "",
            message
        );
        return NULL;
    }

    return parse_device_code_response(response->body, error);
}

static TwitchAuthToken *poll_device_token(PollTokenData *data, GCancellable *cancel, GError **error)
{
    guint interval = data->interval > 0 ? data->interval : 5;
    gint64 deadline_us = g_get_monotonic_time() + MAX(data->expires_in, 1) * G_USEC_PER_SEC;

    while (g_get_monotonic_time() < deadline_us) {
        if (g_cancellable_set_error_if_cancelled(cancel, error)) {
            return NULL;
        }

        g_autoptr(GString) form = g_string_new(NULL);
        append_form_pair(form, "client_id", data->client_id);
        append_form_pair(form, "scope", TWITCH_FOLLOWS_SCOPE);
        append_form_pair(form, "device_code", data->device_code);
        append_form_pair(form, "grant_type", TWITCH_DEVICE_GRANT);

        g_autoptr(AuthResponse) response = post_auth_form(TWITCH_TOKEN_URI, form->str, cancel, error);
        if (response == NULL) {
            return NULL;
        }

        if (response->status >= 200 && response->status < 300) {
            return parse_token_response(response->body, error);
        }

        g_autofree char *message = parse_auth_error_message(response->body);
        if (g_strcmp0(message, "authorization_pending") == 0) {
            if (!sleep_poll_interval(interval, cancel, error)) {
                return NULL;
            }
            continue;
        }
        if (g_strcmp0(message, "slow_down") == 0) {
            interval += 5;
            if (!sleep_poll_interval(interval, cancel, error)) {
                return NULL;
            }
            continue;
        }

        g_set_error(
            error,
            G_IO_ERROR,
            G_IO_ERROR_FAILED,
            "Twitch authorization failed%s%s",
            message[0] != '\0' ? ": " : "",
            message
        );
        return NULL;
    }

    g_set_error(error, G_IO_ERROR, G_IO_ERROR_TIMED_OUT, "Twitch authorization timed out");
    return NULL;
}

static void request_device_code_worker(GTask *task, gpointer source_object, gpointer task_data, GCancellable *cancel)
{
    (void)source_object;
    DeviceCodeData *data = task_data;
    g_autoptr(GError) error = NULL;
    TwitchAuthDeviceCode *code = request_device_code(data, cancel, &error);

    if (error != NULL) {
        g_task_return_error(task, g_steal_pointer(&error));
        return;
    }

    g_task_return_pointer(task, code, (GDestroyNotify)twitch_auth_device_code_free);
}

static void poll_device_token_worker(GTask *task, gpointer source_object, gpointer task_data, GCancellable *cancel)
{
    (void)source_object;
    PollTokenData *data = task_data;
    g_autoptr(GError) error = NULL;
    TwitchAuthToken *token = poll_device_token(data, cancel, &error);

    if (error != NULL) {
        g_task_return_error(task, g_steal_pointer(&error));
        return;
    }

    g_task_return_pointer(task, token, (GDestroyNotify)twitch_auth_token_free);
}

void twitch_auth_request_device_code_async(
    const char *client_id,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
)
{
    g_return_if_fail(client_id != NULL);
    g_return_if_fail(client_id[0] != '\0');

    DeviceCodeData *data = g_new0(DeviceCodeData, 1);
    data->client_id = g_strdup(client_id);

    GTask *task = g_task_new(NULL, cancel, callback, user_data);
    g_task_set_task_data(task, data, (GDestroyNotify)device_code_data_free);
    g_task_run_in_thread(task, request_device_code_worker);
    g_object_unref(task);
}

TwitchAuthDeviceCode *twitch_auth_request_device_code_finish(GAsyncResult *result, GError **error)
{
    g_return_val_if_fail(g_task_is_valid(result, NULL), NULL);

    return g_task_propagate_pointer(G_TASK(result), error);
}

void twitch_auth_poll_device_token_async(
    const char *client_id,
    const TwitchAuthDeviceCode *code,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
)
{
    g_return_if_fail(client_id != NULL);
    g_return_if_fail(client_id[0] != '\0');
    g_return_if_fail(code != NULL);
    g_return_if_fail(code->device_code != NULL);
    g_return_if_fail(code->device_code[0] != '\0');

    PollTokenData *data = g_new0(PollTokenData, 1);
    data->client_id = g_strdup(client_id);
    data->device_code = g_strdup(code->device_code);
    data->expires_in = code->expires_in;
    data->interval = code->interval;

    GTask *task = g_task_new(NULL, cancel, callback, user_data);
    g_task_set_task_data(task, data, (GDestroyNotify)poll_token_data_free);
    g_task_run_in_thread(task, poll_device_token_worker);
    g_object_unref(task);
}

TwitchAuthToken *twitch_auth_poll_device_token_finish(GAsyncResult *result, GError **error)
{
    g_return_val_if_fail(g_task_is_valid(result, NULL), NULL);

    return g_task_propagate_pointer(G_TASK(result), error);
}
