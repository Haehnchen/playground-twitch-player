use super::chat::{
    get_chat_paned_position_for_width as position_for,
    get_chat_width_for_position as chat_width_for,
};
use super::layout::compact_tile_order;
use super::layout::{
    player_surface_fullscreen_should_exit_app as should_exit_app,
    player_surface_fullscreen_should_restore as should_restore,
};
use super::{
    MAX_TILES, PlayerSurface, player_surface_free, player_surface_ref, player_surface_unref,
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn stopping_an_owned_session_does_not_create_an_idle_replacement() {
    unsafe {
        let mut surface = PlayerSurface::default();
        let mut tile = super::StreamTile {
            app: &mut surface,
            session: crate::player::session::player_session_new(),
            owns_session: 1,
            ..Default::default()
        };
        super::playback::reset_owned_tile_session(&mut tile);
        assert!(tile.session.is_null());
    }
}

#[test]
fn compact_tile_order_moves_streams_ahead_of_grid_gaps() {
    let mut tiles_with_streams = [false; MAX_TILES];
    tiles_with_streams[0] = true;
    tiles_with_streams[5] = true;

    let order = compact_tile_order(0, &tiles_with_streams);

    assert_eq!(order, [0, 5, 1, 2, 3, 4, 6]);
    assert!(order[..4].contains(&5));
}

#[test]
fn compact_tile_order_keeps_a_focused_stream_first() {
    let mut tiles_with_streams = [false; MAX_TILES];
    tiles_with_streams[0] = true;
    tiles_with_streams[5] = true;

    assert_eq!(
        compact_tile_order(5, &tiles_with_streams),
        [5, 0, 1, 2, 3, 4, 6]
    );
}

#[test]
fn compact_tile_order_does_not_prioritize_an_empty_focused_tile() {
    let mut tiles_with_streams = [false; MAX_TILES];
    tiles_with_streams[0] = true;
    tiles_with_streams[5] = true;

    assert_eq!(
        compact_tile_order(4, &tiles_with_streams),
        [0, 5, 4, 1, 2, 3, 6]
    );
}

#[test]
fn owner_release_waits_for_pending_callback_reference() {
    unsafe {
        let state = Box::into_raw(Box::new(PlayerSurface {
            ref_count: AtomicUsize::new(1),
            ..Default::default()
        }));
        let pending = player_surface_ref(state);

        player_surface_free(state);

        assert!((*pending).closing.load(Ordering::Acquire));
        assert_eq!((*pending).ref_count.load(Ordering::Acquire), 1);

        player_surface_unref(pending);
    }
}

#[test]
fn fullscreen_restore_and_exit() {
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

#[test]
fn chat_width_keeps_video_visible() {
    assert_eq!(1100 - position_for(220, 1100), 220);
    assert_eq!(1920 - position_for(460, 1920), 280);
    assert_eq!(chat_width_for(1100, 1100 - 460), 280);
    assert_eq!(512 - position_for(220, 512), 220);
    assert_eq!(400 - position_for(220, 400), 180);
    assert_eq!(300 - position_for(220, 300), 120);
}
