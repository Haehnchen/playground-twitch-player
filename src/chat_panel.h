#pragma once

#include <gtk/gtk.h>

#include "twitch_chat.h"

typedef struct {
    GtkWidget *widget;
    TwitchChatClient *client;
} ChatPanel;

ChatPanel *chat_panel_new(int width);
void chat_panel_start(ChatPanel *panel, const char *channel);
void chat_panel_free(ChatPanel *panel);
