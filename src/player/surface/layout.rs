use super::chat::{activate_tile_chat, capture_chat_width_from_paned, queue_chat_position_update};
use super::controls::show_tile_overlay;
use super::ffi::{
    GtkButton, GtkGridLayoutChild, GtkWidget, g_cancellable_cancel, gtk_button_set_child,
    gtk_grid_layout_child_get_type, gtk_grid_layout_child_set_column,
    gtk_grid_layout_child_set_column_span, gtk_grid_layout_child_set_row,
    gtk_grid_layout_child_set_row_span, gtk_layout_manager_get_layout_child,
    gtk_widget_add_css_class, gtk_widget_get_layout_manager, gtk_widget_remove_css_class,
    gtk_widget_set_tooltip_text, gtk_widget_set_visible,
};
use super::playback::{
    ensure_tile_session, load_tile_stream, tile_has_stream, tile_has_visible_stream,
    update_tile_empty_state,
};
use super::render::{clear_tile_render_context, queue_tile_render};
use super::runtime::{
    clear_object, clear_pointer, disable_tile_wakeup_callback, is_instance,
    remove_source_if_active, tile_is_closing,
};
use super::stream_details::{
    hide_tile_twitch_playback_panel, set_tile_twitch_playback_not_checked,
};
use super::streams::{clear_tile_stream_qualities, set_tile_stream_title};
use super::{FALSE, MAX_TILES, PLAYER_TILE_FOCUS_ICON_EXPAND, PlayerSurface, StreamTile, TRUE};
use crate::player::icons::player_tile_focus_icon_new;
use crate::player::layout::{PLAYER_LAYOUT_2X2_INDEX, PLAYER_LAYOUT_SINGLE_INDEX, PLAYER_LAYOUTS};
use crate::player::session::{player_session_free, player_session_is_playing, player_session_stop};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

pub(super) const PLAYER_TILE_FOCUS_ICON_RESTORE: c_int = 1;

pub(super) unsafe fn get_grid_layout_child(
    state: *mut PlayerSurface,
    child: *mut GtkWidget,
) -> *mut GtkGridLayoutChild {
    unsafe {
        let layout = gtk_widget_get_layout_manager((*state).grid);
        let layout_child = gtk_layout_manager_get_layout_child(layout, child);

        if !is_instance(layout_child, gtk_grid_layout_child_get_type()) {
            return ptr::null_mut();
        }

        layout_child as *mut GtkGridLayoutChild
    }
}

pub(super) unsafe fn set_grid_item_layout(
    state: *mut PlayerSurface,
    widget: *mut GtkWidget,
    column: c_int,
    row: c_int,
    column_span: c_int,
    row_span: c_int,
) {
    unsafe {
        let child = get_grid_layout_child(state, widget);

        if child.is_null() {
            return;
        }

        gtk_grid_layout_child_set_column(child, column);
        gtk_grid_layout_child_set_row(child, row);
        gtk_grid_layout_child_set_column_span(child, column_span);
        gtk_grid_layout_child_set_row_span(child, row_span);
    }
}

pub(super) fn compact_tile_order(
    primary_index: usize,
    tiles_with_streams: &[bool; MAX_TILES],
) -> [usize; MAX_TILES] {
    let primary_index = primary_index.min(MAX_TILES - 1);
    let mut order = [0; MAX_TILES];
    let mut next_slot = 0;

    // Keep the focused tile first within its group, then preserve tile order.
    // Running this once for streams and once for empty tiles removes gaps without
    // moving the StreamTile structs, whose addresses are used by GTK callbacks.
    for want_stream in [true, false] {
        if tiles_with_streams[primary_index] == want_stream {
            order[next_slot] = primary_index;
            next_slot += 1;
        }

        for (tile_index, has_stream) in tiles_with_streams.iter().copied().enumerate() {
            if tile_index != primary_index && has_stream == want_stream {
                order[next_slot] = tile_index;
                next_slot += 1;
            }
        }
    }

    order
}

pub(super) unsafe fn current_compact_tile_order(
    state: *mut PlayerSurface,
    primary_index: usize,
) -> [usize; MAX_TILES] {
    unsafe {
        let mut tiles_with_streams = [false; MAX_TILES];
        for (tile_index, has_stream) in tiles_with_streams.iter_mut().enumerate() {
            *has_stream = tile_has_visible_stream(&mut (*state).tiles[tile_index]);
        }

        compact_tile_order(primary_index, &tiles_with_streams)
    }
}

