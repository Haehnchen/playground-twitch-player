#define G_LOG_DOMAIN "twitch-channel-list"

#include "twitch_channel_list.h"

#include "twitch_auth.h"
#include "twitch_stream_info.h"

#define FOLLOWED_CHANNELS_CACHE_SECONDS 120

typedef struct {
    AppSettings *settings;
    char **manual_channels;
    guint manual_channel_count;
    char *oauth_token;
    char *refresh_token;
    gint64 oauth_expires_at;
} FetchChannelListData;

typedef struct {
    AppSettings *settings;
    char **channels;
    guint channel_count;
    TwitchAuthToken *refreshed_token;
    GError *error;
} ChannelListResult;

static GMutex followed_cache_mutex;
static GPtrArray *followed_channels_cache;
static gint64 followed_channels_cached_at_us;

static void fetch_channel_list_data_free(FetchChannelListData *data)
{
    if (data == NULL) {
        return;
    }

    g_strfreev(data->manual_channels);
    g_free(data->oauth_token);
    g_free(data->refresh_token);
    g_free(data);
}

static void channel_list_result_free(ChannelListResult *result)
{
    if (result == NULL) {
        return;
    }

    g_strfreev(result->channels);
    twitch_auth_token_free(result->refreshed_token);
    g_clear_error(&result->error);
    g_free(result);
}

static gint64 token_expires_at_from_expires_in(guint expires_in)
{
    return expires_in > 0 ? g_get_real_time() / G_USEC_PER_SEC + expires_in : 0;
}

static gboolean token_needs_refresh(gint64 expires_at)
{
    if (expires_at <= 0) {
        return FALSE;
    }

    return g_get_real_time() / G_USEC_PER_SEC + 60 >= expires_at;
}

static void add_unique_channel(GPtrArray *channels, const char *channel)
{
    if (channel == NULL || channel[0] == '\0') {
        return;
    }

    for (guint i = 0; i < channels->len; i++) {
        const char *existing = g_ptr_array_index(channels, i);
        if (g_ascii_strcasecmp(existing, channel) == 0) {
            return;
        }
    }

    g_ptr_array_add(channels, g_ascii_strdown(channel, -1));
}

static char **channels_array_from_ptr_array(GPtrArray *channels)
{
    char **result = g_new0(char *, channels->len + 1);

    for (guint i = 0; i < channels->len; i++) {
        result[i] = g_strdup(g_ptr_array_index(channels, i));
    }

    return result;
}

static char **collect_settings_channels(const AppSettings *settings, guint *channel_count_out)
{
    guint settings_channel_count = app_settings_get_channel_count(settings);
    char **channels = g_new0(char *, settings_channel_count + 1);
    guint out = 0;

    for (guint i = 0; i < settings_channel_count; i++) {
        const AppSettingsChannel *channel = app_settings_get_channel(settings, i);
        if (channel == NULL || channel->channel == NULL || channel->channel[0] == '\0') {
            continue;
        }

        channels[out++] = g_strdup(channel->channel);
    }

    *channel_count_out = out;
    return channels;
}

static GPtrArray *dup_fresh_followed_cache(void)
{
    GPtrArray *cache = NULL;
    gint64 now_us = g_get_monotonic_time();

    g_mutex_lock(&followed_cache_mutex);
    if (followed_channels_cache != NULL &&
        now_us - followed_channels_cached_at_us < FOLLOWED_CHANNELS_CACHE_SECONDS * G_USEC_PER_SEC) {
        cache = g_ptr_array_ref(followed_channels_cache);
    }
    g_mutex_unlock(&followed_cache_mutex);

    return cache;
}

static void store_followed_cache(GPtrArray *channels)
{
    g_mutex_lock(&followed_cache_mutex);
    if (followed_channels_cache != NULL) {
        g_ptr_array_unref(followed_channels_cache);
    }
    followed_channels_cache = channels != NULL ? g_ptr_array_ref(channels) : NULL;
    followed_channels_cached_at_us = g_get_monotonic_time();
    g_mutex_unlock(&followed_cache_mutex);
}

