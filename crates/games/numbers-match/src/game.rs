use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, GameInfo, Outcome, OutcomeKind, PlayerSpec,
    SetupError, Slot, StateHasher, Status, Viewer,
};

use crate::io::{Command, WorldEvent};
use crate::model::{Batch, Board, Cell, CellPair, CellValue, Hint, HintsLeft};
use crate::rules::{LossReason, Phase};
use crate::settings::{HintMode, LIMITED_HINTS_PER_REFILL, LIMITED_INITIAL_HINTS, Settings};
use crate::view::{LastAction, Stats, View};
use crate::{COLUMNS, INITIAL_CELLS};

const PLAYER: Slot = Slot::new(0);

#[derive(Clone, Debug)]
pub struct NumbersMatch {
    settings: Settings,
    board: Board,
    phase: Phase,
    refills_left: u32,
    hints_left: HintsLeft,
    hint: Option<Hint>,
    last_action: Option<LastAction>,
    stats: Stats,
}

impl NumbersMatch {
    /// The first removable pair in reading order (`first < second`), in O(cells).
    pub fn find_pair(&self) -> Option<CellPair> {
        self.board.find_pair()
    }

    pub fn can_remove_pair(&self, pair: CellPair) -> bool {
        self.board.can_remove_pair(pair)
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn cells(&self) -> &[Cell] {
        self.board.cells()
    }

    pub fn refills_left(&self) -> u32 {
        self.refills_left
    }

    pub fn hints_left(&self) -> HintsLeft {
        self.hints_left
    }

    pub fn hint(&self) -> Option<Hint> {
        self.hint
    }

    pub fn last_action(&self) -> Option<&LastAction> {
        self.last_action.as_ref()
    }

    pub fn stats(&self) -> Stats {
        self.stats
    }

    fn check_game_over(&mut self) {
        if self.board.active_count() == 0 {
            self.phase = Phase::Won;
        } else if self.refills_left == 0 && self.board.find_pair().is_none() {
            self.phase = Phase::Lost(LossReason::NoMoves);
        }
    }

    fn remove_pair(&mut self, pair: CellPair) -> Result<(), CommandError> {
        if !self.board.can_remove_pair(pair) {
            return Err(CommandError::Illegal("these cells cannot be removed"));
        }
        self.board.remove_pair(pair);
        let deleted_rows = self.board.delete_inactive_rows();
        self.stats.pairs_removed = self.stats.pairs_removed.saturating_add(1);
        self.hint = None;
        self.last_action = Some(LastAction::PairRemoved { pair, deleted_rows });
        self.check_game_over();
        Ok(())
    }

    fn refill(&mut self) -> Result<(), CommandError> {
        if self.refills_left == 0 {
            return Err(CommandError::Illegal("no refills left"));
        }
        self.refills_left -= 1;
        self.stats.refills_used = self.stats.refills_used.saturating_add(1);
        let (start, values) = self.board.refill(Batch::refill(self.stats.refills_used));
        if self.settings.hint_mode == HintMode::Limited
            && let HintsLeft::Count(count) = &mut self.hints_left
        {
            *count = count.saturating_add(LIMITED_HINTS_PER_REFILL);
        }
        self.hint = None;
        self.last_action = Some(LastAction::Refilled { start, values });
        self.check_game_over();
        Ok(())
    }

    fn request_hint(&mut self) -> Result<(), CommandError> {
        if self.settings.hint_mode == HintMode::Disabled {
            return Err(CommandError::Illegal("hints are disabled"));
        }
        if self.hints_left == HintsLeft::Count(0) {
            return Err(CommandError::Illegal("no hints left"));
        }
        if self.hint.is_some() {
            return Err(CommandError::Illegal("a hint is already shown"));
        }
        // While playing, "no pair" implies a refill is left, otherwise the game would be lost.
        let hint = self
            .board
            .find_pair()
            .map_or(Hint::SuggestRefill, Hint::Pair);
        if let HintsLeft::Count(count) = &mut self.hints_left {
            *count -= 1;
        }
        self.stats.hints_used = self.stats.hints_used.saturating_add(1);
        self.hint = Some(hint);
        self.last_action = Some(LastAction::HintShown);
        Ok(())
    }

    /// A game in `Playing` (or already over) on an arbitrary board.
    #[cfg(test)]
    pub(crate) fn with_board(settings: Settings, board: Board) -> Self {
        let mut game = Self::new(&settings, &[PlayerSpec::HUMAN]).unwrap();
        game.board = board;
        game.phase = Phase::Playing;
        game.check_game_over();
        game
    }

    #[cfg(test)]
    pub(crate) fn board(&self) -> &Board {
        &self.board
    }

    #[cfg(test)]
    pub(crate) fn set_refills_left(&mut self, refills: u32) {
        self.refills_left = refills;
    }
}

/// Same distribution as legacy's rejection loop (uniform over the digits that differ from the
/// left neighbour), without the unbounded loop.
fn deal(rng: &mut CoreRng) -> Vec<CellValue> {
    let mut values: Vec<CellValue> = Vec::with_capacity(INITIAL_CELLS);
    for index in 0..INITIAL_CELLS {
        let left_neighbour = values.last().filter(|_| index % COLUMNS != 0);
        let value = match left_neighbour {
            None => rng.range_inclusive(1, 9) as u8,
            Some(left) => {
                let value = rng.range_inclusive(1, 8) as u8;
                if value >= left.get() {
                    value + 1
                } else {
                    value
                }
            }
        };
        values.push(CellValue::clamped(value));
    }
    values
}

fn check_deal(values: &[CellValue]) -> Result<(), CommandError> {
    if values.len() != INITIAL_CELLS {
        return Err(CommandError::Illegal("the deal must have 42 values"));
    }
    let has_adjacent_duplicate = values
        .chunks(COLUMNS)
        .any(|row| row.windows(2).any(|pair| pair.first() == pair.get(1)));
    if has_adjacent_duplicate {
        return Err(CommandError::Illegal("dealt row has equal adjacent values"));
    }
    Ok(())
}

impl Game for NumbersMatch {
    type Settings = Settings;
    type Command = Command;
    type WorldEvent = WorldEvent;
    type View = View;