pub(super) unsafe fn update_grid_item_borders(
    widget: *mut GtkWidget,
    layout_index: usize,
    slot: usize,
) {
    unsafe {
        gtk_widget_remove_css_class(widget, c"tile-left".as_ptr());
        gtk_widget_remove_css_class(widget, c"tile-top".as_ptr());

        let Some(layout) = PLAYER_LAYOUTS.get(layout_index) else {
            return;
        };
        let Some(cell) = layout.cells.get(slot) else {
            return;
        };

        if cell.column + cell.column_span < layout.column_count() {
            gtk_widget_add_css_class(widget, c"tile-left".as_ptr());
        }
        if cell.row + cell.row_span < layout.row_count() {
            gtk_widget_add_css_class(widget, c"tile-top".as_ptr());
        }
    }
}

pub(super) unsafe fn restore_active_layout_with_primary_slot(
    state: *mut PlayerSurface,
    primary_index: c_uint,
) {
    unsafe {
        let primary_index = primary_index.min((MAX_TILES - 1) as c_uint) as usize;
        let layout_index = (*state).active_layout_index.min(PLAYER_LAYOUTS.len() - 1);
        let layout = &PLAYER_LAYOUTS[layout_index];
        let tile_order = current_compact_tile_order(state, primary_index);
        let first_tile_index = tile_order[0];

        for (slot, tile_index) in tile_order.into_iter().enumerate() {
            if (*state).grid_items[tile_index].is_null() {
                continue;
            }

            let Some(cell) = layout.cells.get(slot) else {
                gtk_widget_set_visible((*state).grid_items[tile_index], FALSE);
                continue;
            };

            set_grid_item_layout(
                state,
                (*state).grid_items[tile_index],
                cell.column,
                cell.row,
                cell.column_span,
                cell.row_span,
            );
            update_grid_item_borders((*state).grid_items[tile_index], layout_index, slot);
            gtk_widget_set_visible((*state).grid_items[tile_index], TRUE);
        }

        (*state).tile_focused = FALSE;
        (*state).focused_tile = first_tile_index as c_uint;
    }
}

pub(super) unsafe fn preferred_single_tile(state: *mut PlayerSurface) -> *mut StreamTile {
    unsafe {
        if (*state).tile_focused != 0 && (*state).focused_tile < MAX_TILES as c_uint {
            let tile = &mut (*state).tiles[(*state).focused_tile as usize] as *mut StreamTile;
            if tile_has_visible_stream(tile) {
                return tile;
            }
        }

        for i in 0..MAX_TILES {
            let tile = &mut (*state).tiles[i] as *mut StreamTile;
            if player_session_is_playing((*tile).session) != 0 {
                return tile;
            }
        }

        for i in 0..MAX_TILES {
            let tile = &mut (*state).tiles[i] as *mut StreamTile;
            if tile_has_stream(tile) {
                return tile;
            }
        }

        &mut (*state).tiles[0] as *mut StreamTile
    }
}

pub(super) unsafe fn is_tile_focused(tile: *mut StreamTile) -> bool {
    unsafe {
        let state = (*tile).app;

        (*state).tile_focused != 0 && (*state).focused_tile == (*tile).index
    }
}

pub(super) unsafe fn update_tile_focus_buttons(state: *mut PlayerSurface) {
    unsafe {
        for i in 0..MAX_TILES {
            let tile = &mut (*state).tiles[i] as *mut StreamTile;
            if (*tile).focus_button.is_null() {
                continue;
            }

            let focused = is_tile_focused(tile);
            gtk_button_set_child(
                (*tile).focus_button as *mut GtkButton,
                player_tile_focus_icon_new(if focused {
                    PLAYER_TILE_FOCUS_ICON_RESTORE
                } else {
                    PLAYER_TILE_FOCUS_ICON_EXPAND
                }),
            );
            gtk_widget_set_tooltip_text(
                (*tile).focus_button,
                if focused {
                    c"Restore layout".as_ptr()
                } else {
                    c"Focus tile".as_ptr()
                },
            );
        }
    }
}

