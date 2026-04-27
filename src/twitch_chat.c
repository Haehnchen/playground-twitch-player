#define G_LOG_DOMAIN "twitch-chat"

#include "twitch_chat.h"

#include <gio/gio.h>
#include <string.h>

#define CHAT_CONNECT_TIMEOUT_SECONDS 15
#define CHAT_RECONNECT_DELAY_MS 3000
#define CHAT_RECONNECT_POLL_MS 100

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
    TwitchChatLine line;
    char *display_name;
    char *message;
    char *color;
    char *emotes;
    char *reply_display_name;
    char *reply_message;
} ChatLineData;

typedef struct {
    char *display_name;
    char *message;
    char *color;
    char *emotes;
    char *reply_display_name;
    char *reply_message;
} ParsedPrivmsg;

static gboolean emit_line_on_main(gpointer user_data)
{
    ChatLineData *data = user_data;

    if (data->line_func != NULL) {
        data->line.display_name = data->display_name;
        data->line.message = data->message;
        data->line.color = data->color;
        data->line.emotes = data->emotes;
        data->line.reply_display_name = data->reply_display_name;
        data->line.reply_message = data->reply_message;
        data->line_func(&data->line, data->user_data);
    }

    g_free(data->display_name);
    g_free(data->message);
    g_free(data->color);
    g_free(data->emotes);
    g_free(data->reply_display_name);
    g_free(data->reply_message);
    g_free(data);
    return G_SOURCE_REMOVE;
}

static void emit_status(TwitchChatClient *client, guint generation, const char *message)
{
    (void)generation;

    ChatLineData *data = g_new0(ChatLineData, 1);
    data->line_func = client->line_func;
    data->user_data = client->user_data;
    data->line.kind = TWITCH_CHAT_LINE_STATUS;
    data->message = g_strdup(message);

    g_main_context_invoke(NULL, emit_line_on_main, data);
}

static void emit_message(TwitchChatClient *client, guint generation, ParsedPrivmsg *message)
{
    (void)generation;

    ChatLineData *data = g_new0(ChatLineData, 1);
    data->line_func = client->line_func;
    data->user_data = client->user_data;
    data->line.kind = TWITCH_CHAT_LINE_MESSAGE;
    data->display_name = g_strdup(message->display_name);
    data->message = g_strdup(message->message);
    data->color = g_strdup(message->color);
    data->emotes = g_strdup(message->emotes);
    data->reply_display_name = g_strdup(message->reply_display_name);
    data->reply_message = g_strdup(message->reply_message);

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
    GString *decoded = g_string_new(NULL);
    for (const char *p = raw; *p != '\0'; p++) {
        if (*p == '\\' && p[1] != '\0') {
            p++;
            switch (*p) {
            case 's':
                g_string_append_c(decoded, ' ');
                break;
            case ':':
                g_string_append_c(decoded, ';');
                break;
            case '\\':
                g_string_append_c(decoded, '\\');
                break;
            case 'r':
                g_string_append_c(decoded, '\r');
                break;
            case 'n':
                g_string_append_c(decoded, '\n');
                break;
            default:
                g_string_append_c(decoded, *p);
                break;
            }
        } else {
            g_string_append_c(decoded, *p);
        }
    }

    return g_string_free(decoded, FALSE);
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

static void parsed_privmsg_free(ParsedPrivmsg *message)
{
    if (message == NULL) {
        return;
    }

    g_free(message->display_name);
    g_free(message->message);
    g_free(message->color);
    g_free(message->emotes);
    g_free(message->reply_display_name);
    g_free(message->reply_message);
    g_free(message);
}

static ParsedPrivmsg *parse_privmsg(const char *line)
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
    g_autofree char *color = NULL;
    g_autofree char *emotes = NULL;
    g_autofree char *reply_display_name = NULL;
    g_autofree char *reply_message = NULL;
    if (line[0] == '@') {
        const char *tags_end = strchr(line, ' ');
        if (tags_end != NULL) {
            g_autofree char *tags = g_strndup(line + 1, tags_end - line - 1);
            name = extract_irc_tag(tags, "display-name");
            color = extract_irc_tag(tags, "color");
            emotes = extract_irc_tag(tags, "emotes");
            reply_display_name = extract_irc_tag(tags, "reply-parent-display-name");
            reply_message = extract_irc_tag(tags, "reply-parent-msg-body");
        }
    }

    if (name == NULL) {
        name = extract_sender_from_prefix(line);
    }

    g_autofree char *message = g_strdup(message_start);
    g_strchomp(message);

    ParsedPrivmsg *parsed = g_new0(ParsedPrivmsg, 1);
    parsed->display_name = g_steal_pointer(&name);
    parsed->message = g_steal_pointer(&message);
    parsed->color = g_steal_pointer(&color);
    parsed->emotes = g_steal_pointer(&emotes);
    parsed->reply_display_name = g_steal_pointer(&reply_display_name);
    parsed->reply_message = g_steal_pointer(&reply_message);
    return parsed;
}