    const INFO: GameInfo = GameInfo {
        id: "numbers_match",
        name: "Numbers Match",
        min_players: 1,
        max_players: 1,
        has_bots: false,
        cooperative: true,
    };

    fn new(settings: &Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        settings.validate()?;
        SetupError::check_player_count(
            players.len(),
            Self::INFO.min_players,
            Self::INFO.max_players,
        )?;
        if !players.iter().all(|player| player.is_human()) {
            return Err(SetupError::InvalidPlayers(
                "Numbers Match has no bots".into(),
            ));
        }
        Ok(Self {
            settings: *settings,
            board: Board::default(),
            phase: Phase::Dealing,
            refills_left: settings.refills,
            hints_left: match settings.hint_mode {
                HintMode::Limited => HintsLeft::Count(LIMITED_INITIAL_HINTS),
                HintMode::Unlimited => HintsLeft::Unlimited,
                HintMode::Disabled => HintsLeft::Count(0),
            },
            hint: None,
            last_action: None,
            stats: Stats::default(),
        })
    }

    fn clock(&self) -> Clock {
        Clock::TurnBased
    }

    fn apply_command(&mut self, slot: Slot, command: &Command) -> Result<(), CommandError> {
        if slot != PLAYER {
            return Err(CommandError::NotPlaying);
        }
        match self.phase {
            Phase::Dealing => return Err(CommandError::Illegal("the board is not dealt yet")),
            Phase::Won | Phase::Lost(_) => return Err(CommandError::GameOver),
            Phase::Playing => {}
        }
        match *command {
            Command::RemovePair(pair) => self.remove_pair(pair),
            Command::Refill => self.refill(),
            Command::RequestHint => self.request_hint(),
        }
    }

    fn roll_world(&self, rng: &mut CoreRng) -> Vec<WorldEvent> {
        if self.phase == Phase::Dealing {
            vec![WorldEvent::BoardDealt { values: deal(rng) }]
        } else {
            Vec::new()
        }
    }

