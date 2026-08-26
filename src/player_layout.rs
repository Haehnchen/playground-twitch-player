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
}

const SINGLE_CELLS: &[PlayerLayoutCell] = &[PlayerLayoutCell::new(0, 0, 1, 1)];

const GRID_2X2_CELLS: &[PlayerLayoutCell] = &[
    PlayerLayoutCell::new(0, 0, 1, 1),
    PlayerLayoutCell::new(1, 0, 1, 1),
    PlayerLayoutCell::new(0, 1, 1, 1),
    PlayerLayoutCell::new(1, 1, 1, 1),
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

pub const PLAYER_LAYOUT_SINGLE_INDEX: usize = 0;
pub const PLAYER_LAYOUT_2X2_INDEX: usize = 1;

// Adding another layout only requires a name and a cell array here. The icon,
// button and GTK grid arrangement all consume this same configuration.
pub const PLAYER_LAYOUTS: &[PlayerLayout] = &[
    PlayerLayout {
        name: "Single stream",
        cells: SINGLE_CELLS,
    },
    PlayerLayout {
        name: "2x2 grid",
        cells: GRID_2X2_CELLS,
    },
    PlayerLayout {
        name: "6-tile mosaic, large top left",
        cells: SIX_MOSAIC_TOP_LEFT_CELLS,
    },
    PlayerLayout {
        name: "7-tile mosaic",
        cells: SEVEN_MOSAIC_CELLS,
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
    fn additional_mosaics_preserve_the_window_aspect_ratio() {
        for layout in &PLAYER_LAYOUTS[2..] {
            assert_eq!(layout.column_count(), layout.row_count(), "{}", layout.name);
            assert!(
                layout
                    .cells
                    .iter()
                    .all(|cell| cell.column_span == cell.row_span),
                "{}",
                layout.name
            );
        }
    }
}
