use super::controls::show_tile_overlay;
use super::ffi::{
    G_SOURCE_CONTINUE, G_SOURCE_REMOVE, GParamSpec, GtkButton, GtkPaned, GtkWidget,
    gtk_button_set_child, gtk_paned_get_position, gtk_paned_set_end_child, gtk_paned_set_position,
    gtk_widget_get_width, gtk_widget_set_size_request, gtk_widget_set_tooltip_text,
};
use super::runtime::{add_tile_timeout, remove_source_if_active, tile_is_closing};
use super::{DEFAULT_CHAT_WIDTH, FALSE, PLAYER_CHAT_ICON_OPEN, StreamTile, TRUE};
use crate::chat::panel::{chat_panel_get_widget, chat_panel_start};
use crate::glib_strings::is_nonempty;
use crate::player::icons::player_chat_icon_new;
use std::ffi::{c_int, c_void};
use std::ptr;
use std::sync::atomic::Ordering;
pub(super) const MIN_CHAT_WIDTH: c_int = 120;
pub(super) const MAX_CHAT_WIDTH: c_int = 280;
pub(super) const MIN_VIDEO_WIDTH: c_int = 180;
pub(super) const MAX_CHAT_WIDTH_PERCENT: c_int = 45;
pub(super) const PLAYER_CHAT_ICON_CLOSE: c_int = 1;

pub(super) fn get_default_chat_width() -> c_int {
    DEFAULT_CHAT_WIDTH
}

pub(super) fn get_max_chat_width_for_pane(width: c_int) -> c_int {
    if width <= 1 {
        return get_default_chat_width();
    }

    // Tile chat must never take over the whole tile; keep video readable in 2x2.
    let by_fraction = (width * MAX_CHAT_WIDTH_PERCENT / 100).max(1);
    let by_video = (width - MIN_VIDEO_WIDTH).max(1);

    MAX_CHAT_WIDTH.min(by_fraction).min(by_video).max(1)
}

pub(super) fn clamp_chat_width_for_pane(width: c_int, chat_width: c_int) -> c_int {
    if width <= 1 {
        return if chat_width > 0 {
            chat_width.min(MAX_CHAT_WIDTH)
        } else {
            get_default_chat_width()
        };
    }

    let max_width = get_max_chat_width_for_pane(width);
    let min_width = MIN_CHAT_WIDTH.min(max_width);
    let preferred = if chat_width > 0 {
        chat_width
    } else {
        get_default_chat_width()
    };

    preferred.clamp(min_width, max_width)
}

pub(super) fn get_chat_width_for_position(width: c_int, position: c_int) -> c_int {
    if width > 1 && position > 0 {
        clamp_chat_width_for_pane(width, width - position)
    } else {
        get_default_chat_width()
    }
}

pub(super) fn get_chat_paned_position_for_width(chat_width: c_int, width: c_int) -> c_int {
    let chat_width = if chat_width > 0 {
        chat_width
    } else {
        get_default_chat_width()
    };

    if width > 1 {
        // GtkPaned stores divider position from the video side; we store chat width instead.
        width - clamp_chat_width_for_pane(width, chat_width)
    } else {
        0
    }
}

pub(super) unsafe fn update_chat_button(tile: *mut StreamTile) {
    unsafe {
        if (*tile).chat_toggle_button.is_null() {
            return;
        }

        let active = (*tile).chat_visible != 0;
        gtk_widget_set_tooltip_text(
            (*tile).chat_toggle_button,
            if active {
                c"Close chat".as_ptr()
            } else {
                c"Open chat".as_ptr()
            },
        );
        gtk_button_set_child(
            (*tile).chat_toggle_button as *mut GtkButton,
            player_chat_icon_new(if active {
                PLAYER_CHAT_ICON_CLOSE
            } else {
                PLAYER_CHAT_ICON_OPEN
            }),
        );
    }
}

pub(super) unsafe fn start_chat_for_tile(tile: *mut StreamTile) {
    unsafe {
        if (*tile).chat_panel.is_null() || !is_nonempty((*tile).channel) {
            return;
        }

        chat_panel_start((*tile).chat_panel, (*tile).channel);
    }
}

pub(super) unsafe fn capture_chat_width_from_paned(tile: *mut StreamTile) {
    unsafe {
        if (*tile).chat_visible == 0 || (*tile).container.is_null() {
            return;
        }

        let width = gtk_widget_get_width((*tile).container);
        let position = gtk_paned_get_position((*tile).container as *mut GtkPaned);
        if width > 1 && position > 0 {
            (*tile).chat_width = get_chat_width_for_position(width, position);
        }
    }
}