pub(super) unsafe fn focus_tile(tile: *mut StreamTile) {
    unsafe {
        let state = (*tile).app;

        for i in 0..MAX_TILES {
            if !(*state).grid_items[i].is_null() {
                gtk_widget_set_visible(
                    (*state).grid_items[i],
                    (i as c_uint == (*tile).index) as c_int,
                );
            }
        }

        let layout = &PLAYER_LAYOUTS[(*state).active_layout_index.min(PLAYER_LAYOUTS.len() - 1)];
        set_grid_item_layout(
            state,
            (*tile).frame,
            0,
            0,
            layout.column_count(),
            layout.row_count(),
        );
        (*state).focused_tile = (*tile).index;
        (*state).tile_focused = TRUE;
    }
}

pub(super) unsafe fn set_tile_single_presentation(tile: *mut StreamTile, single: c_int) {
    unsafe {
        if (*tile).frame.is_null() {
            return;
        }

        if single != 0 {
            gtk_widget_add_css_class((*tile).frame, c"single-template".as_ptr());
        } else {
            gtk_widget_remove_css_class((*tile).frame, c"single-template".as_ptr());
        }
    }
}

pub(super) unsafe fn set_all_tiles_single_presentation(
    state: *mut PlayerSurface,
    selected: *mut StreamTile,
) {
    unsafe {
        for i in 0..MAX_TILES {
            let tile = &mut (*state).tiles[i] as *mut StreamTile;
            set_tile_single_presentation(tile, (tile == selected) as c_int);
        }
    }
}

pub(super) unsafe fn reset_tile_for_template_switch(tile: *mut StreamTile) {
    unsafe {
        if (*tile).chat_visible != 0 {
            activate_tile_chat(tile, FALSE);
        }
        remove_source_if_active(&mut (*tile).chat_position_source);

        // Template switches are state resets for discarded tiles, not hidden grid slots.
        if !(*tile).title_cancel.is_null() {
            g_cancellable_cancel((*tile).title_cancel);
            clear_object(&mut (*tile).title_cancel);
        }
        (*tile).title_fetch_in_progress = FALSE;
        clear_tile_render_context(tile);
        clear_tile_stream_qualities(tile);
        hide_tile_twitch_playback_panel(tile);
        set_tile_twitch_playback_not_checked(tile);
        set_tile_stream_title(tile, c"".as_ptr(), c"".as_ptr());
        clear_pointer(&mut (*tile).label);
        clear_pointer(&mut (*tile).channel);

        if !(*tile).session.is_null() {
            disable_tile_wakeup_callback(tile);
            player_session_stop((*tile).session);
            if (*tile).owns_session != 0 {
                player_session_free((*tile).session);
            }
            (*tile).session = ptr::null_mut();
            (*tile).owns_session = TRUE;
        }

        update_tile_empty_state(tile);
        queue_tile_render(tile);
    }
}

pub(super) unsafe fn clear_surface_targets(state: *mut PlayerSurface) {
    unsafe {
        for i in 0..MAX_TILES {
            clear_pointer(&mut (*state).targets[i]);
        }
        (*state).target_count = 0;
    }
}

pub(super) unsafe fn reset_tiles_outside_layout(
    state: *mut PlayerSurface,
    layout_index: usize,
    primary_index: c_uint,
) {
    unsafe {
        let layout = &PLAYER_LAYOUTS[layout_index];
        let primary_index = primary_index.min((MAX_TILES - 1) as c_uint) as usize;
        let tile_order = current_compact_tile_order(state, primary_index);

        for tile_index in tile_order.into_iter().skip(layout.cells.len()) {
            reset_tile_for_template_switch(&mut (*state).tiles[tile_index]);
        }
    }
}

pub(super) unsafe fn enter_single_template(state: *mut PlayerSurface, tile: *mut StreamTile) {
    unsafe {
        if tile.is_null() {
            return;
        }

        clear_surface_targets(state);

        for i in 0..MAX_TILES {
            let other = &mut (*state).tiles[i] as *mut StreamTile;
            if other != tile {
                reset_tile_for_template_switch(other);
            }
        }

        // The selected GLArea/session stays mounted; the template change only resizes it.
        set_all_tiles_single_presentation(state, tile);
        focus_tile(tile);
        (*state).single_template_active = TRUE;
        (*state).single_template_tile = (*tile).index;
        (*state).video_fullscreen_active = FALSE;
        update_tile_focus_buttons(state);
        show_tile_overlay(tile);
    }
}

