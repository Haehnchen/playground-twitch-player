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

static void test_parse_stream_title_response_returns_title(void)
{
    const char *json = "{\"data\":{\"user\":{\"stream\":{\"title\":\"Live now\"}}}}";
    g_autoptr(GError) error = NULL;
    g_autofree char *title = parse_stream_title_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_cmpstr(title, ==, "Live now");
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

static void test_parse_live_channels_response_reports_invalid_json(void)
{
    const char *json = "{";
    g_autoptr(GError) error = NULL;
    g_autoptr(GPtrArray) previews = parse_live_channels_response(json, strlen(json), &error);

    g_assert_null(previews);
    g_assert_nonnull(error);
    g_assert_cmpuint(error->domain, ==, JSON_PARSER_ERROR);
}

static void test_parse_stream_title_response_returns_null_for_offline_stream(void)
{
    const char *json = "{\"data\":{\"user\":{\"stream\":null}}}";
    g_autoptr(GError) error = NULL;
    g_autofree char *title = parse_stream_title_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_null(title);
}

static void test_parse_stream_title_response_returns_null_for_missing_user(void)
{
    const char *json = "{\"data\":{\"user\":null}}";
    g_autoptr(GError) error = NULL;
    g_autofree char *title = parse_stream_title_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_null(title);
}

static void test_parse_stream_title_response_reports_invalid_json(void)
{
    const char *json = "{";
    g_autoptr(GError) error = NULL;
    g_autofree char *title = parse_stream_title_response(json, strlen(json), &error);

    g_assert_null(title);
    g_assert_nonnull(error);
    g_assert_cmpuint(error->domain, ==, JSON_PARSER_ERROR);
}

int main(int argc, char **argv)
{
    g_test_init(&argc, &argv, NULL);

    g_test_add_func("/twitch-stream-info/build-request-body", test_build_stream_title_request_body);
    g_test_add_func("/twitch-stream-info/build-live-request-body", test_build_live_channels_request_body);
    g_test_add_func("/twitch-stream-info/build-live-request-body/skips-empty", test_build_live_channels_request_body_skips_empty_channels);
    g_test_add_func("/twitch-stream-info/parse-response/title", test_parse_stream_title_response_returns_title);
    g_test_add_func("/twitch-stream-info/parse-live-response/only-live", test_parse_live_channels_response_returns_only_live_streams);
    g_test_add_func("/twitch-stream-info/parse-live-response/missing-optional-fields", test_parse_live_channels_response_handles_missing_optional_fields);
    g_test_add_func("/twitch-stream-info/parse-live-response/tie-sort", test_parse_live_channels_response_sorts_equal_viewers_by_display_name);
    g_test_add_func("/twitch-stream-info/parse-live-response/missing-users", test_parse_live_channels_response_returns_empty_for_missing_users);
    g_test_add_func("/twitch-stream-info/parse-live-response/invalid-json", test_parse_live_channels_response_reports_invalid_json);
    g_test_add_func("/twitch-stream-info/parse-response/offline", test_parse_stream_title_response_returns_null_for_offline_stream);
    g_test_add_func("/twitch-stream-info/parse-response/missing-user", test_parse_stream_title_response_returns_null_for_missing_user);
    g_test_add_func("/twitch-stream-info/parse-response/invalid-json", test_parse_stream_title_response_reports_invalid_json);

    return g_test_run();
}
