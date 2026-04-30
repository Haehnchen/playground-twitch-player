#include <glib.h>
#include <glib/gstdio.h>

#include "../src/settings.h"

static void test_settings_round_trip_channels(void)
{
    g_autoptr(GError) error = NULL;
    g_autofree char *config_dir = g_dir_make_tmp("twitch-player-settings-XXXXXX", &error);
    g_assert_no_error(error);

    g_assert_true(g_setenv("XDG_CONFIG_HOME", config_dir, TRUE));

    AppSettings *settings = app_settings_new();
    g_assert_true(app_settings_get_hwdec_enabled(settings));
    app_settings_set_hwdec_enabled(settings, FALSE);
    app_settings_set_twitch_oauth_token(settings, "token-123");
    app_settings_add_channel(settings, "Papaplatte Live", "https://www.twitch.tv/PapaPlatte", NULL);

    g_assert_true(app_settings_save(settings, &error));
    g_assert_no_error(error);
    app_settings_free(settings);

    settings = app_settings_load();
    g_assert_false(app_settings_get_hwdec_enabled(settings));
    g_assert_cmpstr(app_settings_get_twitch_oauth_token(settings), ==, "token-123");
    g_assert_cmpuint(app_settings_get_channel_count(settings), ==, 1);

    const AppSettingsChannel *channel = app_settings_get_channel(settings, 0);
    g_assert_nonnull(channel);
    g_assert_cmpstr(channel->label, ==, "Papaplatte Live");
    g_assert_cmpstr(channel->channel, ==, "papaplatte");
    g_assert_cmpstr(channel->url, ==, "https://www.twitch.tv/papaplatte");
    app_settings_free(settings);

    g_autofree char *settings_path = app_settings_get_path();
    g_autofree char *app_dir = g_path_get_dirname(settings_path);
    g_assert_cmpint(g_remove(settings_path), ==, 0);
    g_assert_cmpint(g_rmdir(app_dir), ==, 0);
    g_assert_cmpint(g_rmdir(config_dir), ==, 0);
}

int main(int argc, char **argv)
{
    g_test_init(&argc, &argv, NULL);
    g_test_add_func("/app-settings/round-trip-channels", test_settings_round_trip_channels);
    return g_test_run();
}
