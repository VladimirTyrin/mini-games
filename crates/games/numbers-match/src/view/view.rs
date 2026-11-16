use crate::model::{Cell, Hint, HintsLeft};
use crate::rules::Phase;
use crate::settings::HintMode;
use crate::view::{LastAction, Stats};

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
