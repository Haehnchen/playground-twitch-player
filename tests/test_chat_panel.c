#include "../src/chat_panel.c"

static void test_fallback_username_color_is_deterministic(void)
{
    const char *first = fallback_username_color("alice");
    const char *second = fallback_username_color("alice");
    const char *empty = fallback_username_color(NULL);

    g_assert_cmpstr(first, ==, second);
    g_assert_true(g_str_has_prefix(first, "#"));
    g_assert_cmpuint(strlen(first), ==, 7);
    g_assert_true(g_str_has_prefix(empty, "#"));
    g_assert_cmpuint(strlen(empty), ==, 7);
}

static void test_adjustment_is_at_bottom(void)
{
    g_autoptr(GtkAdjustment) adjustment = gtk_adjustment_new(90.0, 0.0, 100.0, 1.0, 10.0, 10.0);

    g_assert_true(adjustment_is_at_bottom(NULL));
    g_assert_true(adjustment_is_at_bottom(adjustment));

    gtk_adjustment_set_value(adjustment, 87.0);
    g_assert_false(adjustment_is_at_bottom(adjustment));

    gtk_adjustment_set_value(adjustment, 88.0);
    g_assert_true(adjustment_is_at_bottom(adjustment));
}

int main(int argc, char **argv)
{
    g_test_init(&argc, &argv, NULL);

    g_test_add_func("/chat-panel/fallback-username-color", test_fallback_username_color_is_deterministic);
    g_test_add_func("/chat-panel/adjustment-is-at-bottom", test_adjustment_is_at_bottom);

    return g_test_run();
}
