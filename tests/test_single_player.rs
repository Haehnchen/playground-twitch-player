use std::ffi::c_int;
use twitch_player_core::single_player;

fn position_for(chat_width: c_int, window_width: c_int) -> c_int {
    single_player::single_player_test_chat_position_for_width(chat_width, window_width)
}

fn chat_width_for(window_width: c_int, position: c_int) -> c_int {
    single_player::single_player_test_chat_width_for_position(window_width, position)
}

fn main() {
    let default_chat_width = 280;
    let wide_chat_width = 460;

    let normal_position = position_for(default_chat_width, 1100);
    let fullscreen_position = position_for(default_chat_width, 1920);
    assert_eq!(1100 - normal_position, default_chat_width);
    assert_eq!(1920 - fullscreen_position, default_chat_width);

    let normal_position = position_for(wide_chat_width, 1100);
    let fullscreen_position = position_for(wide_chat_width, 1920);
    assert_eq!(1100 - normal_position, wide_chat_width);
    assert_eq!(1920 - fullscreen_position, wide_chat_width);

    assert_eq!(
        chat_width_for(1100, 1100 - wide_chat_width),
        wide_chat_width
    );
    assert_eq!(
        chat_width_for(1920, 1920 - wide_chat_width),
        wide_chat_width
    );

    // Very small windows clamp to minimum chat width instead of letting video disappear.
    let cramped_position = position_for(default_chat_width, 400);
    assert_eq!(400 - cramped_position, 180);
}
