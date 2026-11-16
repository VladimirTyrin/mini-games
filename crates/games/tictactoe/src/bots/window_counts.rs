use super::stone::Stone;

/// Stones inside one `win_count`-long window of a line.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WindowCounts {
    pub(crate) bot: usize,
    pub(crate) opponent: usize,
    pub(crate) off_board: usize,
}

impl WindowCounts {
    pub(crate) fn add(&mut self, stone: Stone) {
        match stone {
            Stone::Empty => {}
            Stone::Bot => self.bot += 1,
            Stone::Opponent => self.opponent += 1,
            Stone::OffBoard => self.off_board += 1,
        }
    }

    pub(crate) fn remove(&mut self, stone: Stone) {
        match stone {
            Stone::Empty => {}
            Stone::Bot => self.bot -= 1,
            Stone::Opponent => self.opponent -= 1,
            Stone::OffBoard => self.off_board -= 1,
        }
    }
}
