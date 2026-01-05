use super::game_state::Mark;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub fn to_proto(&self) -> common::proto::tictactoe::Position {
        common::proto::tictactoe::Position {
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

    pub fn to_proto(&self) -> common::proto::tictactoe::WinningLine {
        common::proto::tictactoe::WinningLine {
            start_x: self.start.x as u32,
            start_y: self.start.y as u32,
            end_x: self.end.x as u32,
            end_y: self.end.y as u32,
        }
    }
}
