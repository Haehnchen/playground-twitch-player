#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerLayoutCell {
    pub column: i32,
    pub row: i32,
    pub column_span: i32,
    pub row_span: i32,
}

impl PlayerLayoutCell {
    pub const fn new(column: i32, row: i32, column_span: i32, row_span: i32) -> Self {
        Self {
            column,
            row,
            column_span,
            row_span,
        }
    }
}

#[derive(Debug)]
pub struct PlayerLayout {
    pub name: &'static str,
    pub target_aspect: (i32, i32),
    pub priority_cell_count: usize,
    pub cells: &'static [PlayerLayoutCell],
}

impl PlayerLayout {
    pub fn column_count(&self) -> i32 {
        self.cells
            .iter()
            .map(|cell| cell.column + cell.column_span)
            .max()
            .unwrap_or(1)
    }

    pub fn row_count(&self) -> i32 {
        self.cells
            .iter()
            .map(|cell| cell.row + cell.row_span)
            .max()
            .unwrap_or(1)
    }

    pub fn is_single(&self) -> bool {
        self.cells.len() == 1
    }

    pub fn is_three_by_two(&self) -> bool {
        self.target_aspect == (3, 2)
    }
}

const SINGLE_CELLS: &[PlayerLayoutCell] = &[PlayerLayoutCell::new(0, 0, 1, 1)];

const GRID_2X2_CELLS: &[PlayerLayoutCell] = &[
    PlayerLayoutCell::new(0, 0, 1, 1),
    PlayerLayoutCell::new(1, 0, 1, 1),
    PlayerLayoutCell::new(0, 1, 1, 1),
    PlayerLayoutCell::new(1, 1, 1, 1),
];

// On the measured 3:2 screen, a 16/27-wide half-height tile is exactly 16:9.
// The first two slots therefore get the wider left column; the less important
// right column is split into three nearly 16:9 tiles.
const FIVE_3X2_LEFT_PRIORITY_CELLS: &[PlayerLayoutCell] = &[
    PlayerLayoutCell::new(0, 0, 16, 3),
    PlayerLayoutCell::new(0, 3, 16, 3),
    PlayerLayoutCell::new(16, 0, 11, 2),
    PlayerLayoutCell::new(16, 2, 11, 2),
    PlayerLayoutCell::new(16, 4, 11, 2),
];

const SIX_MOSAIC_TOP_LEFT_CELLS: &[PlayerLayoutCell] = &[
    PlayerLayoutCell::new(0, 0, 2, 2),
    PlayerLayoutCell::new(2, 0, 1, 1),
    PlayerLayoutCell::new(2, 1, 1, 1),
    PlayerLayoutCell::new(0, 2, 1, 1),
    PlayerLayoutCell::new(1, 2, 1, 1),
    PlayerLayoutCell::new(2, 2, 1, 1),
];

const SEVEN_MOSAIC_CELLS: &[PlayerLayoutCell] = &[
    PlayerLayoutCell::new(0, 0, 2, 2),
    PlayerLayoutCell::new(2, 0, 2, 2),
    PlayerLayoutCell::new(0, 2, 2, 2),
    PlayerLayoutCell::new(2, 2, 1, 1),
    PlayerLayoutCell::new(3, 2, 1, 1),
    PlayerLayoutCell::new(2, 3, 1, 1),
    PlayerLayoutCell::new(3, 3, 1, 1),
];

// A 9x10 logical canvas keeps every tile at the same 5:3 aspect ratio on a
// 3:2 display, close enough to 16:9 to avoid visibly uneven tile sizing.
const SIX_TALL_SCREEN_CELLS: &[PlayerLayoutCell] = &[
    PlayerLayoutCell::new(0, 0, 5, 5),
    PlayerLayoutCell::new(5, 0, 4, 4),
    PlayerLayoutCell::new(5, 4, 4, 4),
    PlayerLayoutCell::new(0, 5, 5, 5),
    PlayerLayoutCell::new(5, 8, 2, 2),
    PlayerLayoutCell::new(7, 8, 2, 2),
];

const FIVE_TALL_SCREEN_CELLS: &[PlayerLayoutCell] = &[
    PlayerLayoutCell::new(0, 0, 12, 12),
    PlayerLayoutCell::new(12, 0, 6, 6),
    PlayerLayoutCell::new(12, 6, 6, 6),
    PlayerLayoutCell::new(0, 12, 9, 8),
    PlayerLayoutCell::new(9, 12, 9, 8),
];

