use mg_core::StateHasher;

use crate::model::{Cell, Orientation};

/// A ship on the board: from its bow (top or left end) `length` cells along `orientation`. In
/// bounds by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ship {
    bow: Cell,
    length: u8,
    orientation: Orientation,
}

impl Ship {
    pub const MAX_LENGTH: u8 = 4;

    /// `None` when the ship would leave the board or its length is not 1–4.
    pub fn new(bow: Cell, length: u8, orientation: Orientation) -> Option<Self> {
        if !(1..=Self::MAX_LENGTH).contains(&length) {
            return None;
        }
        let (dx, dy) = orientation.step();
        let last = i8::try_from(length - 1).ok()?;
        bow.offset(dx * last, dy * last)?;
        Some(Self {
            bow,
            length,
            orientation,
        })
    }

    pub const fn bow(self) -> Cell {
        self.bow
    }

    pub const fn length(self) -> u8 {
        self.length
    }

    pub const fn orientation(self) -> Orientation {
        self.orientation
    }

    pub fn cells(self) -> impl Iterator<Item = Cell> {
        let (dx, dy) = self.orientation.step();
        (0..self.length).filter_map(move |step| {
            let step = i8::try_from(step).ok()?;
            self.bow.offset(dx * step, dy * step)
        })
    }

    pub fn covers(self, cell: Cell) -> bool {
        self.cells().any(|own| own == cell)
    }

    /// Whether the ship stands on or right next to (side or corner) `cell`.
    pub fn touches(self, cell: Cell) -> bool {
        self.cells()
            .any(|own| own == cell || own.around().any(|near| near == cell))
    }

    pub(crate) fn hash(self, hasher: &mut StateHasher) {
        self.bow.hash(hasher);
        hasher.u8(self.length);
        hasher.u8(self.orientation.hash_code());
    }
}
