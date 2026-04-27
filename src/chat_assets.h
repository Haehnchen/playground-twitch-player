#pragma once

#include <gtk/gtk.h>

typedef struct ChatAssets ChatAssets;

ChatAssets *chat_assets_new(void);
void chat_assets_free(ChatAssets *assets);

void chat_assets_insert_message_text(
    ChatAssets *assets,
    GtkTextBuffer *buffer,
    GtkTextView *view,
    GtkTextIter *iter,
    const char *message,
    const char *emotes
);
