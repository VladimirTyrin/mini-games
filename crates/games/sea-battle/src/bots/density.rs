use crate::model::{Cell, CellMark, Orientation, Ship, Waters};

/// How strongly a placement through a wounded ship counts against one through open water: a ship
/// already hit there is far likelier than one that merely could be.
const WOUNDED_WEIGHT: u32 = 50;

/// For every cell, how many placements of the ships still afloat could cover it, given what the
/// sea shows. While a ship is wounded only placements through its hits count, so the density
/// points along it. Row by row, [`Cell::COUNT`] of them.
pub(crate) fn ship_density(waters: &Waters) -> Vec<u32> {
    let mut density = vec![0u32; Cell::COUNT];
    let has_wounded = Cell::all().any(|cell| waters.mark(cell) == CellMark::Hit);
    for length in waters.afloat() {
        for bow in Cell::all() {
            for orientation in [Orientation::Horizontal, Orientation::Vertical] {
                let Some(ship) = Ship::new(bow, length, orientation) else {
                    continue;
                };
                let mut wounded = 0;
                let mut possible = true;
                for cell in ship.cells() {
                    match waters.mark(cell) {
                        CellMark::Unknown => {}
                        CellMark::Hit => wounded += 1,
                        CellMark::Miss | CellMark::Sunk | CellMark::Cleared => possible = false,
                    }
                }
                if !possible || (has_wounded && wounded == 0) {
                    continue;
                }
                let weight = if wounded > 0 {
                    WOUNDED_WEIGHT * wounded
                } else {
                    1
                };
                for cell in ship.cells() {
                    if waters.mark(cell).is_open()
                        && let Some(count) = density.get_mut(cell.index())
                    {
                        *count += weight;
                    }
                }
            }
        }
    }
    density
}
