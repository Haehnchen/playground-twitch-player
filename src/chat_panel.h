#pragma once

#include <gtk/gtk.h>

#include "twitch_chat.h"

typedef struct ChatPanelPrivate ChatPanelPrivate;

typedef struct {
    GtkWidget *widget;
    TwitchChatClient *client;
    ChatPanelPrivate *priv;
} ChatPanel;

/**
 * chat_panel_new:
 * @width: Initial requested panel width in pixels.
 *
 * Creates the chat panel widget and its backing Twitch chat client.
 *
 * Returns: A new chat panel owned by the caller.
 */
ChatPanel *chat_panel_new(int width);

/**
 * chat_panel_start:
 * @panel: A chat panel.
 * @channel: Twitch channel login to display.
 *
 * Clears the current chat view and starts loading chat for @channel.
 */
void chat_panel_start(ChatPanel *panel, const char *channel);

/**
 * chat_panel_free:
 * @panel: A chat panel, or NULL.
 *
 * Stops the chat client, removes pending UI callbacks, and frees the panel.
 */
void chat_panel_free(ChatPanel *panel);