pub(super) unsafe fn leave_single_template(state: *mut PlayerSurface) {
    unsafe {
        let single_tile = if (*state).focused_tile < MAX_TILES as c_uint {
            (*state).focused_tile
        } else {
            0
        };

        (*state).single_template_active = FALSE;
        set_all_tiles_single_presentation(state, ptr::null_mut());
        restore_active_layout_with_primary_slot(state, single_tile);
        update_tile_focus_buttons(state);

        for i in 0..MAX_TILES {
            let tile = &mut (*state).tiles[i] as *mut StreamTile;
            if !tile_has_stream(tile) || player_session_is_playing((*tile).session) != 0 {
                continue;
            }

            if ensure_tile_session(tile) != 0 {
                load_tile_stream(tile);
            }
        }
    }
}

pub(super) unsafe fn toggle_tile_focus(tile: *mut StreamTile) {
    unsafe {
        let state = (*tile).app;

        if (*state).single_template_active != 0 && (*state).single_template_tile == (*tile).index {
            leave_single_template(state);
            show_tile_overlay(tile);
            return;
        }

        enter_single_template(state, tile);
    }
}

pub(super) unsafe extern "C" fn on_tile_focus_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if !tile_is_closing(tile) {
            toggle_tile_focus(tile);
        }
    }
}

pub(super) unsafe fn get_target_count(state: *mut PlayerSurface) -> c_uint {
    unsafe { (*state).target_count.min(MAX_TILES as c_uint) }
}

