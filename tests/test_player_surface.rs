use std::ffi::{c_int, c_uint};
use twitch_player_core::player_surface;

fn should_restore(
    video_fullscreen_active: bool,
    app_fullscreen: bool,
    tile_focused: bool,
    focused_tile: c_uint,
    tile_index: c_uint,
) -> bool {
    player_surface::player_surface_test_fullscreen_should_restore(
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
    player_surface::player_surface_test_fullscreen_should_exit_app(
        app_fullscreen as c_int,
        video_fullscreen_active as c_int,
        restore_app_fullscreen as c_int,
    ) != 0
}

fn position_for(chat_width: c_int, window_width: c_int) -> c_int {
    player_surface::player_surface_test_chat_position_for_width(chat_width, window_width)
}

fn chat_width_for(window_width: c_int, position: c_int) -> c_int {
    player_surface::player_surface_test_chat_width_for_position(window_width, position)
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

    assert_eq!(1100 - position_for(220, 1100), 220);
    assert_eq!(1920 - position_for(460, 1920), 280);
    assert_eq!(chat_width_for(1100, 1100 - 460), 280);
    assert_eq!(512 - position_for(220, 512), 220);
    assert_eq!(400 - position_for(220, 400), 180);
    assert_eq!(300 - position_for(220, 300), 120);
}
