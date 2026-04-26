#define G_LOG_DOMAIN "twitch-chat"

#include "twitch_chat.h"

#include <gio/gio.h>
#include <string.h>

struct TwitchChatClient {
    TwitchChatLineFunc line_func;
    gpointer user_data;
    GThread *thread;
    GCancellable *cancel;
    guint generation;
};

typedef struct {
    TwitchChatClient *client;
    char *channel;
    guint generation;
    GCancellable *cancel;
} ChatWorkerData;

typedef struct {
    TwitchChatLineFunc line_func;
    gpointer user_data;
    char *line;
} ChatLineData;

static gboolean emit_line_on_main(gpointer user_data)
{
    ChatLineData *data = user_data;

    if (data->line_func != NULL) {
        data->line_func(data->line, data->user_data);
    }

    g_free(data->line);
    g_free(data);
    return G_SOURCE_REMOVE;
}

static void emit_line(TwitchChatClient *client, guint generation, const char *line)
{
    (void)generation;

    ChatLineData *data = g_new0(ChatLineData, 1);
    data->line_func = client->line_func;
    data->user_data = client->user_data;
    data->line = g_strdup(line);

    g_main_context_invoke(NULL, emit_line_on_main, data);
}

static gboolean write_irc_line(GOutputStream *output, const char *line, GCancellable *cancel, GError **error)
{
    g_autofree char *wire_line = g_strdup_printf("%s\r\n", line);
    gsize written = 0;

    return g_output_stream_write_all(output, wire_line, strlen(wire_line), &written, cancel, error);
}

static char *extract_irc_tag(const char *tags, const char *key)
{
    g_autofree char *needle = g_strdup_printf("%s=", key);
    const char *start = strstr(tags, needle);

    if (start == NULL) {
        return NULL;
    }

    start += strlen(needle);
    const char *end = strchr(start, ';');
    if (end == NULL) {
        end = start + strlen(start);
    }

    if (end == start) {
        return NULL;
    }

    g_autofree char *raw = g_strndup(start, end - start);
    char *decoded = g_strdup(raw);
    for (char *p = decoded; *p != '\0'; p++) {
        if (p[0] == '\\' && p[1] == 's') {
            p[0] = ' ';
            memmove(p + 1, p + 2, strlen(p + 2) + 1);
        }
    }

    return decoded;
}

static char *extract_sender_from_prefix(const char *line)
{
    const char *prefix = strchr(line, ':');
    if (prefix == NULL) {
        return g_strdup("chat");
    }

    prefix++;
    const char *bang = strchr(prefix, '!');
    if (bang == NULL || bang == prefix) {
        return g_strdup("chat");
    }

    return g_strndup(prefix, bang - prefix);
}

static char *parse_privmsg(const char *line)
{
    const char *message_start = strstr(line, " PRIVMSG ");
    if (message_start == NULL) {
        return NULL;
    }

    message_start = strstr(message_start, " :");
    if (message_start == NULL) {
        return NULL;
    }
    message_start += 2;

    g_autofree char *name = NULL;
    if (line[0] == '@') {
        const char *tags_end = strchr(line, ' ');
        if (tags_end != NULL) {
            g_autofree char *tags = g_strndup(line + 1, tags_end - line - 1);
            name = extract_irc_tag(tags, "display-name");
        }
    }

    if (name == NULL) {
        name = extract_sender_from_prefix(line);
    }

    g_autofree char *message = g_strdup(message_start);
    g_strchomp(message);

    return g_strdup_printf("%s: %s", name, message);
}

