#pragma once

#include <glib.h>

typedef struct TwitchChatClient TwitchChatClient;

typedef void (*TwitchChatLineFunc)(const char *line, gpointer user_data);

TwitchChatClient *twitch_chat_client_new(TwitchChatLineFunc line_func, gpointer user_data);
void twitch_chat_client_start(TwitchChatClient *client, const char *channel);
void twitch_chat_client_stop(TwitchChatClient *client);
void twitch_chat_client_free(TwitchChatClient *client);
