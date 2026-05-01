#define G_LOG_DOMAIN "twitch-player-settings"

#include "settings.h"

#include <errno.h>
#include <json-glib/json-glib.h>
#include <string.h>

#define SETTINGS_DIR_NAME "twitch-player"
#define SETTINGS_FILE_NAME "settings.json"

struct _AppSettings {
    GPtrArray *channels;
    char *twitch_oauth_token;
    char *twitch_refresh_token;
    gint64 twitch_oauth_expires_at;
    gboolean hwdec_enabled;
};

static void app_settings_channel_free(gpointer data)
{
    AppSettingsChannel *channel = data;

    if (channel == NULL) {
        return;
    }

    g_free(channel->label);
    g_free(channel->channel);
    g_free(channel->url);
    g_free(channel);
}

static char *trim_string(const char *value)
{
    if (value == NULL) {
        return g_strdup("");
    }

    g_autofree char *copy = g_strdup(value);
    return g_strdup(g_strstrip(copy));
}

static char *extract_twitch_channel_name(const char *value)
{
    if (value == NULL || value[0] == '\0') {
        return NULL;
    }

    /* Accept Twitch URLs as a convenience for CLI/startup imports. */
    const char *start = strstr(value, "twitch.tv/");
    if (start != NULL) {
        start += strlen("twitch.tv/");
    } else {
        start = value;
    }

    while (*start == '/' || *start == '@') {
        start++;
    }

    const char *end = start;
    while (g_ascii_isalnum(*end) || *end == '_') {
        end++;
    }

    if (end == start) {
        return NULL;
    }

    g_autofree char *channel = g_strndup(start, end - start);
    return g_ascii_strdown(channel, -1);
}

AppSettings *app_settings_new(void)
{
    AppSettings *settings = g_new0(AppSettings, 1);
    settings->channels = g_ptr_array_new_with_free_func(app_settings_channel_free);
    settings->hwdec_enabled = TRUE;
    return settings;
}

char *app_settings_get_path(void)
{
    /* GLib maps this to $XDG_CONFIG_HOME or ~/.config on Linux. */
    return g_build_filename(g_get_user_config_dir(), SETTINGS_DIR_NAME, SETTINGS_FILE_NAME, NULL);
}

void app_settings_free(AppSettings *settings)
{
    if (settings == NULL) {
        return;
    }

    g_ptr_array_unref(settings->channels);
    g_free(settings->twitch_oauth_token);
    g_free(settings->twitch_refresh_token);
    g_free(settings);
}

guint app_settings_get_channel_count(const AppSettings *settings)
{
    return settings != NULL ? settings->channels->len : 0;
}

const AppSettingsChannel *app_settings_get_channel(const AppSettings *settings, guint index)
{
    if (settings == NULL || index >= settings->channels->len) {
        return NULL;
    }

    return g_ptr_array_index(settings->channels, index);
}

gboolean app_settings_get_hwdec_enabled(const AppSettings *settings)
{
    return settings != NULL ? settings->hwdec_enabled : TRUE;
}

void app_settings_set_hwdec_enabled(AppSettings *settings, gboolean enabled)
{
    if (settings != NULL) {
        settings->hwdec_enabled = enabled;
    }
}

const char *app_settings_get_twitch_oauth_token(const AppSettings *settings)
{
    return settings != NULL ? settings->twitch_oauth_token : NULL;
}

const char *app_settings_get_twitch_refresh_token(const AppSettings *settings)
{
    return settings != NULL ? settings->twitch_refresh_token : NULL;
}

gint64 app_settings_get_twitch_oauth_expires_at(const AppSettings *settings)
{
    return settings != NULL ? settings->twitch_oauth_expires_at : 0;
}

void app_settings_set_twitch_oauth_token(AppSettings *settings, const char *oauth_token)
{
    if (settings == NULL) {
        return;
    }

    g_autofree char *new_oauth_token = oauth_token != NULL && oauth_token[0] != '\0'
        ? g_strdup(oauth_token)
        : NULL;

    g_free(settings->twitch_oauth_token);
    settings->twitch_oauth_token = g_steal_pointer(&new_oauth_token);
    if (settings->twitch_oauth_token == NULL) {
        g_clear_pointer(&settings->twitch_refresh_token, g_free);
        settings->twitch_oauth_expires_at = 0;
    }
}

