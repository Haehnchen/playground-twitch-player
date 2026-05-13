use std::ffi::{c_int, c_uint};
use twitch_player_core::grid_player;

fn should_restore(
    video_fullscreen_active: bool,
    app_fullscreen: bool,
    tile_focused: bool,
    focused_tile: c_uint,
    tile_index: c_uint,
) -> bool {
    grid_player::grid_player_test_fullscreen_should_restore(
        video_fullscreen_active as c_int,
        app_fullscreen as c_int,
        tile_focused as c_int,
        focused_tile,
        tile_index,
    ) != 0
}

fn should_exit_app(
    app_fullscreen: bool,
    video_fullscreen_active: bool,
    restore_app_fullscreen: bool,
) -> bool {
    grid_player::grid_player_test_fullscreen_should_exit_app(
        app_fullscreen as c_int,
        video_fullscreen_active as c_int,
        restore_app_fullscreen as c_int,
    ) != 0
}

fn main() {
    assert!(should_restore(true, true, true, 1, 1));
    assert!(should_exit_app(true, true, false));

    assert!(should_restore(false, true, true, 2, 2));
    assert!(should_exit_app(true, false, false));

    assert!(should_restore(true, true, true, 0, 0));
    assert!(!should_exit_app(true, true, true));

    assert!(!should_restore(false, false, false, 0, 1));
    assert!(!should_exit_app(false, false, false));

    assert!(!should_restore(false, true, true, 0, 1));
    assert!(!should_restore(false, false, true, 0, 0));
}
