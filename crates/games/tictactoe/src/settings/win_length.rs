use mg_core::SetupError;

use crate::settings::{Dimensions, MIN_WIN_COUNT};

/// Validated `win_count`: at least `MIN_WIN_COUNT` and at most the board's shorter side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WinLength(u8);

impl WinLength {
    pub(crate) fn new(win_count: u32, dimensions: Dimensions) -> Result<Self, SetupError> {
        SetupError::check_range(
            "win_count",
            win_count,
            MIN_WIN_COUNT,
            dimensions.shorter_side(),
        )?;
        u8::try_from(win_count)
            .map(Self)
            .map_err(|error| SetupError::setting("win_count", error.to_string()))
    }

    pub(crate) fn get(self) -> u32 {
        u32::from(self.0)
    }

    pub(crate) fn cells(self) -> usize {
        usize::from(self.0)
    }

    /// Steps from a cell to the farthest cell that can share a winning window with it.
    pub(crate) fn reach(self) -> i32 {
        i32::from(self.0) - 1
    }

    /// Cells of a line that can share a winning window with its middle cell: `2 * reach + 1`.
    pub(crate) fn segment_cells(self) -> usize {
        2 * usize::from(self.0) - 1
    }
}