pub(super) unsafe fn apply_stored_chat_width(tile: *mut StreamTile, width: c_int) {
    unsafe {
        // Recompute the divider after tile/window resizes so chat stays bounded per tile.
        let position = get_chat_paned_position_for_width((*tile).chat_width, width);
        let chat_width = if width > 1 {
            width - position
        } else {
            clamp_chat_width_for_pane(width, (*tile).chat_width)
        };
        let chat_widget: *mut GtkWidget = chat_panel_get_widget((*tile).chat_panel);
        if !chat_widget.is_null() {
            gtk_widget_set_size_request(chat_widget, chat_width, -1);
        }
        gtk_paned_set_position((*tile).container as *mut GtkPaned, position);
        (*tile).chat_width = chat_width;
        (*tile).last_chat_paned_width = width;
    }
}

pub(super) unsafe fn queue_chat_position_update(tile: *mut StreamTile) {
    unsafe {
        if (*tile).chat_position_source == 0 {
            (*tile).chat_position_source = add_tile_timeout(50, Some(apply_chat_position), tile);
        }
    }
}

pub(super) unsafe extern "C" fn apply_chat_position(user_data: *mut c_void) -> c_int {
    unsafe {
        let tile = user_data as *mut StreamTile;
        let state = (*tile).app;

        if (*state).closing.load(Ordering::Acquire) || (*tile).container.is_null() {
            (*tile).chat_position_source = 0;
            return G_SOURCE_REMOVE;
        }

        let width = gtk_widget_get_width((*tile).container);
        if width <= 1 {
            return G_SOURCE_CONTINUE;
        }

        apply_stored_chat_width(tile, width);
        (*tile).chat_position_source = 0;

        G_SOURCE_REMOVE
    }
}

pub(super) unsafe extern "C" fn on_chat_paned_layout_changed(
    _paned: *mut GtkPaned,
    _pspec: *mut GParamSpec,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        let state = (*tile).app;
        if (*state).closing.load(Ordering::Acquire)
            || (*tile).chat_visible == 0
            || (*tile).container.is_null()
        {
            return;
        }

        let width = gtk_widget_get_width((*tile).container);
        if width <= 1 {
            return;
        }

        if width != (*tile).last_chat_paned_width {
            apply_stored_chat_width(tile, width);
        } else {
            capture_chat_width_from_paned(tile);
        }
    }
}

pub(super) unsafe fn activate_tile_chat(tile: *mut StreamTile, visible: c_int) {
    unsafe {
        if (*tile).container.is_null() || (*tile).chat_panel.is_null() {
            return;
        }

        if visible != 0 {
            if !is_nonempty((*tile).channel) {
                return;
            }

            // Each tile owns its chat panel; there is intentionally no global right sidebar.
            start_chat_for_tile(tile);
            if (*tile).chat_visible == 0 {
                let width = gtk_widget_get_width((*tile).container);
                if width > 1 {
                    let chat_widget: *mut GtkWidget = chat_panel_get_widget((*tile).chat_panel);
                    if !chat_widget.is_null() {
                        gtk_widget_set_size_request(
                            chat_widget,
                            clamp_chat_width_for_pane(width, (*tile).chat_width),
                            -1,
                        );
                    }
                }
                gtk_paned_set_end_child(
                    (*tile).container as *mut GtkPaned,
                    chat_panel_get_widget((*tile).chat_panel),
                );
                if width > 1 {
                    apply_stored_chat_width(tile, width);
                }
                queue_chat_position_update(tile);
            }
            (*tile).chat_visible = TRUE;
        } else {
            remove_source_if_active(&mut (*tile).chat_position_source);
            capture_chat_width_from_paned(tile);
            (*tile).chat_visible = FALSE;
            gtk_paned_set_end_child((*tile).container as *mut GtkPaned, ptr::null_mut());
        }

        update_chat_button(tile);
    }
}

pub(super) unsafe fn sync_chat_for_tile(tile: *mut StreamTile) {
    unsafe {
        if (*tile).chat_visible != 0 {
            start_chat_for_tile(tile);
            update_chat_button(tile);
        }
    }
}

pub(super) unsafe extern "C" fn on_tile_chat_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        let close_active = (*tile).chat_visible != 0;

        activate_tile_chat(tile, (!close_active) as c_int);
        show_tile_overlay(tile);
    }
}
