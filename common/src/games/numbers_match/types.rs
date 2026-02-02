pub const FIELD_WIDTH: usize = 9;
pub const INITIAL_CELLS: usize = 42;
pub const INITIAL_REFILLS: u32 = 3;
pub const INITIAL_HINTS_LIMITED: u32 = 3;
pub const HINT_BONUS_PER_REFILL: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HintMode {
    Limited,
    Unlimited,
    Disabled,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameStatus {
    InProgress,
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Cell {
    pub value: u8,
    pub removed: bool,
}

impl Cell {
    pub fn new(value: u8) -> Self {
        Self {
            value,
            removed: false,
        }
    }

    pub fn empty() -> Self {
        Self {
            value: 0,
            removed: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.value > 0 && !self.removed
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    pub fn to_index(self) -> usize {
        self.row * FIELD_WIDTH + self.col
    }

    pub fn from_index(index: usize) -> Self {
        Self {
            row: index / FIELD_WIDTH,
            col: index % FIELD_WIDTH,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HintResult {
    Pair(Position, Position),
    SuggestRefill,
    NoMoves,
}

#[derive(Clone, Debug)]
pub enum GameEvent {
    PairRemoved {
        first: Position,
        second: Position,
    },
    RowsDeleted {
        row_indices: Vec<usize>,
    },
    Refill {
        old_row_count: usize,
        new_row_count: usize,
        added_values: Vec<u8>,
    },
    HintShown {
        hint: HintResult,
    },
}
