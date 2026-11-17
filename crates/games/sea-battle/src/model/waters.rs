use mg_core::StateHasher;

use crate::model::{Cell, CellMark, FLEET_LENGTHS, Ship};

/// One seat's sea as everyone sees it: where it was shot at and what was sunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Waters {
    marks: Vec<CellMark>,
    sunk: Vec<Ship>,
}

impl Default for Waters {
    fn default() -> Self {
        Self {
            marks: vec![CellMark::Unknown; Cell::COUNT],
            sunk: Vec::new(),
        }
    }
}

impl Waters {
    pub fn mark(&self, cell: Cell) -> CellMark {
        self.marks.get(cell.index()).copied().unwrap_or_default()
    }

    /// Row by row, [`Cell::COUNT`] of them.
    pub fn marks(&self) -> &[CellMark] {
        &self.marks
    }

    /// In the order they went down.
    pub fn sunk(&self) -> &[Ship] {
        &self.sunk
    }

    pub fn is_all_sunk(&self) -> bool {
        self.sunk.len() == FLEET_LENGTHS.len()
    }

    /// The lengths of the ships still afloat, longest first.
    pub fn afloat(&self) -> Vec<u8> {
        let mut afloat = FLEET_LENGTHS.to_vec();
        for ship in &self.sunk {
            if let Some(index) = afloat.iter().position(|length| *length == ship.length()) {
                afloat.remove(index);
            }
        }
        afloat
    }

    pub(crate) fn set(&mut self, cell: Cell, mark: CellMark) {
        if let Some(slot) = self.marks.get_mut(cell.index()) {
            *slot = mark;
        }
    }

    /// Marks the ship sunk and the water around it cleared.
    pub(crate) fn sink(&mut self, ship: Ship) {
        for cell in ship.cells() {
            self.set(cell, CellMark::Sunk);
        }
        for cell in ship.cells().flat_map(Cell::around) {
            if self.mark(cell) == CellMark::Unknown {
                self.set(cell, CellMark::Cleared);
            }
        }
        self.sunk.push(ship);
    }

    pub(crate) fn hash(&self, hasher: &mut StateHasher) {
        for mark in &self.marks {
            hasher.u8(mark.hash_code());
        }
        hasher.length(self.sunk.len());
        for ship in &self.sunk {
            ship.hash(hasher);
        }
    }
}
