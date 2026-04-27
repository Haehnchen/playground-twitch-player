#pragma once

#include <gio/gio.h>

/**
 * twitch_stream_info_fetch_title_async:
 * @channel: Twitch channel login.
 * @cancel: Optional cancellable.
 * @callback: Completion callback.
 * @user_data: User data passed to @callback.
 *
 * Fetches the current Twitch stream title asynchronously. The result may be
 * NULL when the channel or stream is unavailable.
 */
void twitch_stream_info_fetch_title_async(
    const char *channel,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
);

/**
 * twitch_stream_info_fetch_title_finish:
 * @result: Async result passed to the completion callback.
 * @error: Return location for a GError, or NULL.
 *
 * Finishes twitch_stream_info_fetch_title_async().
 *
 * Returns: The stream title, or NULL when no title is available or an error occurred.
 */
char *twitch_stream_info_fetch_title_finish(GAsyncResult *result, GError **error);