void app_settings_set_twitch_auth_tokens(
    AppSettings *settings,
    const char *oauth_token,
    const char *refresh_token,
    gint64 oauth_expires_at
)
{
    if (settings == NULL) {
        return;
    }

    g_autofree char *new_oauth_token = oauth_token != NULL && oauth_token[0] != '\0'
        ? g_strdup(oauth_token)
        : NULL;
    g_autofree char *new_refresh_token = refresh_token != NULL && refresh_token[0] != '\0'
        ? g_strdup(refresh_token)
        : NULL;

    g_free(settings->twitch_oauth_token);
    settings->twitch_oauth_token = g_steal_pointer(&new_oauth_token);

    g_free(settings->twitch_refresh_token);
    settings->twitch_refresh_token = g_steal_pointer(&new_refresh_token);

    settings->twitch_oauth_expires_at = settings->twitch_oauth_token != NULL ? oauth_expires_at : 0;
}

void app_settings_clear_channels(AppSettings *settings)
{
    if (settings == NULL) {
        return;
    }

    g_ptr_array_set_size(settings->channels, 0);
}

void app_settings_add_channel(AppSettings *settings, const char *label, const char *channel, const char *url)
{
    if (settings == NULL) {
        return;
    }

    g_autofree char *trimmed_label = trim_string(label);
    g_autofree char *trimmed_channel = trim_string(channel);
    g_autofree char *trimmed_url = trim_string(url);
    g_autofree char *derived_channel = NULL;

    /* Normalize all stored channels to the Twitch login used by IRC/mpv paths. */
    if (trimmed_channel[0] != '\0') {
        derived_channel = extract_twitch_channel_name(trimmed_channel);
    }
    if (derived_channel == NULL && trimmed_url[0] != '\0') {
        derived_channel = extract_twitch_channel_name(trimmed_url);
    }

    if (derived_channel == NULL && trimmed_url[0] == '\0' && trimmed_channel[0] == '\0') {
        return;
    }

    AppSettingsChannel *entry = g_new0(AppSettingsChannel, 1);
    entry->channel = derived_channel != NULL ? g_steal_pointer(&derived_channel) : g_strdup(trimmed_channel);

    if (trimmed_url[0] != '\0') {
        entry->url = g_strdup(trimmed_url);
    } else if (entry->channel != NULL && entry->channel[0] != '\0') {
        entry->url = g_strdup_printf("https://www.twitch.tv/%s", entry->channel);
    } else {
        entry->url = g_strdup("");
    }

    if (trimmed_label[0] != '\0') {
        entry->label = g_strdup(trimmed_label);
    } else if (entry->channel != NULL && entry->channel[0] != '\0') {
        entry->label = g_strdup(entry->channel);
    } else {
        entry->label = g_strdup(entry->url);
    }

    g_ptr_array_add(settings->channels, entry);
}

static void load_channels(AppSettings *settings, JsonObject *root)
{
    JsonNode *channels_node = json_object_get_member(root, "channels");

    if (channels_node == NULL || !JSON_NODE_HOLDS_ARRAY(channels_node)) {
        return;
    }

    JsonArray *channels = json_node_get_array(channels_node);
    guint length = json_array_get_length(channels);

    for (guint i = 0; i < length; i++) {
        JsonNode *node = json_array_get_element(channels, i);
        if (node == NULL || !JSON_NODE_HOLDS_OBJECT(node)) {
            continue;
        }

        JsonObject *item = json_node_get_object(node);
        const char *label = json_object_get_string_member_with_default(item, "label", "");
        const char *channel = json_object_get_string_member_with_default(item, "channel", "");
        const char *url = json_object_get_string_member_with_default(item, "url", "");
        app_settings_add_channel(settings, label, channel, url);
    }
}

