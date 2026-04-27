#include "../src/chat_assets.c"

static void test_parse_emote_ranges_returns_sorted_ranges(void)
{
    g_autoptr(GArray) ranges = parse_emote_ranges("1902:12-16/25:0-4,6-10");

    g_assert_nonnull(ranges);
    g_assert_cmpuint(ranges->len, ==, 3);

    EmoteRange *first = &g_array_index(ranges, EmoteRange, 0);
    EmoteRange *second = &g_array_index(ranges, EmoteRange, 1);
    EmoteRange *third = &g_array_index(ranges, EmoteRange, 2);

    g_assert_cmpuint(first->start, ==, 0);
    g_assert_cmpuint(first->end, ==, 4);
    g_assert_cmpstr(first->id, ==, "25");
    g_assert_cmpuint(second->start, ==, 6);
    g_assert_cmpuint(second->end, ==, 10);
    g_assert_cmpstr(second->id, ==, "25");
    g_assert_cmpuint(third->start, ==, 12);
    g_assert_cmpuint(third->end, ==, 16);
    g_assert_cmpstr(third->id, ==, "1902");
}

static void test_parse_emote_ranges_ignores_invalid_specs(void)
{
    g_autoptr(GArray) ranges = parse_emote_ranges("bad/25:4-2,abc-5,1-x/33:2-3");

    g_assert_nonnull(ranges);
    g_assert_cmpuint(ranges->len, ==, 1);

    EmoteRange *range = &g_array_index(ranges, EmoteRange, 0);
    g_assert_cmpuint(range->start, ==, 2);
    g_assert_cmpuint(range->end, ==, 3);
    g_assert_cmpstr(range->id, ==, "33");
}

static void test_parse_emote_ranges_returns_null_for_empty_input(void)
{
    g_assert_null(parse_emote_ranges(NULL));
    g_assert_null(parse_emote_ranges(""));
    g_assert_null(parse_emote_ranges("bad/also-bad"));
}

static void test_utf8_offset_to_pointer_safe(void)
{
    const char *text = "a\xc3\xa4" "b";

    g_assert_true(utf8_offset_to_pointer_safe(text, 0) == text);
    g_assert_cmpstr(utf8_offset_to_pointer_safe(text, 1), ==, "\xc3\xa4" "b");
    g_assert_cmpstr(utf8_offset_to_pointer_safe(text, 2), ==, "b");
    g_assert_cmpstr(utf8_offset_to_pointer_safe(text, 3), ==, "");
    g_assert_null(utf8_offset_to_pointer_safe(text, 4));
}

int main(int argc, char **argv)
{
    g_test_init(&argc, &argv, NULL);

    g_test_add_func("/chat-assets/parse-emote-ranges/sorted", test_parse_emote_ranges_returns_sorted_ranges);
    g_test_add_func("/chat-assets/parse-emote-ranges/invalid-specs", test_parse_emote_ranges_ignores_invalid_specs);
    g_test_add_func("/chat-assets/parse-emote-ranges/empty", test_parse_emote_ranges_returns_null_for_empty_input);
    g_test_add_func("/chat-assets/utf8-offset-to-pointer-safe", test_utf8_offset_to_pointer_safe);

    return g_test_run();
}
