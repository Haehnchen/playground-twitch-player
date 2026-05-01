#include "../src/twitch_stream_info.c"

static void test_build_stream_title_request_body(void)
{
    g_autofree char *body = build_stream_title_request_body("papaplatte");
    g_autoptr(JsonParser) parser = json_parser_new();
    g_autoptr(GError) error = NULL;

    g_assert_true(json_parser_load_from_data(parser, body, -1, &error));
    g_assert_no_error(error);

    JsonObject *root = json_node_get_object(json_parser_get_root(parser));
    JsonObject *variables = json_object_get_object_member(root, "variables");

    g_assert_cmpstr(json_object_get_string_member(root, "query"), ==, TWITCH_GQL_QUERY);
    g_assert_cmpstr(json_object_get_string_member(variables, "login"), ==, "papaplatte");
}

static void test_build_live_channels_request_body(void)
{
    const char *channels[] = { "papaplatte", "rocketbeans" };
    g_autofree char *body = build_live_channels_request_body(channels, G_N_ELEMENTS(channels));
    g_autoptr(JsonParser) parser = json_parser_new();
    g_autoptr(GError) error = NULL;

    g_assert_true(json_parser_load_from_data(parser, body, -1, &error));
    g_assert_no_error(error);

    JsonObject *root = json_node_get_object(json_parser_get_root(parser));
    JsonObject *variables = json_object_get_object_member(root, "variables");
    JsonArray *logins = json_object_get_array_member(variables, "logins");

    g_assert_cmpstr(json_object_get_string_member(root, "query"), ==, TWITCH_GQL_LIVE_CHANNELS_QUERY);
    g_assert_cmpuint(json_array_get_length(logins), ==, 2);
    g_assert_cmpstr(json_array_get_string_element(logins, 0), ==, "papaplatte");
    g_assert_cmpstr(json_array_get_string_element(logins, 1), ==, "rocketbeans");
}

static void test_build_live_channels_request_body_skips_empty_channels(void)
{
    const char *channels[] = { "papaplatte", "", NULL, "rocketbeans" };
    g_autofree char *body = build_live_channels_request_body(channels, G_N_ELEMENTS(channels));
    g_autoptr(JsonParser) parser = json_parser_new();
    g_autoptr(GError) error = NULL;

    g_assert_true(json_parser_load_from_data(parser, body, -1, &error));
    g_assert_no_error(error);

    JsonObject *root = json_node_get_object(json_parser_get_root(parser));
    JsonObject *variables = json_object_get_object_member(root, "variables");
    JsonArray *logins = json_object_get_array_member(variables, "logins");

    g_assert_cmpuint(json_array_get_length(logins), ==, 2);
    g_assert_cmpstr(json_array_get_string_element(logins, 0), ==, "papaplatte");
    g_assert_cmpstr(json_array_get_string_element(logins, 1), ==, "rocketbeans");
}

static void test_parse_current_stream_response_returns_title_and_viewers(void)
{
    const char *json = "{\"data\":{\"user\":{\"stream\":{\"title\":\"Live now\",\"viewersCount\":1234}}}}";
    g_autoptr(GError) error = NULL;
    g_autoptr(TwitchCurrentStream) stream = parse_current_stream_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_nonnull(stream);
    g_assert_cmpstr(stream->title, ==, "Live now");
    g_assert_cmpuint(stream->viewer_count, ==, 1234);
}

static void test_parse_current_stream_response_handles_missing_optional_fields(void)
{
    const char *json = "{\"data\":{\"user\":{\"stream\":{\"title\":null,\"viewersCount\":null}}}}";
    g_autoptr(GError) error = NULL;
    g_autoptr(TwitchCurrentStream) stream = parse_current_stream_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_nonnull(stream);
    g_assert_cmpstr(stream->title, ==, "");
    g_assert_cmpuint(stream->viewer_count, ==, 0);
}

