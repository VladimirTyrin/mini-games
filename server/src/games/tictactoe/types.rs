#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Empty,
    X,
    O,
}

impl Mark {
    pub fn to_proto(self) -> i32 {
        match self {
            Mark::Empty => 1,
            Mark::X => 2,
            Mark::O => 3,
        }
    }

    pub fn opponent(&self) -> Option<Mark> {
        match self {
            Mark::X => Some(Mark::O),
            Mark::O => Some(Mark::X),
            Mark::Empty => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStatus {
    InProgress,
    XWon,
    OWon,
    Draw,
}

impl GameStatus {
    pub fn to_proto(self) -> i32 {
        match self {
            GameStatus::InProgress => 1,
            GameStatus::XWon => 2,
            GameStatus::OWon => 3,
            GameStatus::Draw => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstPlayerMode {
    Random,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub fn to_proto(self) -> crate::proto::tictactoe::Position {
        crate::proto::tictactoe::Position {
            x: self.x as u32,
            y: self.y as u32,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WinningLine {
    pub mark: Mark,
    pub start: Position,
    pub end: Position,
}

impl WinningLine {
    pub fn new(mark: Mark, start: Position, end: Position) -> Self {
        Self { mark, start, end }
    }

    pub fn to_proto(self) -> crate::proto::tictactoe::WinningLine {
        crate::proto::tictactoe::WinningLine {
            start_x: self.start.x as u32,
            start_y: self.start.y as u32,
            end_x: self.end.x as u32,
            end_y: self.end.y as u32,
        }
    }
}