static gboolean wait_before_reconnect(GCancellable *cancel)
{
    for (guint elapsed = 0; elapsed < CHAT_RECONNECT_DELAY_MS; elapsed += CHAT_RECONNECT_POLL_MS) {
        if (g_cancellable_is_cancelled(cancel)) {
            return FALSE;
        }

        g_usleep(CHAT_RECONNECT_POLL_MS * 1000);
    }

    return !g_cancellable_is_cancelled(cancel);
}

static gboolean run_chat_session(ChatWorkerData *data, GSocketClient *socket_client)
{
    g_autoptr(GError) error = NULL;

    g_autoptr(GSocketConnection) connection = g_socket_client_connect_to_host(
        socket_client,
        "irc.chat.twitch.tv",
        6697,
        data->cancel,
        &error
    );

    if (connection == NULL) {
        if (g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            return FALSE;
        }

        if (!g_cancellable_is_cancelled(data->cancel)) {
            g_autofree char *line = g_strdup_printf("Chat-Verbindung fehlgeschlagen: %s", error->message);
            emit_status(data->client, data->generation, line);
        }
        return TRUE;
    }

    GSocket *socket = g_socket_connection_get_socket(connection);
    if (socket != NULL) {
        g_socket_set_timeout(socket, 0);
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
        if (g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            return FALSE;
        }

        if (!g_cancellable_is_cancelled(data->cancel)) {
            g_autofree char *line = g_strdup_printf("Chat-Login fehlgeschlagen: %s", error->message);
            emit_status(data->client, data->generation, line);
        }
        return TRUE;
    }

    {
        g_autofree char *line = g_strdup_printf("Chat verbunden: #%s", data->channel);
        emit_status(data->client, data->generation, line);
    }

    while (!g_cancellable_is_cancelled(data->cancel)) {
        gsize length = 0;
        g_clear_error(&error);
        g_autofree char *line = g_data_input_stream_read_line_utf8(data_input, &length, data->cancel, &error);

        if (line == NULL) {
            if (error != NULL && g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
                return FALSE;
            }

            if (error != NULL && !g_cancellable_is_cancelled(data->cancel)) {
                g_autofree char *message = g_strdup_printf("Chat getrennt: %s", error->message);
                emit_status(data->client, data->generation, message);
            } else if (!g_cancellable_is_cancelled(data->cancel)) {
                emit_status(data->client, data->generation, "Chat getrennt");
            }
            return TRUE;
        }

        if (g_str_has_prefix(line, "PING ")) {
            g_autofree char *pong = g_strdup_printf("PONG %s", line + 5);
            g_clear_error(&error);
            if (!write_irc_line(output, pong, data->cancel, &error)) {
                if (g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
                    return FALSE;
                }

                if (!g_cancellable_is_cancelled(data->cancel)) {
                    g_autofree char *message = g_strdup_printf("Chat getrennt: %s", error->message);
                    emit_status(data->client, data->generation, message);
                }
                return TRUE;
            }
            continue;
        }

        ParsedPrivmsg *chat_line = parse_privmsg(line);
        if (chat_line != NULL) {
            emit_message(data->client, data->generation, chat_line);
            parsed_privmsg_free(chat_line);
        }
    }

    return FALSE;
}

static gpointer chat_worker(gpointer user_data)
{
    ChatWorkerData *data = user_data;
    g_autoptr(GSocketClient) socket_client = g_socket_client_new();

    g_socket_client_set_tls(socket_client, TRUE);
    g_socket_client_set_timeout(socket_client, CHAT_CONNECT_TIMEOUT_SECONDS);

    while (!g_cancellable_is_cancelled(data->cancel)) {
        gboolean reconnect = run_chat_session(data, socket_client);

        if (!reconnect || g_cancellable_is_cancelled(data->cancel)) {
            break;
        }

        emit_status(data->client, data->generation, "Chat verbindet in 3 Sekunden neu ...");
        if (!wait_before_reconnect(data->cancel)) {
            break;
        }
    }

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
