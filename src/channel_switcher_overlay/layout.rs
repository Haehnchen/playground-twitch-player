//! Panel and card dimensions.

use super::ChannelSwitcherOverlay;
use super::ffi::*;
use std::ffi::{c_double, c_int, c_uint};

const PANEL_MARGIN: c_int = 12;
const PANEL_TOP_SAFE_MARGIN: c_int = 48;
const PANEL_BOTTOM_MARGIN: c_int = 64;
const PANEL_EXTRA_VERTICAL_SPACE: c_int = 36;
const PANEL_MIN_WIDTH: c_int = 430;
const PANEL_MAX_WIDTH: c_int = 1300;
const PANEL_MAX_COLUMNS: c_uint = 4;
const PANEL_HORIZONTAL_PADDING: c_int = 16;
const PANEL_SCROLLBAR_RESERVE_WIDTH: c_int = 18;
const CARD_WIDTH: c_int = 226;
const CARD_HORIZONTAL_PADDING: c_int = 10;
pub(super) const CARD_SPACING: c_int = 6;
pub(super) const AVATAR_SIZE: c_int = 24;
const CARD_OUTER_WIDTH: c_int = CARD_WIDTH + CARD_HORIZONTAL_PADDING;

fn calculate_panel_width(overlay_width: c_int) -> c_int {
    let available_width = 1.max(overlay_width - PANEL_MARGIN * 2);
    let max_panel_width = PANEL_MAX_WIDTH.min(available_width);
    let min_panel_width = PANEL_MIN_WIDTH.min(available_width);

    max_panel_width.max(min_panel_width).min(max_panel_width)
}

fn calculate_grid_width(panel_width: c_int) -> c_int {
    1.max(panel_width - PANEL_HORIZONTAL_PADDING - PANEL_SCROLLBAR_RESERVE_WIDTH)
}

fn calculate_grid_columns(grid_width: c_int) -> c_uint {
    PANEL_MAX_COLUMNS
        .min(1.max((grid_width + CARD_SPACING) / (CARD_OUTER_WIDTH + CARD_SPACING)) as c_uint)
}

fn calculate_card_width(grid_width: c_int, columns: c_uint) -> c_int {
    let spacing_width = (if columns > 0 { columns - 1 } else { 0 }) as c_int * CARD_SPACING;
    let card_outer_width = (grid_width - spacing_width) / 1.max(columns as c_int);

    CARD_WIDTH.max(card_outer_width - CARD_HORIZONTAL_PADDING)
}

fn calculate_preview_height(preview_width: c_int) -> c_int {
    1.max((preview_width * 9 + 8) / 16)
}

pub(super) struct CardLayout {
    pub(super) columns: c_uint,
    pub(super) card_width: c_int,
    pub(super) preview_width: c_int,
    pub(super) preview_height: c_int,
}

// Calculate dimensions without reading GTK state.
fn calculate_card_layout(panel_width: c_int) -> CardLayout {
    let grid_width = calculate_grid_width(panel_width);
    let columns = calculate_grid_columns(grid_width);
    let card_width = calculate_card_width(grid_width, columns);

    CardLayout {
        columns,
        card_width,
        preview_width: card_width,
        preview_height: calculate_preview_height(card_width),
    }
}

pub(super) unsafe fn current_card_layout(switcher: *mut ChannelSwitcherOverlay) -> CardLayout {
    unsafe {
        let mut panel_width = 0;

        if !(*switcher).overlay.is_null() {
            panel_width =
                calculate_panel_width(gtk_widget_get_width((*switcher).overlay as *mut GtkWidget));
        } else if !(*switcher).panel.is_null() {
            panel_width = gtk_widget_get_width((*switcher).panel);
        }

        calculate_card_layout(panel_width)
    }
}

pub(super) unsafe fn get_grid_columns(switcher: *mut ChannelSwitcherOverlay) -> c_uint {
    unsafe { current_card_layout(switcher).columns }
}

pub(super) unsafe fn position_panel(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if (*switcher).overlay.is_null()
            || (*switcher).panel.is_null()
            || (*switcher).scroller.is_null()
        {
            return;
        }

        let overlay_width = gtk_widget_get_width((*switcher).overlay as *mut GtkWidget);
        let overlay_height = gtk_widget_get_height((*switcher).overlay as *mut GtkWidget);
        let mut root_y = PANEL_TOP_SAFE_MARGIN as c_double;
        let root = gtk_widget_get_root((*switcher).overlay as *mut GtkWidget);
        if !root.is_null() {
            let origin = GraphenePoint {
                x: 0.0,
                y: 0.0,
            };
            let mut root_point = GraphenePoint {
                x: 0.0,
                y: 0.0,
            };
            if gtk_widget_compute_point(
                (*switcher).overlay as *mut GtkWidget,
                root as *mut GtkWidget,
                &origin,
                &mut root_point,
            ) != 0
            {
                root_y = root_point.y as c_double;
            }
        }

        let top_margin = if root_y < PANEL_TOP_SAFE_MARGIN as c_double {
            PANEL_TOP_SAFE_MARGIN - root_y as c_int
        } else {
            PANEL_MARGIN
        };
        let panel_width = calculate_panel_width(overlay_width);
        let grid_width = calculate_grid_width(panel_width);
        let scroller_height =
            1.max(overlay_height - top_margin - PANEL_BOTTOM_MARGIN - PANEL_EXTRA_VERTICAL_SPACE);

        gtk_widget_set_size_request((*switcher).panel, panel_width, -1);
        gtk_scrolled_window_set_min_content_width(
            (*switcher).scroller as *mut GtkScrolledWindow,
            grid_width,
        );
        gtk_scrolled_window_set_max_content_width(
            (*switcher).scroller as *mut GtkScrolledWindow,
            grid_width,
        );
        gtk_scrolled_window_set_max_content_height(
            (*switcher).scroller as *mut GtkScrolledWindow,
            scroller_height,
        );
        gtk_widget_set_margin_start((*switcher).panel, PANEL_MARGIN);
        gtk_widget_set_margin_end((*switcher).panel, PANEL_MARGIN);
        gtk_widget_set_margin_top((*switcher).panel, top_margin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_uses_available_columns_up_to_four() {
        for (grid_width, expected_columns) in [(250, 1), (500, 2), (1000, 4), (1800, 4)] {
            let panel_width = grid_width + PANEL_HORIZONTAL_PADDING + PANEL_SCROLLBAR_RESERVE_WIDTH;
            assert_eq!(calculate_card_layout(panel_width).columns, expected_columns);
        }
    }

    #[test]
    fn cards_fit_the_panel_and_keep_widescreen_previews() {
        for overlay_width in [320, 640, 1280, 1920, 3840] {
            let panel_width = calculate_panel_width(overlay_width);
            let layout = calculate_card_layout(panel_width);
            let columns = layout.columns as c_int;
            let occupied_width = columns * (layout.card_width + CARD_HORIZONTAL_PADDING)
                + (columns - 1) * CARD_SPACING;

            assert!(panel_width <= overlay_width - PANEL_MARGIN * 2);
            assert!(panel_width <= PANEL_MAX_WIDTH);
            assert!(occupied_width <= calculate_grid_width(panel_width));
            assert!(layout.card_width >= CARD_WIDTH);
            assert!((layout.preview_height * 16 - layout.preview_width * 9).abs() <= 8);
        }
    }
}
