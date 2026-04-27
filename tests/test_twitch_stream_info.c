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

static void test_parse_stream_title_response_returns_title(void)
{
    const char *json = "{\"data\":{\"user\":{\"stream\":{\"title\":\"Live now\"}}}}";
    g_autoptr(GError) error = NULL;
    g_autofree char *title = parse_stream_title_response(json, strlen(json), &error);

    g_assert_no_error(error);
    g_assert_cmpstr(title, ==, "Live now");
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
    g_test_add_func("/twitch-stream-info/parse-response/title", test_parse_stream_title_response_returns_title);
    g_test_add_func("/twitch-stream-info/parse-response/offline", test_parse_stream_title_response_returns_null_for_offline_stream);
    g_test_add_func("/twitch-stream-info/parse-response/missing-user", test_parse_stream_title_response_returns_null_for_missing_user);
    g_test_add_func("/twitch-stream-info/parse-response/invalid-json", test_parse_stream_title_response_reports_invalid_json);

    return g_test_run();
}
