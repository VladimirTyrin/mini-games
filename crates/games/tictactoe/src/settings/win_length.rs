use mg_core::{Capped, OutOfRange, SettingRange};

use crate::settings::{Dimensions, FieldSize};

type WinCount = Capped<3, { FieldSize::MAX }>;

/// Marks in a row needed to win: at least 3 and at most the board's shorter side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct WinLength(WinCount);

impl WinLength {
    pub(crate) const FIELD: &'static str = "win_count";

    /// The upper bound is the board's shorter side, so it is checked against `dimensions` here
    /// rather than fixed in the type.
    pub(crate) fn new(win_count: u32, dimensions: Dimensions) -> Result<Self, OutOfRange> {
        let win_count = OutOfRange::check(win_count, WinCount::MIN, dimensions.shorter_side())?;
        Ok(Self(WinCount::new(win_count)?))
    }

    /// Only for defaults, next to a compile-time check that `VALUE` fits their board.
    pub(crate) const fn constant<const VALUE: u32>() -> Self {
        Self(WinCount::constant::<VALUE>())
    }

    pub(crate) fn range(shorter_side: FieldSize) -> SettingRange {
        SettingRange::new(Self::FIELD, WinCount::MIN, shorter_side.get())
    }

    pub(crate) fn get(self) -> u32 {
        self.0.get()
    }

    pub(crate) fn cells(self) -> usize {
        usize::from(self.0.to_u8())
    }

    /// Steps from a cell to the farthest cell that can share a winning window with it.
    pub(crate) fn reach(self) -> i32 {
        i32::from(self.0.to_u8()) - 1
    }

    /// Cells of a line that can share a winning window with its middle cell: `2 * reach + 1`.
    pub(crate) fn segment_cells(self) -> usize {
        2 * usize::from(self.0.to_u8()) - 1
    }
}
