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

typedef void (*TwitchChatLineFunc)(const TwitchChatLine *line, gpointer user_data);

TwitchChatClient *twitch_chat_client_new(TwitchChatLineFunc line_func, gpointer user_data);
void twitch_chat_client_start(TwitchChatClient *client, const char *channel);
void twitch_chat_client_stop(TwitchChatClient *client);
void twitch_chat_client_free(TwitchChatClient *client);