static void test_parse_stream_qualities_playlist_returns_sorted_variants(void)
{
    const char *playlist =
        "#EXTM3U\n"
        "#EXT-X-STREAM-INF:BANDWIDTH=2500000,RESOLUTION=1280x720,FRAME-RATE=60.000\n"
        "https://example.test/720p60.m3u8\n"
        "#EXT-X-STREAM-INF:BANDWIDTH=6000000,RESOLUTION=1920x1080,FRAME-RATE=60.000\n"
        "https://example.test/1080p60.m3u8\n"
        "#EXT-X-STREAM-INF:BANDWIDTH=900000,RESOLUTION=852x480,FRAME-RATE=30.000\n"
        "https://example.test/480p.m3u8\n"
        "#EXT-X-STREAM-INF:BANDWIDTH=160000,NAME=\"Audio Only\"\n"
        "https://example.test/audio.m3u8\n";
    g_autoptr(GError) error = NULL;
    g_autoptr(GPtrArray) qualities = parse_stream_qualities_playlist(playlist, &error);

    g_assert_no_error(error);
    g_assert_nonnull(qualities);
    g_assert_cmpuint(qualities->len, ==, 3);

    TwitchStreamQuality *low = g_ptr_array_index(qualities, 0);
    TwitchStreamQuality *mid = g_ptr_array_index(qualities, 1);
    TwitchStreamQuality *source = g_ptr_array_index(qualities, 2);

    g_assert_cmpstr(low->label, ==, "480p");
    g_assert_cmpstr(mid->label, ==, "720p60");
    g_assert_cmpstr(source->label, ==, "1080p60");
    g_assert_cmpstr(source->url, ==, "https://example.test/1080p60.m3u8");
    g_assert_cmpuint(source->width, ==, 1920);
    g_assert_cmpuint(source->height, ==, 1080);
}

static void test_format_viewer_count_compacts_large_counts(void)
{
    g_autofree char *small = twitch_stream_info_format_viewer_count(999);
    g_autofree char *thousands = twitch_stream_info_format_viewer_count(1234);
    g_autofree char *millions = twitch_stream_info_format_viewer_count(1234567);

    g_assert_cmpstr(small, ==, "999");
    g_assert_cmpstr(thousands, ==, "1.2K");
    g_assert_cmpstr(millions, ==, "1.2M");
}

static void test_format_current_stream_title_joins_viewers_and_title(void)
{
    TwitchCurrentStream stream = {
        .title = "Live now",
        .viewer_count = 1234,
    };
    TwitchCurrentStream untitled_stream = {
        .title = "",
        .viewer_count = 42,
    };

    g_autofree char *title = twitch_stream_info_format_current_stream_title(&stream);
    g_autofree char *untitled = twitch_stream_info_format_current_stream_title(&untitled_stream);

    g_assert_cmpstr(title, ==, "1.2K • Live now");
    g_assert_cmpstr(untitled, ==, "42");
}

static void test_parse_live_channels_response_returns_only_live_streams(void)
{
    const char *json =
        "{"
        "\"data\":{\"users\":["
        "{\"login\":\"LiveOne\",\"displayName\":\"Live One\",\"profileImageURL\":\"https://avatar\","
        "\"stream\":{\"title\":\"Now live\",\"viewersCount\":1234,\"createdAt\":\"2026-04-30T10:00:00Z\","
        "\"game\":{\"name\":\"Just Chatting\"},"
        "\"previewImageURL\":\"https://preview\"}},"
        "{\"login\":\"BiggerLive\",\"displayName\":\"Bigger Live\",\"profileImageURL\":\"https://avatar-big\","
        "\"stream\":{\"title\":\"More live\",\"viewersCount\":9999,\"createdAt\":\"2026-04-30T09:00:00Z\","
        "\"game\":{\"name\":\"Games + Demos\"},"
        "\"previewImageURL\":\"https://preview-big\"}},"
        "{\"login\":\"OfflineOne\",\"displayName\":\"Offline One\",\"profileImageURL\":\"https://offline\",\"stream\":null}"
        "]}"
        "}";
    g_autoptr(GError) error = NULL;
    g_autoptr(GPtrArray) previews = parse_live_channels_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_nonnull(previews);
    g_assert_cmpuint(previews->len, ==, 2);

    TwitchStreamPreview *preview = g_ptr_array_index(previews, 0);
    g_assert_cmpstr(preview->channel, ==, "biggerlive");
    g_assert_cmpstr(preview->display_name, ==, "Bigger Live");
    g_assert_cmpstr(preview->title, ==, "More live");
    g_assert_cmpstr(preview->avatar_url, ==, "https://avatar-big");
    g_assert_cmpstr(preview->preview_url, ==, "https://preview-big");
    g_assert_cmpstr(preview->started_at, ==, "2026-04-30T09:00:00Z");
    g_assert_cmpstr(preview->category_name, ==, "Games + Demos");
    g_assert_cmpuint(preview->viewer_count, ==, 9999);

    preview = g_ptr_array_index(previews, 1);
    g_assert_cmpstr(preview->channel, ==, "liveone");
    g_assert_cmpuint(preview->viewer_count, ==, 1234);
}