    fn apply_world(&mut self, event: &WorldEvent) -> Result<(), CommandError> {
        match event {
            WorldEvent::BoardDealt { values } => {
                if self.phase != Phase::Dealing {
                    return Err(CommandError::Illegal("the board is already dealt"));
                }
                check_deal(values)?;
                self.board = Board::dealt(values);
                self.phase = Phase::Playing;
                // A deal without pairs and zero refills is lost immediately.
                self.check_game_over();
                Ok(())
            }
        }
    }

    fn drop_player(&mut self, slot: Slot) {
        if slot == PLAYER && !self.phase.is_over() {
            self.phase = Phase::Lost(LossReason::Dropped);
        }
    }

    /// The player's turn even while `Dealing`: the authority deals in the very first frame.
    fn awaiting(&self) -> Awaiting {
        if self.phase.is_over() {
            Awaiting::Nobody
        } else {
            Awaiting::Turn(PLAYER)
        }
    }

    /// Legacy score: 1 for a cleared board, 0 otherwise.
    fn status(&self) -> Status {
        let kind = match self.phase {
            Phase::Dealing | Phase::Playing => return Status::InProgress,
            Phase::Won => OutcomeKind::Victory,
            Phase::Lost(_) => OutcomeKind::Defeat,
        };
        Status::Finished(Outcome {
            kind,
            winners: Vec::new(),
            scores: vec![i64::from(kind == OutcomeKind::Victory)],
        })
    }

    fn view(&self, _viewer: Viewer) -> View {
        // `Game::View` is owned, so the board is copied once per render.
        View {
            cells: self.board.cells().to_vec(),
            batches: self.board.batches().to_vec(),
            rows: u32::try_from(self.board.rows()).unwrap_or(u32::MAX),
            phase: self.phase,
            hint_mode: self.settings.hint_mode,
            hints_left: self.hints_left,
            refills_left: self.refills_left,
            hint: self.hint,
            last_action: self.last_action.clone(),
            stats: self.stats,
        }
    }

    fn hash_state(&self, hasher: &mut StateHasher) {
        hasher.u8(match self.settings.hint_mode {
            HintMode::Limited => 0,
            HintMode::Unlimited => 1,
            HintMode::Disabled => 2,
        });
        hasher.u32(self.settings.refills);
        // Batches only colour the cells; leaving them out keeps the hash about the rules.
        hasher.length(self.board.cells().len());
        for cell in self.board.cells() {
            hasher.u8(match *cell {
                Cell::Empty => 0,
                Cell::Active(value) => value.get(),
                Cell::Removed(value) => 0x10 | value.get(),
            });
        }
        hasher.u8(match self.phase {
            Phase::Dealing => 0,
            Phase::Playing => 1,
            Phase::Won => 2,
            Phase::Lost(LossReason::NoMoves) => 3,
            Phase::Lost(LossReason::Dropped) => 4,
        });
        hasher.u32(self.refills_left);
        match self.hints_left {
            HintsLeft::Unlimited => hasher.u8(0),
            HintsLeft::Count(count) => {
                hasher.u8(1);
                hasher.u32(count);
            }
        }
        match self.hint {
            None => hasher.u8(0),
            Some(Hint::Pair(pair)) => {
                hasher.u8(1);
                hash_pair(hasher, pair);
            }
            Some(Hint::SuggestRefill) => hasher.u8(2),
        }
        match &self.last_action {
            None => hasher.u8(0),
            Some(LastAction::PairRemoved { pair, deleted_rows }) => {
                hasher.u8(1);
                hash_pair(hasher, *pair);
                hasher.length(deleted_rows.len());
                for row in deleted_rows {
                    hasher.u32(row.get());
                }
            }
            Some(LastAction::Refilled { start, values }) => {
                hasher.u8(2);
                hasher.u32(start.get());
                hasher.length(values.len());
                for value in values {
                    hasher.u8(value.get());
                }
            }
            Some(LastAction::HintShown) => hasher.u8(3),
        }
        hasher.u32(self.stats.pairs_removed);
        hasher.u32(self.stats.refills_used);
        hasher.u32(self.stats.hints_used);
    }
}

fn hash_pair(hasher: &mut StateHasher, pair: CellPair) {
    hasher.u32(pair.first.get());
    hasher.u32(pair.second.get());
}
