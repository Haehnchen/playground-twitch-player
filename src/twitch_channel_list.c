#define G_LOG_DOMAIN "twitch-channel-list"

#include "twitch_channel_list.h"

#include "twitch_auth.h"
#include "twitch_stream_info.h"

#define FOLLOWED_CHANNELS_CACHE_SECONDS 120

typedef struct {
    char **manual_channels;
    guint manual_channel_count;
    char *oauth_token;
} FetchChannelListData;

typedef struct {
    char **channels;
    guint channel_count;
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
    g_free(data);
}

static void channel_list_result_free(ChannelListResult *result)
{
    if (result == NULL) {
        return;
    }

    g_strfreev(result->channels);
    g_free(result);
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

static ChannelListResult *fetch_channel_list(FetchChannelListData *data, GCancellable *cancel, GError **error)
{
    if (data->oauth_token == NULL || data->oauth_token[0] == '\0') {
        return build_channel_list_result(data->manual_channels, data->manual_channel_count, NULL);
    }

    g_autoptr(GPtrArray) followed_channels = dup_fresh_followed_cache();
    if (followed_channels != NULL) {
        return build_channel_list_result(data->manual_channels, data->manual_channel_count, followed_channels);
    }

    const char *client_id = TWITCH_AUTH_CLIENT_ID;
    const char *oauth_token = data->oauth_token;
    if (client_id == NULL || client_id[0] == '\0' || oauth_token == NULL || oauth_token[0] == '\0') {
        if (data->manual_channel_count > 0) {
            g_debug("followed channels enabled but Twitch credentials are incomplete");
            return build_channel_list_result(data->manual_channels, data->manual_channel_count, NULL);
        }

        g_set_error(
            error,
            G_IO_ERROR,
            G_IO_ERROR_FAILED,
            "Connect Twitch in Settings to load followed channels"
        );
        return NULL;
    }

    followed_channels = twitch_stream_info_fetch_followed_channels(client_id, oauth_token, cancel, error);
    if (followed_channels == NULL) {
        if (data->manual_channel_count > 0 &&
            (error == NULL || *error == NULL || !g_error_matches(*error, G_IO_ERROR, G_IO_ERROR_CANCELLED))) {
            if (error != NULL && *error != NULL) {
                g_debug("followed channel fetch failed: %s", (*error)->message);
                g_clear_error(error);
            }
            return build_channel_list_result(data->manual_channels, data->manual_channel_count, NULL);
        }
        return NULL;
    }

    store_followed_cache(followed_channels);
    return build_channel_list_result(data->manual_channels, data->manual_channel_count, followed_channels);
}

static void fetch_channel_list_worker(GTask *task, gpointer source_object, gpointer task_data, GCancellable *cancel)
{
    (void)source_object;
    FetchChannelListData *data = task_data;
    g_autoptr(GError) error = NULL;
    ChannelListResult *result = fetch_channel_list(data, cancel, &error);

    if (error != NULL) {
        g_task_return_error(task, g_steal_pointer(&error));
        return;
    }

    g_task_return_pointer(task, result, (GDestroyNotify)channel_list_result_free);
}

void twitch_channel_list_fetch_async(
    const AppSettings *settings,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
)
{
    FetchChannelListData *data = g_new0(FetchChannelListData, 1);
    data->manual_channels = collect_settings_channels(settings, &data->manual_channel_count);
    data->oauth_token = g_strdup(app_settings_get_twitch_oauth_token(settings));

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

    char **channels = g_steal_pointer(&list->channels);
    if (channel_count_out != NULL) {
        *channel_count_out = list->channel_count;
    }
    channel_list_result_free(list);
    return channels;
}
