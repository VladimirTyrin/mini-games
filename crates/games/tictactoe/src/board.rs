use mg_core::CommandError;

use crate::line_direction::LineDirection;
use crate::mark::Mark;
use crate::point::Point;
use crate::winning_line::WinningLine;

/// Row-major grid. Dimensions are validated by [`crate::Settings::validate`] before construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Board {
    width: u32,
    height: u32,
    cells: Vec<Option<Mark>>,
}

impl Board {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            cells: vec![None; width as usize * height as usize],
        }
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn cells(&self) -> &[Option<Mark>] {
        &self.cells
    }

    pub(crate) fn cell_count(&self) -> usize {
        self.cells.len()
    }

    fn index(&self, point: Point) -> Option<usize> {
        (point.x < self.width && point.y < self.height)
            .then(|| point.y as usize * self.width as usize + point.x as usize)
    }

    pub(crate) fn mark_at(&self, point: Point) -> Option<Mark> {
        self.index(point)
            .and_then(|index| self.cells.get(index).copied().flatten())
    }

    pub(crate) fn place(&mut self, point: Point, mark: Mark) -> Result<(), CommandError> {
        let cell = self
            .index(point)
            .and_then(|index| self.cells.get_mut(index))
            .ok_or(CommandError::Illegal("cell is outside the board"))?;
        if cell.is_some() {
            return Err(CommandError::Illegal("cell is already marked"));
        }
        *cell = Some(mark);
        Ok(())
    }

    /// The first run (in [`LineDirection::ALL`] order) through `point` that is at least
    /// `win_count` long.
    pub(crate) fn line_through(&self, point: Point, win_count: u32) -> Option<WinningLine> {
        let mark = self.mark_at(point)?;
        LineDirection::ALL.into_iter().find_map(|direction| {
            let start = self.run_end(point, mark, direction, -1);
            let end = self.run_end(point, mark, direction, 1);
            let length = start.x.abs_diff(end.x).max(start.y.abs_diff(end.y)) + 1;
            (length >= win_count).then_some(WinningLine { mark, start, end })
        })
    }

    fn run_end(&self, from: Point, mark: Mark, direction: LineDirection, step: i32) -> Point {
        let mut end = from;
        while let Some(next) = direction.walk(end, step)
            && self.mark_at(next) == Some(mark)
        {
            end = next;
        }
        end
    }
}
