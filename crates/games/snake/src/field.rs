use crate::direction::Direction;
use crate::point::Point;
use crate::wall_collision_mode::WallCollisionMode;

/// Validated board geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Field {
    width: u16,
    height: u16,
    wall_mode: WallCollisionMode,
}

impl Field {
    /// Callers pass sizes already validated against the settings limits (both > 0).
    pub(crate) fn new(width: u16, height: u16, wall_mode: WallCollisionMode) -> Self {
        Self {
            width,
            height,
            wall_mode,
        }
    }

    pub(crate) fn width(self) -> u16 {
        self.width
    }

    pub(crate) fn height(self) -> u16 {
        self.height
    }

    pub(crate) fn wall_mode(self) -> WallCollisionMode {
        self.wall_mode
    }

    pub(crate) fn cell_count(self) -> usize {
        usize::from(self.width) * usize::from(self.height)
    }

    pub(crate) fn contains(self, point: Point) -> bool {
        point.x < self.width && point.y < self.height
    }

    /// Row-major index, `None` outside the field.
    pub(crate) fn index_of(self, point: Point) -> Option<usize> {
        self.contains(point)
            .then(|| usize::from(point.y) * usize::from(self.width) + usize::from(point.x))
    }

    pub(crate) fn point_at(self, index: usize) -> Option<Point> {
        let width = usize::from(self.width);
        let x = u16::try_from(index % width).ok()?;
        let y = u16::try_from(index / width).ok()?;
        let point = Point::new(x, y);
        self.contains(point).then_some(point)
    }

    /// The cell one step away; `None` when that step hits a wall in `Death` mode.
    pub(crate) fn neighbour(self, from: Point, direction: Direction) -> Option<Point> {
        let Point { x, y } = from;
        let last_column = self.width - 1;
        let last_row = self.height - 1;
        let wraps = self.wall_mode == WallCollisionMode::WrapAround;
        let (x, y) = match direction {
            Direction::Up if y > 0 => (x, y - 1),
            Direction::Up if wraps => (x, last_row),
            Direction::Down if y < last_row => (x, y + 1),
            Direction::Down if wraps => (x, 0),
            Direction::Left if x > 0 => (x - 1, y),
            Direction::Left if wraps => (last_column, y),
            Direction::Right if x < last_column => (x + 1, y),
            Direction::Right if wraps => (0, y),
            _ => return None,
        };
        Some(Point::new(x, y))
    }

    /// Manhattan distance, taking the shorter way around in `WrapAround` mode.
    pub(crate) fn distance(self, from: Point, to: Point) -> u32 {
        let along = |a: u16, b: u16, size: u16| {
            let direct = a.abs_diff(b);
            let distance = match self.wall_mode {
                WallCollisionMode::Death => direct,
                WallCollisionMode::WrapAround => direct.min(size.saturating_sub(direct)),
            };
            u32::from(distance)
        };
        along(from.x, to.x, self.width) + along(from.y, to.y, self.height)
    }
}
