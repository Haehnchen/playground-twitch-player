#pragma once

#include <glib.h>

typedef struct TwitchChatClient TwitchChatClient;

typedef enum {
    TWITCH_CHAT_LINE_STATUS,
    TWITCH_CHAT_LINE_MESSAGE,
} TwitchChatLineKind;

typedef struct {
    TwitchChatLineKind kind;
    const char *display_name;
    const char *message;
    const char *color;
    const char *emotes;
    const char *reply_display_name;
    const char *reply_message;
} TwitchChatLine;

/**
 * TwitchChatLineFunc:
 * @line: Chat status or message data. The pointer is only valid during the callback.
 * @user_data: User data passed to twitch_chat_client_new().
 *
 * Called on the main context whenever the chat client emits a status line or
 * parsed Twitch chat message.
 */
typedef void (*TwitchChatLineFunc)(const TwitchChatLine *line, gpointer user_data);

/**
 * twitch_chat_client_new:
 * @line_func: Callback for incoming status and message lines.
 * @user_data: User data passed to @line_func.
 *
 * Creates a read-only anonymous Twitch IRC client.
 *
 * Returns: A new chat client owned by the caller.
 */
TwitchChatClient *twitch_chat_client_new(TwitchChatLineFunc line_func, gpointer user_data);

/**
 * twitch_chat_client_start:
 * @client: A Twitch chat client.
 * @channel: Twitch channel login to join.
 *
 * Starts or restarts the client for @channel. Channel names are normalized to
 * lowercase before connecting.
 */
void twitch_chat_client_start(TwitchChatClient *client, const char *channel);

/**
 * twitch_chat_client_stop:
 * @client: A Twitch chat client.
 *
 * Cancels the active connection without waiting for the worker thread to exit.
 */
void twitch_chat_client_stop(TwitchChatClient *client);

/**
 * twitch_chat_client_free:
 * @client: A Twitch chat client, or NULL.
 *
 * Stops and frees the chat client.
 */
void twitch_chat_client_free(TwitchChatClient *client);