pub(super) unsafe fn get_target_at(state: *mut PlayerSurface, index: c_uint) -> *const c_char {
    unsafe {
        if index < (*state).target_count {
            (*state).targets[index as usize]
        } else {
            ptr::null()
        }
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
pub unsafe fn player_surface_apply_single_template(player: *mut PlayerSurface) {
    unsafe {
        player_surface_apply_layout(player, PLAYER_LAYOUT_SINGLE_INDEX);
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
pub unsafe fn player_surface_apply_2x2_template(player: *mut PlayerSurface) {
    unsafe {
        player_surface_apply_layout(player, PLAYER_LAYOUT_2X2_INDEX);
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
pub unsafe fn player_surface_apply_layout(player: *mut PlayerSurface, layout_index: usize) {
    unsafe {
        if player.is_null() {
            return;
        }
        let Some(layout) = PLAYER_LAYOUTS.get(layout_index) else {
            return;
        };

        if layout.is_single() {
            let tile = preferred_single_tile(player);
            enter_single_template(player, tile);
            return;
        }

        if (*player).video_fullscreen_active != 0 {
            restore_video_fullscreen_layout(player, ptr::null_mut());
        }

        let primary_index = (*player).focused_tile.min((MAX_TILES - 1) as c_uint);
        (*player).active_layout_index = layout_index;
        reset_tiles_outside_layout(player, layout_index, primary_index);
        leave_single_template(player);
        show_tile_overlay(preferred_single_tile(player));
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
pub unsafe fn player_surface_is_single_template(player: *mut PlayerSurface) -> c_int {
    unsafe { (!player.is_null() && (*player).single_template_active != 0) as c_int }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
pub unsafe fn player_surface_get_layout_index(player: *mut PlayerSurface) -> usize {
    unsafe {
        if player.is_null() {
            return PLAYER_LAYOUT_SINGLE_INDEX;
        }

        if (*player).single_template_active != 0 {
            PLAYER_LAYOUT_SINGLE_INDEX
        } else {
            (*player).active_layout_index
        }
    }
}

pub(super) fn player_surface_fullscreen_should_restore(
    video_fullscreen_active: bool,
    app_fullscreen: bool,
    tile_focused: bool,
    focused_tile: c_uint,
    tile_index: c_uint,
) -> bool {
    video_fullscreen_active || (app_fullscreen && tile_focused && focused_tile == tile_index)
}

pub(super) fn player_surface_fullscreen_should_exit_app(
    app_fullscreen: bool,
    video_fullscreen_active: bool,
    restore_app_fullscreen: bool,
) -> bool {
    app_fullscreen && (!video_fullscreen_active || !restore_app_fullscreen)
}

pub(super) unsafe fn apply_video_fullscreen_focus(tile: *mut StreamTile) {
    unsafe {
        let state = (*tile).app;

        if (*state).closing.load(Ordering::Acquire) {
            return;
        }

        if (*state).single_template_active != 0 {
            if !is_tile_focused(tile) {
                focus_tile(tile);
            }
            set_all_tiles_single_presentation(state, tile);
            show_tile_overlay(tile);
            return;
        }

        // Video fullscreen from 2x2 is temporary focus, not a template switch; keep other sessions.
        set_all_tiles_single_presentation(state, tile);
        if !is_tile_focused(tile) {
            focus_tile(tile);
        }
        show_tile_overlay(tile);
    }
}

// Restore the grid and focused tile after video fullscreen.
pub(super) unsafe fn restore_video_fullscreen_layout(
    state: *mut PlayerSurface,
    tile: *mut StreamTile,
) {
    unsafe {
        let restore_tile_focused = (*state).video_fullscreen_active != 0
            && (*state).video_fullscreen_restore_tile_focused != 0;
        let restore_focused_tile = (*state).video_fullscreen_restore_focused_tile;

        (*state).video_fullscreen_active = FALSE;

        if (*state).single_template_active != 0
            && (*state).single_template_tile < MAX_TILES as c_uint
            && !(*state).grid_items[(*state).single_template_tile as usize].is_null()
        {
            let single_tile = &mut (*state).tiles[(*state).single_template_tile as usize];
            set_all_tiles_single_presentation(state, single_tile);
            focus_tile(single_tile);
        } else if restore_tile_focused
            && restore_focused_tile < MAX_TILES as c_uint
            && !(*state).grid_items[restore_focused_tile as usize].is_null()
        {
            set_all_tiles_single_presentation(state, ptr::null_mut());
            focus_tile(&mut (*state).tiles[restore_focused_tile as usize]);
        } else {
            set_all_tiles_single_presentation(state, ptr::null_mut());
            restore_active_layout_with_primary_slot(state, restore_focused_tile);
        }

        update_tile_focus_buttons(state);
        if !tile.is_null() {
            show_tile_overlay(tile);
        }
    }
}

pub(super) unsafe fn request_tile_fullscreen_toggle(tile: *mut StreamTile) {
    unsafe {
        let state = (*tile).app;

        if (*state).single_template_active != 0 && (*state).single_template_tile == (*tile).index {
            if let Some(callback) = (*state).fullscreen_callback {
                callback((*state).fullscreen_user_data);
            }
            show_tile_overlay(tile);
            return;
        }

        let video_fullscreen_active = (*state).video_fullscreen_active;

        if player_surface_fullscreen_should_restore(
            video_fullscreen_active != 0,
            (*state).fullscreen != 0,
            (*state).tile_focused != 0,
            (*state).focused_tile,
            (*tile).index,
        ) {
            // Second double-click restores the previous template/focus state.
            let exit_app_fullscreen = player_surface_fullscreen_should_exit_app(
                (*state).fullscreen != 0,
                video_fullscreen_active != 0,
                (*state).video_fullscreen_restore_app_fullscreen != 0,
            );

            restore_video_fullscreen_layout(state, tile);
            if exit_app_fullscreen && let Some(callback) = (*state).fullscreen_callback {
                callback((*state).fullscreen_user_data);
            }
            return;
        }

        (*state).video_fullscreen_restore_app_fullscreen = (*state).fullscreen;
        (*state).video_fullscreen_restore_tile_focused = (*state).tile_focused;
        (*state).video_fullscreen_restore_focused_tile = (*state).focused_tile;
        (*state).video_fullscreen_active = TRUE;

        apply_video_fullscreen_focus(tile);

        if (*state).fullscreen == 0
            && let Some(callback) = (*state).fullscreen_callback
        {
            callback((*state).fullscreen_user_data);
        }
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `player`: null or live instance.
pub unsafe fn player_surface_set_fullscreen(player: *mut PlayerSurface, fullscreen: c_int) {
    unsafe {
        if !player.is_null() {
            // Capture before GTK changes allocation; reapply after so chats do not grow with fullscreen.
            for i in 0..MAX_TILES {
                let tile = &mut (*player).tiles[i] as *mut StreamTile;
                capture_chat_width_from_paned(tile);
            }
            (*player).fullscreen = fullscreen;
            for i in 0..MAX_TILES {
                let tile = &mut (*player).tiles[i] as *mut StreamTile;
                if (*tile).chat_visible != 0 {
                    queue_chat_position_update(tile);
                }
            }
            if fullscreen == 0 && (*player).video_fullscreen_active != 0 {
                restore_video_fullscreen_layout(player, ptr::null_mut());
            }
        }
    }
}