static void test_parse_live_channels_response_handles_missing_optional_fields(void)
{
    const char *json =
        "{"
        "\"data\":{\"users\":["
        "{\"login\":\"FallbackLogin\",\"displayName\":null,\"profileImageURL\":null,"
        "\"stream\":{\"title\":null,\"viewersCount\":null,\"createdAt\":null,\"game\":null,"
        "\"previewImageURL\":null}}"
        "]}"
        "}";
    g_autoptr(GError) error = NULL;
    g_autoptr(GPtrArray) previews = parse_live_channels_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_nonnull(previews);
    g_assert_cmpuint(previews->len, ==, 1);

    TwitchStreamPreview *preview = g_ptr_array_index(previews, 0);
    g_assert_cmpstr(preview->channel, ==, "fallbacklogin");
    g_assert_cmpstr(preview->display_name, ==, "FallbackLogin");
    g_assert_cmpstr(preview->title, ==, "");
    g_assert_null(preview->avatar_url);
    g_assert_null(preview->preview_url);
    g_assert_null(preview->started_at);
    g_assert_null(preview->category_name);
    g_assert_cmpuint(preview->viewer_count, ==, 0);
}

static void test_parse_live_channels_response_sorts_equal_viewers_by_display_name(void)
{
    const char *json =
        "{"
        "\"data\":{\"users\":["
        "{\"login\":\"zeta\",\"displayName\":\"Zeta\",\"stream\":{\"title\":\"Z\",\"viewersCount\":10}},"
        "{\"login\":\"alpha\",\"displayName\":\"Alpha\",\"stream\":{\"title\":\"A\",\"viewersCount\":10}}"
        "]}"
        "}";
    g_autoptr(GError) error = NULL;
    g_autoptr(GPtrArray) previews = parse_live_channels_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_nonnull(previews);
    g_assert_cmpuint(previews->len, ==, 2);

    TwitchStreamPreview *preview = g_ptr_array_index(previews, 0);
    g_assert_cmpstr(preview->display_name, ==, "Alpha");
    preview = g_ptr_array_index(previews, 1);
    g_assert_cmpstr(preview->display_name, ==, "Zeta");
}

static void test_parse_live_channels_response_returns_empty_for_missing_users(void)
{
    const char *json = "{\"data\":{}}";
    g_autoptr(GError) error = NULL;
    g_autoptr(GPtrArray) previews = parse_live_channels_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_nonnull(previews);
    g_assert_cmpuint(previews->len, ==, 0);
}

static void test_parse_helix_user_id_response_returns_user_id(void)
{
    const char *json = "{\"data\":[{\"id\":\"12345\",\"login\":\"viewer\"}]}";
    g_autoptr(GError) error = NULL;
    g_autofree char *user_id = parse_helix_user_id_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_cmpstr(user_id, ==, "12345");
}

static void test_parse_followed_channels_page_returns_channels_and_cursor(void)
{
    const char *json =
        "{"
        "\"data\":["
        "{\"broadcaster_login\":\"PapaPlatte\",\"broadcaster_name\":\"Papaplatte\"},"
        "{\"broadcaster_login\":\"rocketbeans\",\"broadcaster_name\":\"Rocket Beans TV\"}"
        "],"
        "\"pagination\":{\"cursor\":\"next-page\"}"
        "}";
    g_autoptr(GError) error = NULL;
    g_autoptr(GPtrArray) channels = g_ptr_array_new_with_free_func((GDestroyNotify)twitch_followed_channel_free);
    g_autofree char *cursor = NULL;

    g_assert_true(parse_followed_channels_page(json, strlen(json), channels, &cursor, &error));
    g_assert_no_error(error);
    g_assert_cmpuint(channels->len, ==, 2);
    g_assert_cmpstr(cursor, ==, "next-page");

    TwitchFollowedChannel *channel = g_ptr_array_index(channels, 0);
    g_assert_cmpstr(channel->channel, ==, "papaplatte");
    g_assert_cmpstr(channel->display_name, ==, "Papaplatte");

    channel = g_ptr_array_index(channels, 1);
    g_assert_cmpstr(channel->channel, ==, "rocketbeans");
    g_assert_cmpstr(channel->display_name, ==, "Rocket Beans TV");
}

