#pragma once

#include <gtk/gtk.h>

typedef struct ChatAssets ChatAssets;

/**
 * chat_assets_new:
 *
 * Creates a chat asset cache for inline emote images.
 *
 * Returns: A new asset cache owned by the caller.
 */
ChatAssets *chat_assets_new(void);

/**
 * chat_assets_free:
 * @assets: A chat asset cache, or NULL.
 *
 * Frees the cache and any cached image references.
 */
void chat_assets_free(ChatAssets *assets);

/**
 * chat_assets_insert_message_text:
 * @assets: Chat asset cache.
 * @buffer: Text buffer receiving the message.
 * @view: Text view used to attach inline image widgets.
 * @iter: Insertion position.
 * @message: Twitch chat message text.
 * @emotes: Twitch emote range tag, or NULL.
 *
 * Inserts message text and replaces Twitch emote ranges with inline images.
 */
void chat_assets_insert_message_text(
    ChatAssets *assets,
    GtkTextBuffer *buffer,
    GtkTextView *view,
    GtkTextIter *iter,
    const char *message,
    const char *emotes
);
