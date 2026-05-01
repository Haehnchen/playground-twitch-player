#include <glib.h>

#include "../src/grid_player.h"

static void test_restore_when_video_fullscreen_is_active(void)
{
    g_assert_true(grid_player_fullscreen_should_restore(TRUE, TRUE, TRUE, 1, 1));
    g_assert_true(grid_player_fullscreen_should_exit_app(TRUE, TRUE, FALSE));
}

static void test_restore_when_app_fullscreen_focus_state_remains(void)
{
    g_assert_true(grid_player_fullscreen_should_restore(FALSE, TRUE, TRUE, 2, 2));
    g_assert_true(grid_player_fullscreen_should_exit_app(TRUE, FALSE, FALSE));
}

static void test_keep_app_fullscreen_when_it_was_already_fullscreen(void)
{
    g_assert_true(grid_player_fullscreen_should_restore(TRUE, TRUE, TRUE, 0, 0));
    g_assert_false(grid_player_fullscreen_should_exit_app(TRUE, TRUE, TRUE));
}

static void test_do_not_restore_unfocused_tile(void)
{
    g_assert_false(grid_player_fullscreen_should_restore(FALSE, TRUE, TRUE, 0, 1));
    g_assert_false(grid_player_fullscreen_should_restore(FALSE, FALSE, TRUE, 0, 0));
}

int main(int argc, char **argv)
{
    g_test_init(&argc, &argv, NULL);

    g_test_add_func("/grid-player/fullscreen/restore-active", test_restore_when_video_fullscreen_is_active);
    g_test_add_func(
        "/grid-player/fullscreen/restore-from-app-fullscreen-focus-state",
        test_restore_when_app_fullscreen_focus_state_remains
    );
    g_test_add_func(
        "/grid-player/fullscreen/keep-existing-app-fullscreen",
        test_keep_app_fullscreen_when_it_was_already_fullscreen
    );
    g_test_add_func("/grid-player/fullscreen/ignore-unfocused-tile", test_do_not_restore_unfocused_tile);

    return g_test_run();
}