static ChannelListResult *build_channel_list_result(char **manual_channels, guint manual_channel_count, GPtrArray *followed_channels)
{
    GPtrArray *combined = g_ptr_array_new_with_free_func(g_free);

    for (guint i = 0; i < manual_channel_count; i++) {
        add_unique_channel(combined, manual_channels[i]);
    }

    if (followed_channels != NULL) {
        for (guint i = 0; i < followed_channels->len; i++) {
            TwitchFollowedChannel *followed = g_ptr_array_index(followed_channels, i);
            add_unique_channel(combined, followed != NULL ? followed->channel : NULL);
        }
    }

    ChannelListResult *result = g_new0(ChannelListResult, 1);
    result->channels = channels_array_from_ptr_array(combined);
    result->channel_count = combined->len;
    g_ptr_array_unref(combined);
    return result;
}

static gboolean refresh_access_token(
    FetchChannelListData *data,
    ChannelListResult *result,
    char **oauth_token,
    GCancellable *cancel,
    GError **error
)
{
    if (data->refresh_token == NULL || data->refresh_token[0] == '\0') {
        g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED, "Connect Twitch again to refresh followed channels");
        return FALSE;
    }

    g_autoptr(TwitchAuthToken) token = twitch_auth_refresh_token(
        TWITCH_AUTH_CLIENT_ID,
        data->refresh_token,
        cancel,
        error
    );
    if (token == NULL) {
        return FALSE;
    }

    g_free(*oauth_token);
    *oauth_token = g_strdup(token->access_token);
    result->refreshed_token = g_steal_pointer(&token);
    return TRUE;
}

static gboolean save_refreshed_token(ChannelListResult *result, GError **error)
{
    if (result->settings == NULL || result->refreshed_token == NULL) {
        return TRUE;
    }

    app_settings_set_twitch_auth_tokens(
        result->settings,
        result->refreshed_token->access_token,
        result->refreshed_token->refresh_token,
        token_expires_at_from_expires_in(result->refreshed_token->expires_in)
    );

    return app_settings_save(result->settings, error);
}

static ChannelListResult *fetch_channel_list(FetchChannelListData *data, GCancellable *cancel, GError **error)
{
    ChannelListResult *result = NULL;

    g_autoptr(GPtrArray) followed_channels = dup_fresh_followed_cache();
    if (followed_channels != NULL) {
        result = build_channel_list_result(data->manual_channels, data->manual_channel_count, followed_channels);
        result->settings = data->settings;
        return result;
    }

    const char *client_id = TWITCH_AUTH_CLIENT_ID;
    g_autofree char *oauth_token = g_strdup(data->oauth_token);
    if (client_id == NULL || client_id[0] == '\0') {
        if (data->manual_channel_count > 0) {
            g_debug("followed channels enabled but Twitch credentials are incomplete");
            result = build_channel_list_result(data->manual_channels, data->manual_channel_count, NULL);
            result->settings = data->settings;
            return result;
        }

        g_set_error(
            error,
            G_IO_ERROR,
            G_IO_ERROR_FAILED,
            "Connect Twitch in Settings to load followed channels"
        );
        return NULL;
    }

    result = g_new0(ChannelListResult, 1);
    result->settings = data->settings;

    if ((oauth_token == NULL || oauth_token[0] == '\0' || token_needs_refresh(data->oauth_expires_at)) &&
        !refresh_access_token(data, result, &oauth_token, cancel, error)) {
        if (data->manual_channel_count > 0 &&
            (error == NULL || *error == NULL || !g_error_matches(*error, G_IO_ERROR, G_IO_ERROR_CANCELLED))) {
            if (error != NULL && *error != NULL) {
                g_debug("Twitch token refresh failed: %s", (*error)->message);
                g_clear_error(error);
            }
            ChannelListResult *manual_result = build_channel_list_result(data->manual_channels, data->manual_channel_count, NULL);
            manual_result->settings = data->settings;
            channel_list_result_free(result);
            return manual_result;
        }
        channel_list_result_free(result);
        return NULL;
    }

    followed_channels = twitch_stream_info_fetch_followed_channels(client_id, oauth_token, cancel, error);
    if (followed_channels == NULL &&
        error != NULL &&
        g_error_matches(*error, TWITCH_STREAM_INFO_ERROR, TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED) &&
        result->refreshed_token == NULL) {
        g_clear_error(error);
        if (refresh_access_token(data, result, &oauth_token, cancel, error)) {
            followed_channels = twitch_stream_info_fetch_followed_channels(client_id, oauth_token, cancel, error);
        }
    }

    if (followed_channels == NULL) {
        if (data->manual_channel_count > 0 &&
            (error == NULL || *error == NULL || !g_error_matches(*error, G_IO_ERROR, G_IO_ERROR_CANCELLED))) {
            if (error != NULL && *error != NULL) {
                g_debug("followed channel fetch failed: %s", (*error)->message);
                g_clear_error(error);
            }
            ChannelListResult *manual_result = build_channel_list_result(data->manual_channels, data->manual_channel_count, NULL);
            manual_result->settings = data->settings;
            manual_result->refreshed_token = g_steal_pointer(&result->refreshed_token);
            channel_list_result_free(result);
            return manual_result;
        }
        channel_list_result_free(result);
        return NULL;
    }

    store_followed_cache(followed_channels);
    ChannelListResult *channels_result = build_channel_list_result(
        data->manual_channels,
        data->manual_channel_count,
        followed_channels
    );
    channels_result->settings = data->settings;
    channels_result->refreshed_token = g_steal_pointer(&result->refreshed_token);
    channel_list_result_free(result);
    return channels_result;
}

