#pragma once

#include <glib.h>

typedef struct {
    char *label;
    char *channel;
    char *url;
} AppSettingsChannel;

typedef struct _AppSettings AppSettings;

/**
 * app_settings_new:
 *
 * Creates an empty in-memory settings object.
 *
 * Returns: A newly allocated settings object. Free with app_settings_free().
 */
AppSettings *app_settings_new(void);

/**
 * app_settings_load:
 *
 * Loads settings from the user's config directory. Missing or invalid files
 * return an empty settings object so startup can continue.
 *
 * Returns: A newly allocated settings object. Free with app_settings_free().
 */
AppSettings *app_settings_load(void);

/**
 * app_settings_free:
 * @settings: Settings object, or NULL.
 *
 * Releases a settings object and all owned channel data.
 */
void app_settings_free(AppSettings *settings);

/**
 * app_settings_get_path:
 *
 * Builds the full settings JSON path. GLib resolves the base directory via
 * g_get_user_config_dir(), usually $XDG_CONFIG_HOME or ~/.config.
 *
 * Returns: Newly allocated path string. Free with g_free().
 */
char *app_settings_get_path(void);

/**
 * app_settings_save:
 * @settings: Settings object to persist.
 * @error: Return location for a GError, or NULL.
 *
 * Writes settings as pretty JSON into the user's config directory, creating the
 * app config directory when needed.
 *
 * Returns: TRUE on success, FALSE with @error set on failure.
 */
gboolean app_settings_save(AppSettings *settings, GError **error);

/**
 * app_settings_get_channel_count:
 * @settings: Settings object, or NULL.
 *
 * Returns: Number of configured Twitch channels.
 */
guint app_settings_get_channel_count(const AppSettings *settings);

/**
 * app_settings_get_channel:
 * @settings: Settings object.
 * @index: Zero-based channel index.
 *
 * Returns: Borrowed channel entry, or NULL when @index is out of range.
 */
const AppSettingsChannel *app_settings_get_channel(const AppSettings *settings, guint index);

gboolean app_settings_get_hwdec_enabled(const AppSettings *settings);
void app_settings_set_hwdec_enabled(AppSettings *settings, gboolean enabled);
const char *app_settings_get_twitch_oauth_token(const AppSettings *settings);
const char *app_settings_get_twitch_refresh_token(const AppSettings *settings);
gint64 app_settings_get_twitch_oauth_expires_at(const AppSettings *settings);
void app_settings_set_twitch_oauth_token(AppSettings *settings, const char *oauth_token);
void app_settings_set_twitch_auth_tokens(
    AppSettings *settings,
    const char *oauth_token,
    const char *refresh_token,
    gint64 oauth_expires_at
);

/**
 * app_settings_clear_channels:
 * @settings: Settings object, or NULL.
 *
 * Removes all configured channels from the in-memory settings object.
 */
void app_settings_clear_channels(AppSettings *settings);

/**
 * app_settings_add_channel:
 * @settings: Settings object.
 * @label: Optional display label.
 * @channel: Optional Twitch login or Twitch URL.
 * @url: Optional stream URL.
 *
 * Adds a channel entry, deriving missing label or URL values from the Twitch
 * channel name where possible. Empty entries are ignored.
 */
void app_settings_add_channel(AppSettings *settings, const char *label, const char *channel, const char *url);
