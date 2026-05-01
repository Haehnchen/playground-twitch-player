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

typedef struct {
    char *title;
    guint viewer_count;
} TwitchCurrentStream;

typedef struct {
    char *channel;
    char *display_name;
} TwitchFollowedChannel;

typedef enum {
    TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED,
} TwitchStreamInfoError;

#define TWITCH_STREAM_INFO_ERROR twitch_stream_info_error_quark()

GQuark twitch_stream_info_error_quark(void);

void twitch_stream_preview_free(TwitchStreamPreview *preview);
void twitch_current_stream_free(TwitchCurrentStream *stream);
void twitch_followed_channel_free(TwitchFollowedChannel *channel);

G_DEFINE_AUTOPTR_CLEANUP_FUNC(TwitchCurrentStream, twitch_current_stream_free)

char *twitch_stream_info_format_viewer_count(guint viewer_count);
char *twitch_stream_info_format_current_stream_title(const TwitchCurrentStream *stream);

/**
 * twitch_stream_info_fetch_current_stream_async:
 * @channel: Twitch channel login.
 * @cancel: Optional cancellable.
 * @callback: Completion callback.
 * @user_data: User data passed to @callback.
 *
 * Fetches current Twitch stream metadata asynchronously. The result may be NULL
 * when the channel or stream is unavailable.
 */
void twitch_stream_info_fetch_current_stream_async(
    const char *channel,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
);

/**
 * twitch_stream_info_fetch_current_stream_finish:
 * @result: Async result passed to the completion callback.
 * @error: Return location for a GError, or NULL.
 *
 * Finishes twitch_stream_info_fetch_current_stream_async().
 *
 * Returns: (transfer full): The stream metadata, or NULL when unavailable or an error occurred.
 */
TwitchCurrentStream *twitch_stream_info_fetch_current_stream_finish(GAsyncResult *result, GError **error);

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

/**
 * twitch_stream_info_fetch_followed_channels_async:
 * @client_id: Twitch application client ID.
 * @oauth_token: Twitch user access token with user:read:follows.
 * @cancel: Optional cancellable.
 * @callback: Completion callback.
 * @user_data: User data passed to @callback.
 *
 * Fetches the channels followed by the token owner. This uses Helix and needs
 * the user:read:follows OAuth scope.
 */
void twitch_stream_info_fetch_followed_channels_async(
    const char *client_id,
    const char *oauth_token,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
);

GPtrArray *twitch_stream_info_fetch_followed_channels(
    const char *client_id,
    const char *oauth_token,
    GCancellable *cancel,
    GError **error
);

/**
 * twitch_stream_info_fetch_followed_channels_finish:
 * @result: Async result passed to the completion callback.
 * @error: Return location for a GError, or NULL.
 *
 * Finishes twitch_stream_info_fetch_followed_channels_async().
 *
 * Returns: (transfer full): A GPtrArray of TwitchFollowedChannel entries.
 */
GPtrArray *twitch_stream_info_fetch_followed_channels_finish(GAsyncResult *result, GError **error);