static void fetch_channel_list_worker(GTask *task, gpointer source_object, gpointer task_data, GCancellable *cancel)
{
    (void)source_object;
    FetchChannelListData *data = task_data;
    g_autoptr(GError) error = NULL;
    ChannelListResult *result = fetch_channel_list(data, cancel, &error);

    if (result == NULL) {
        result = g_new0(ChannelListResult, 1);
        result->settings = data->settings;
        result->error = g_steal_pointer(&error);
    } else if (error != NULL) {
        result->error = g_steal_pointer(&error);
    }

    g_task_return_pointer(task, result, (GDestroyNotify)channel_list_result_free);
}

void twitch_channel_list_fetch_async(
    AppSettings *settings,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
)
{
    FetchChannelListData *data = g_new0(FetchChannelListData, 1);
    data->settings = settings;
    data->manual_channels = collect_settings_channels(settings, &data->manual_channel_count);
    data->oauth_token = g_strdup(app_settings_get_twitch_oauth_token(settings));
    data->refresh_token = g_strdup(app_settings_get_twitch_refresh_token(settings));
    data->oauth_expires_at = app_settings_get_twitch_oauth_expires_at(settings);

    GTask *task = g_task_new(NULL, cancel, callback, user_data);
    g_task_set_task_data(task, data, (GDestroyNotify)fetch_channel_list_data_free);
    g_task_run_in_thread(task, fetch_channel_list_worker);
    g_object_unref(task);
}

char **twitch_channel_list_fetch_finish(GAsyncResult *result, guint *channel_count_out, GError **error)
{
    g_return_val_if_fail(g_task_is_valid(result, NULL), NULL);

    ChannelListResult *list = g_task_propagate_pointer(G_TASK(result), error);
    if (list == NULL) {
        if (channel_count_out != NULL) {
            *channel_count_out = 0;
        }
        return NULL;
    }

    g_autoptr(GError) save_error = NULL;
    if (!save_refreshed_token(list, &save_error)) {
        if (error != NULL) {
            g_propagate_prefixed_error(error, g_steal_pointer(&save_error), "Twitch token refreshed, but saving failed: ");
        }
        channel_list_result_free(list);
        if (channel_count_out != NULL) {
            *channel_count_out = 0;
        }
        return NULL;
    }

    if (list->error != NULL) {
        if (error != NULL) {
            *error = g_steal_pointer(&list->error);
        }
        channel_list_result_free(list);
        if (channel_count_out != NULL) {
            *channel_count_out = 0;
        }
        return NULL;
    }

    char **channels = g_steal_pointer(&list->channels);
    if (channel_count_out != NULL) {
        *channel_count_out = list->channel_count;
    }
    channel_list_result_free(list);
    return channels;
}
