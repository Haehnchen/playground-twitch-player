#pragma once

#include <gio/gio.h>
#include <glib.h>

typedef struct {
    char *channel;
    char *display_name;
    char *title;
    char *avatar_url;
    char *preview_url;
    char *started_at;
    char *category_name;
    guint viewer_count;
} TwitchStreamPreview;

void twitch_stream_preview_free(TwitchStreamPreview *preview);

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

/**
 * twitch_stream_info_fetch_live_channels_async:
 * @channels: Twitch channel logins.
 * @channel_count: Number of entries in @channels.
 * @cancel: Optional cancellable.
 * @callback: Completion callback.
 * @user_data: User data passed to @callback.
 *
 * Fetches live stream cards for the supplied channels. Offline and unknown
 * channels are omitted from the result.
 */
void twitch_stream_info_fetch_live_channels_async(
    const char * const *channels,
    guint channel_count,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
);

/**
 * twitch_stream_info_fetch_live_channels_finish:
 * @result: Async result passed to the completion callback.
 * @error: Return location for a GError, or NULL.
 *
 * Finishes twitch_stream_info_fetch_live_channels_async().
 *
 * Returns: (transfer full): A GPtrArray of TwitchStreamPreview entries.
 */
GPtrArray *twitch_stream_info_fetch_live_channels_finish(GAsyncResult *result, GError **error);