pub const PLAYER_LAYOUT_SINGLE_INDEX: usize = 0;
pub const PLAYER_LAYOUT_2X2_INDEX: usize = 1;

// Adding another layout only requires one entry and a cell array here. The
// icon, button and GTK grid arrangement all consume this same configuration.
pub const PLAYER_LAYOUTS: &[PlayerLayout] = &[
    PlayerLayout {
        name: "Single stream",
        target_aspect: (16, 9),
        priority_cell_count: 1,
        cells: SINGLE_CELLS,
    },
    PlayerLayout {
        name: "2x2 grid",
        target_aspect: (16, 9),
        priority_cell_count: 4,
        cells: GRID_2X2_CELLS,
    },
    PlayerLayout {
        name: "6-tile mosaic, large top left",
        target_aspect: (16, 9),
        priority_cell_count: 6,
        cells: SIX_MOSAIC_TOP_LEFT_CELLS,
    },
    PlayerLayout {
        name: "7-tile mosaic",
        target_aspect: (16, 9),
        priority_cell_count: 7,
        cells: SEVEN_MOSAIC_CELLS,
    },
    PlayerLayout {
        name: "6-tile 3:2 mosaic",
        target_aspect: (3, 2),
        priority_cell_count: 6,
        cells: SIX_TALL_SCREEN_CELLS,
    },
    PlayerLayout {
        name: "5-tile 3:2, left priority",
        target_aspect: (3, 2),
        priority_cell_count: 5,
        cells: FIVE_3X2_LEFT_PRIORITY_CELLS,
    },
    PlayerLayout {
        name: "5-tile 3:2 mosaic",
        target_aspect: (3, 2),
        priority_cell_count: 5,
        cells: FIVE_TALL_SCREEN_CELLS,
    },
];

const fn maximum_tile_count(layouts: &[PlayerLayout]) -> usize {
    let mut maximum = 0;
    let mut index = 0;
    while index < layouts.len() {
        if layouts[index].cells.len() > maximum {
            maximum = layouts[index].cells.len();
        }
        index += 1;
    }
    maximum
}

pub const PLAYER_LAYOUT_MAX_TILES: usize = maximum_tile_count(PLAYER_LAYOUTS);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_layouts_are_rectangular_and_do_not_overlap() {
        for layout in PLAYER_LAYOUTS {
            let columns = layout.column_count();
            let rows = layout.row_count();
            let mut occupied = vec![false; (columns * rows) as usize];

            for cell in layout.cells {
                assert!(cell.column >= 0 && cell.row >= 0, "{}", layout.name);
                assert!(cell.column_span > 0 && cell.row_span > 0, "{}", layout.name);

                for row in cell.row..cell.row + cell.row_span {
                    for column in cell.column..cell.column + cell.column_span {
                        let index = (row * columns + column) as usize;
                        assert!(!occupied[index], "{} overlaps", layout.name);
                        occupied[index] = true;
                    }
                }
            }

            assert!(
                occupied.into_iter().all(|cell| cell),
                "{} has gaps",
                layout.name
            );
        }
    }

    #[test]
    fn maximum_tile_count_comes_from_the_layout_list() {
        assert_eq!(PLAYER_LAYOUT_MAX_TILES, 7);
    }

    #[test]
    fn three_by_two_layouts_form_the_final_menu_section() {
        let first_three_by_two = PLAYER_LAYOUTS
            .iter()
            .position(PlayerLayout::is_three_by_two)
            .expect("at least one 3:2 layout");

        assert!(PLAYER_LAYOUTS[..first_three_by_two]
            .iter()
            .all(|layout| !layout.is_three_by_two()));
        assert!(PLAYER_LAYOUTS[first_three_by_two..]
            .iter()
            .all(PlayerLayout::is_three_by_two));
    }

    #[test]
    fn every_priority_tile_stays_close_to_16_by_9_at_its_layout_target_aspect() {
        for layout in PLAYER_LAYOUTS {
            let (viewport_width, viewport_height) = layout.target_aspect;
            let columns = layout.column_count();
            let rows = layout.row_count();

            for (cell_index, cell) in layout
                .cells
                .iter()
                .take(layout.priority_cell_count)
                .enumerate()
            {
                let actual = 9 * viewport_width * rows * cell.column_span;
                let ideal = 16 * viewport_height * columns * cell.row_span;
                let error = (actual - ideal).abs();

                assert!(
                    error * 100 <= ideal * 7,
                    "{} cell {}",
                    layout.name,
                    cell_index
                );
            }
        }
    }
}