static gpointer chat_worker(gpointer user_data)
{
    ChatWorkerData *data = user_data;
    g_autoptr(GSocketClient) socket_client = g_socket_client_new();
    g_autoptr(GError) error = NULL;

    g_socket_client_set_tls(socket_client, TRUE);
    g_socket_client_set_timeout(socket_client, 15);

    g_autoptr(GSocketConnection) connection = g_socket_client_connect_to_host(
        socket_client,
        "irc.chat.twitch.tv",
        6697,
        data->cancel,
        &error
    );

    if (connection == NULL) {
        if (!g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            g_autofree char *line = g_strdup_printf("Chat-Verbindung fehlgeschlagen: %s", error->message);
            emit_line(data->client, data->generation, line);
        }
        goto done;
    }

    GOutputStream *output = g_io_stream_get_output_stream(G_IO_STREAM(connection));
    GInputStream *input = g_io_stream_get_input_stream(G_IO_STREAM(connection));
    g_autoptr(GDataInputStream) data_input = g_data_input_stream_new(input);

    g_autofree char *nick = g_strdup_printf("justinfan%u", g_random_int_range(10000, 999999));
    g_autofree char *nick_line = g_strdup_printf("NICK %s", nick);
    g_autofree char *join_line = g_strdup_printf("JOIN #%s", data->channel);

    if (!write_irc_line(output, "CAP REQ :twitch.tv/tags twitch.tv/commands", data->cancel, &error) ||
        !write_irc_line(output, nick_line, data->cancel, &error) ||
        !write_irc_line(output, join_line, data->cancel, &error)) {
        if (!g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            g_autofree char *line = g_strdup_printf("Chat-Login fehlgeschlagen: %s", error->message);
            emit_line(data->client, data->generation, line);
        }
        goto done;
    }

    {
        g_autofree char *line = g_strdup_printf("Chat verbunden: #%s", data->channel);
        emit_line(data->client, data->generation, line);
    }

    while (!g_cancellable_is_cancelled(data->cancel)) {
        gsize length = 0;
        g_clear_error(&error);
        g_autofree char *line = g_data_input_stream_read_line_utf8(data_input, &length, data->cancel, &error);

        if (line == NULL) {
            if (error != NULL && !g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
                g_autofree char *message = g_strdup_printf("Chat getrennt: %s", error->message);
                emit_line(data->client, data->generation, message);
            }
            break;
        }

        if (g_str_has_prefix(line, "PING ")) {
            g_autofree char *pong = g_strdup_printf("PONG %s", line + 5);
            g_clear_error(&error);
            write_irc_line(output, pong, data->cancel, &error);
            continue;
        }

        g_autofree char *chat_line = parse_privmsg(line);
        if (chat_line != NULL) {
            emit_line(data->client, data->generation, chat_line);
        }
    }

done:
    g_clear_object(&data->cancel);
    g_free(data->channel);
    g_free(data);
    return NULL;
}

TwitchChatClient *twitch_chat_client_new(TwitchChatLineFunc line_func, gpointer user_data)
{
    TwitchChatClient *client = g_new0(TwitchChatClient, 1);
    client->line_func = line_func;
    client->user_data = user_data;
    return client;
}

void twitch_chat_client_start(TwitchChatClient *client, const char *channel)
{
    if (client == NULL || channel == NULL || channel[0] == '\0') {
        return;
    }

    twitch_chat_client_stop(client);
    client->generation++;
    client->cancel = g_cancellable_new();

    ChatWorkerData *data = g_new0(ChatWorkerData, 1);
    data->client = client;
    data->channel = g_ascii_strdown(channel, -1);
    data->generation = client->generation;
    data->cancel = g_object_ref(client->cancel);

    client->thread = g_thread_new("twitch-chat", chat_worker, data);
}

void twitch_chat_client_stop(TwitchChatClient *client)
{
    if (client == NULL) {
        return;
    }

    if (client->cancel != NULL) {
        g_cancellable_cancel(client->cancel);
    }

    if (client->thread != NULL) {
        g_thread_join(client->thread);
        client->thread = NULL;
    }

    g_clear_object(&client->cancel);
}

void twitch_chat_client_free(TwitchChatClient *client)
{
    if (client == NULL) {
        return;
    }

    twitch_chat_client_stop(client);
    g_free(client);
}