static void test_parse_live_channels_response_reports_invalid_json(void)
{
    const char *json = "{";
    g_autoptr(GError) error = NULL;
    g_autoptr(GPtrArray) previews = parse_live_channels_response(json, strlen(json), &error);

    g_assert_null(previews);
    g_assert_nonnull(error);
    g_assert_cmpuint(error->domain, ==, JSON_PARSER_ERROR);
}

static void test_parse_current_stream_response_returns_null_for_offline_stream(void)
{
    const char *json = "{\"data\":{\"user\":{\"stream\":null}}}";
    g_autoptr(GError) error = NULL;
    g_autoptr(TwitchCurrentStream) stream = parse_current_stream_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_null(stream);
}

static void test_parse_current_stream_response_returns_null_for_missing_user(void)
{
    const char *json = "{\"data\":{\"user\":null}}";
    g_autoptr(GError) error = NULL;
    g_autoptr(TwitchCurrentStream) stream = parse_current_stream_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_null(stream);
}

static void test_parse_current_stream_response_reports_invalid_json(void)
{
    const char *json = "{";
    g_autoptr(GError) error = NULL;
    g_autoptr(TwitchCurrentStream) stream = parse_current_stream_response(json, strlen(json), &error);

    g_assert_null(stream);
    g_assert_nonnull(error);
    g_assert_cmpuint(error->domain, ==, JSON_PARSER_ERROR);
}

int main(int argc, char **argv)
{
    g_test_init(&argc, &argv, NULL);

    g_test_add_func("/twitch-stream-info/build-request-body", test_build_stream_title_request_body);
    g_test_add_func("/twitch-stream-info/build-live-request-body", test_build_live_channels_request_body);
    g_test_add_func("/twitch-stream-info/build-live-request-body/skips-empty", test_build_live_channels_request_body_skips_empty_channels);
    g_test_add_func("/twitch-stream-info/parse-response/current-stream", test_parse_current_stream_response_returns_title_and_viewers);
    g_test_add_func("/twitch-stream-info/parse-response/current-stream-missing-optional", test_parse_current_stream_response_handles_missing_optional_fields);
    g_test_add_func("/twitch-stream-info/parse-response/stream-qualities", test_parse_stream_qualities_playlist_returns_sorted_variants);
    g_test_add_func("/twitch-stream-info/format/viewer-count", test_format_viewer_count_compacts_large_counts);
    g_test_add_func("/twitch-stream-info/format/current-stream-title", test_format_current_stream_title_joins_viewers_and_title);
    g_test_add_func("/twitch-stream-info/parse-live-response/only-live", test_parse_live_channels_response_returns_only_live_streams);
    g_test_add_func("/twitch-stream-info/parse-live-response/missing-optional-fields", test_parse_live_channels_response_handles_missing_optional_fields);
    g_test_add_func("/twitch-stream-info/parse-live-response/tie-sort", test_parse_live_channels_response_sorts_equal_viewers_by_display_name);
    g_test_add_func("/twitch-stream-info/parse-live-response/missing-users", test_parse_live_channels_response_returns_empty_for_missing_users);
    g_test_add_func("/twitch-stream-info/parse-helix-user/id", test_parse_helix_user_id_response_returns_user_id);
    g_test_add_func("/twitch-stream-info/parse-followed-page/channels", test_parse_followed_channels_page_returns_channels_and_cursor);
    g_test_add_func("/twitch-stream-info/parse-live-response/invalid-json", test_parse_live_channels_response_reports_invalid_json);
    g_test_add_func("/twitch-stream-info/parse-response/offline", test_parse_current_stream_response_returns_null_for_offline_stream);
    g_test_add_func("/twitch-stream-info/parse-response/missing-user", test_parse_current_stream_response_returns_null_for_missing_user);
    g_test_add_func("/twitch-stream-info/parse-response/invalid-json", test_parse_current_stream_response_reports_invalid_json);

    return g_test_run();
}