AppSettings *app_settings_load(void)
{
    AppSettings *settings = app_settings_new();
    g_autofree char *path = app_settings_get_path();

    if (!g_file_test(path, G_FILE_TEST_EXISTS)) {
        return settings;
    }

    g_autoptr(GError) error = NULL;
    g_autoptr(JsonParser) parser = json_parser_new();
    if (!json_parser_load_from_file(parser, path, &error)) {
        g_warning("could not load settings from %s: %s", path, error->message);
        return settings;
    }

    JsonNode *root_node = json_parser_get_root(parser);
    if (root_node == NULL || !JSON_NODE_HOLDS_OBJECT(root_node)) {
        g_warning("settings file %s does not contain a JSON object", path);
        return settings;
    }

    JsonObject *root = json_node_get_object(root_node);
    settings->hwdec_enabled = json_object_get_boolean_member_with_default(root, "hwdec", TRUE);
    app_settings_set_twitch_oauth_token(
        settings,
        json_object_get_string_member_with_default(root, "twitch_oauth_token", NULL)
    );
    app_settings_set_twitch_auth_tokens(
        settings,
        app_settings_get_twitch_oauth_token(settings),
        json_object_get_string_member_with_default(root, "twitch_refresh_token", NULL),
        json_object_get_int_member_with_default(root, "twitch_oauth_expires_at", 0)
    );
    load_channels(settings, root);
    return settings;
}

gboolean app_settings_save(AppSettings *settings, GError **error)
{
    g_autofree char *config_dir = g_build_filename(g_get_user_config_dir(), SETTINGS_DIR_NAME, NULL);
    g_autofree char *path = app_settings_get_path();

    if (g_mkdir_with_parents(config_dir, 0700) < 0) {
        g_set_error(
            error,
            G_FILE_ERROR,
            g_file_error_from_errno(errno),
            "Could not create %s: %s",
            config_dir,
            g_strerror(errno)
        );
        return FALSE;
    }

    /* Use JsonBuilder so user-provided channel names are escaped correctly. */
    g_autoptr(JsonBuilder) builder = json_builder_new();
    json_builder_begin_object(builder);
    json_builder_set_member_name(builder, "hwdec");
    json_builder_add_boolean_value(builder, app_settings_get_hwdec_enabled(settings));
    const char *oauth_token = app_settings_get_twitch_oauth_token(settings);
    if (oauth_token != NULL && oauth_token[0] != '\0') {
        json_builder_set_member_name(builder, "twitch_oauth_token");
        json_builder_add_string_value(builder, oauth_token);
    }
    const char *refresh_token = app_settings_get_twitch_refresh_token(settings);
    if (refresh_token != NULL && refresh_token[0] != '\0') {
        json_builder_set_member_name(builder, "twitch_refresh_token");
        json_builder_add_string_value(builder, refresh_token);
    }
    gint64 oauth_expires_at = app_settings_get_twitch_oauth_expires_at(settings);
    if (oauth_expires_at > 0) {
        json_builder_set_member_name(builder, "twitch_oauth_expires_at");
        json_builder_add_int_value(builder, oauth_expires_at);
    }
    json_builder_set_member_name(builder, "channels");
    json_builder_begin_array(builder);

    for (guint i = 0; i < app_settings_get_channel_count(settings); i++) {
        const AppSettingsChannel *channel = app_settings_get_channel(settings, i);

        json_builder_begin_object(builder);
        json_builder_set_member_name(builder, "label");
        json_builder_add_string_value(builder, channel->label);
        json_builder_set_member_name(builder, "channel");
        json_builder_add_string_value(builder, channel->channel);
        json_builder_set_member_name(builder, "url");
        json_builder_add_string_value(builder, channel->url);
        json_builder_end_object(builder);
    }

    json_builder_end_array(builder);
    json_builder_end_object(builder);

    g_autoptr(JsonNode) root = json_builder_get_root(builder);
    g_autoptr(JsonGenerator) generator = json_generator_new();
    json_generator_set_root(generator, root);
    json_generator_set_pretty(generator, TRUE);

    return json_generator_to_file(generator, path, error);
}
