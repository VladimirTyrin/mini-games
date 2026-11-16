use crate::cell::Cell;
use crate::hint::Hint;
use crate::hint_mode::HintMode;
use crate::hints_left::HintsLeft;
use crate::last_action::LastAction;
use crate::phase::Phase;
use crate::stats::Stats;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    /// Reading order; `cells.len() == rows * COLUMNS`.
    pub cells: Vec<Cell>,
    pub rows: u32,
    pub phase: Phase,
    pub hint_mode: HintMode,
    pub hints_left: HintsLeft,
    pub refills_left: u32,
    pub hint: Option<Hint>,
    pub last_action: Option<LastAction>,
    pub stats: Stats,
}
