use super::board::Board;
use super::types::{
    GameEvent, GameStatus, HintMode, HintResult, Position,
    HINT_BONUS_PER_REFILL, INITIAL_HINTS_LIMITED, INITIAL_REFILLS,
};
use crate::games::session_rng::SessionRng;
use crate::proto::numbers_match as proto;

pub struct NumbersMatchGameState {
    board: Board,
    hint_mode: HintMode,
    refills_remaining: u32,
    hints_remaining: Option<u32>,
    hints_used: u32,
    pairs_removed: u32,
    refills_used: u32,
    status: GameStatus,
    current_hint: Option<HintResult>,
    pending_events: Vec<GameEvent>,
}

impl NumbersMatchGameState {
    pub fn new(rng: &mut SessionRng, hint_mode: HintMode) -> Self {
        let hints_remaining = match hint_mode {
            HintMode::Limited => Some(INITIAL_HINTS_LIMITED),
            HintMode::Unlimited => None,
            HintMode::Disabled => Some(0),
        };

        Self {
            board: Board::new(rng),
            hint_mode,
            refills_remaining: INITIAL_REFILLS,
            hints_remaining,
            hints_used: 0,
            pairs_removed: 0,
            refills_used: 0,
            status: GameStatus::InProgress,
            current_hint: None,
            pending_events: Vec::new(),
        }
    }

    pub fn remove_pair(&mut self, pos1: Position, pos2: Position) -> Result<(), String> {
        if self.status != GameStatus::InProgress {
            return Err("Game is not in progress".to_string());
        }

        if !self.board.can_remove_pair(pos1, pos2) {
            return Err("Cannot remove this pair".to_string());
        }

        if let Some(cell) = self.board.get_mut(pos1) {
            cell.removed = true;
        }
        if let Some(cell) = self.board.get_mut(pos2) {
            cell.removed = true;
        }

        self.pairs_removed += 1;
        self.current_hint = None;

        self.pending_events.push(GameEvent::PairRemoved {
            first: pos1,
            second: pos2,
        });

        let removed_rows = self.board.remove_empty_rows();
        if !removed_rows.is_empty() {
            self.pending_events.push(GameEvent::RowsDeleted {
                row_indices: removed_rows,
            });
        }

        self.check_game_over();

        Ok(())
    }

    pub fn refill(&mut self) -> Result<(), String> {
        if self.status != GameStatus::InProgress {
            return Err("Game is not in progress".to_string());
        }

        if self.refills_remaining == 0 {
            return Err("No refills remaining".to_string());
        }

        let old_row_count = self.board.row_count();
        let added_values = self.board.refill();
        let new_row_count = self.board.row_count();

        self.refills_remaining -= 1;
        self.refills_used += 1;
        self.current_hint = None;

        if self.hint_mode == HintMode::Limited
            && let Some(ref mut hints) = self.hints_remaining
        {
            *hints += HINT_BONUS_PER_REFILL;
        }

        self.pending_events.push(GameEvent::Refill {
            old_row_count,
            new_row_count,
            added_values,
        });

        self.check_game_over();

        Ok(())
    }

    pub fn request_hint(&mut self) -> Result<HintResult, String> {
        if self.status != GameStatus::InProgress {
            return Err("Game is not in progress".to_string());
        }

        if self.hint_mode == HintMode::Disabled {
            return Err("Hints are disabled".to_string());
        }

        if let Some(hints) = self.hints_remaining
            && hints == 0
        {
            return Err("No hints remaining".to_string());
        }

        let hint = self.calculate_hint();

        if self.hint_mode == HintMode::Limited
            && let Some(ref mut hints) = self.hints_remaining
        {
            *hints -= 1;
        }
        self.hints_used += 1;

        self.current_hint = Some(hint.clone());
        self.pending_events.push(GameEvent::HintShown {
            hint: hint.clone(),
        });

        if hint == HintResult::NoMoves {
            self.status = GameStatus::Lost;
        }

        Ok(hint)
    }

    fn calculate_hint(&self) -> HintResult {
        if let Some((pos1, pos2)) = self.board.find_any_valid_pair() {
            return HintResult::Pair(pos1, pos2);
        }

        if self.refills_remaining > 0 {
            return HintResult::SuggestRefill;
        }

        HintResult::NoMoves
    }

    fn check_game_over(&mut self) {
        if self.board.active_cell_count() == 0 {
            self.status = GameStatus::Won;
            return;
        }

        if self.board.find_any_valid_pair().is_some() {
            return;
        }

        if self.refills_remaining > 0 {
            return;
        }

        self.status = GameStatus::Lost;
    }

    pub fn take_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn status(&self) -> GameStatus {
        self.status
    }

    pub fn pairs_removed(&self) -> u32 {
        self.pairs_removed
    }

    pub fn refills_used(&self) -> u32 {
        self.refills_used
    }

    pub fn hints_used(&self) -> u32 {
        self.hints_used
    }

    pub fn to_proto(&self) -> proto::NumbersMatchGameState {
        let cells: Vec<proto::Cell> = self
            .board
            .cells()
            .iter()
            .map(|cell| proto::Cell {
                value: cell.value as u32,
                removed: cell.removed,
            })
            .collect();

        let events: Vec<proto::GameEvent> = self
            .pending_events
            .iter()
            .map(|e| self.event_to_proto(e))
            .collect();

        proto::NumbersMatchGameState {
            cells,
            row_count: self.board.row_count() as u32,
            refills_remaining: self.refills_remaining,
            hints_remaining: self.hints_remaining,
            hint_mode: self.hint_mode_to_proto().into(),
            status: self.status_to_proto().into(),
            events,
            current_hint: self.current_hint.as_ref().map(|h| self.hint_to_proto(h)),
        }
    }

    fn hint_mode_to_proto(&self) -> proto::HintMode {
        match self.hint_mode {
            HintMode::Limited => proto::HintMode::Limited,
            HintMode::Unlimited => proto::HintMode::Unlimited,
            HintMode::Disabled => proto::HintMode::Disabled,
        }
    }

    fn status_to_proto(&self) -> proto::GameStatus {
        match self.status {
            GameStatus::InProgress => proto::GameStatus::InProgress,
            GameStatus::Won => proto::GameStatus::Won,
            GameStatus::Lost => proto::GameStatus::Lost,
        }
    }

    fn event_to_proto(&self, event: &GameEvent) -> proto::GameEvent {
        let event_oneof = match event {
            GameEvent::PairRemoved { first, second } => {
                proto::game_event::Event::PairRemoved(proto::PairRemovedEvent {
                    first_index: first.to_index() as u32,
                    second_index: second.to_index() as u32,
                })
            }
            GameEvent::RowsDeleted { row_indices } => {
                proto::game_event::Event::RowsDeleted(proto::RowsDeletedEvent {
                    row_indices: row_indices.iter().map(|&i| i as u32).collect(),
                })
            }
            GameEvent::Refill {
                old_row_count,
                new_row_count,
                added_values,
            } => proto::game_event::Event::Refill(proto::RefillEvent {
                old_row_count: *old_row_count as u32,
                new_row_count: *new_row_count as u32,
                added_values: added_values.iter().map(|&v| v as u32).collect(),
            }),
            GameEvent::HintShown { hint } => {
                proto::game_event::Event::HintShown(proto::HintShownEvent {
                    hint: Some(self.hint_to_proto(hint)),
                })
            }
        };

        proto::GameEvent {
            event: Some(event_oneof),
        }
    }

    fn hint_to_proto(&self, hint: &HintResult) -> proto::HintResult {
        let hint_oneof = match hint {
            HintResult::Pair(pos1, pos2) => proto::hint_result::Hint::Pair(proto::PairHint {
                first_index: pos1.to_index() as u32,
                second_index: pos2.to_index() as u32,
            }),
            HintResult::SuggestRefill => {
                proto::hint_result::Hint::SuggestRefill(proto::RefillHint {})
            }
            HintResult::NoMoves => proto::hint_result::Hint::NoMoves(proto::NoMovesHint {}),
        };

        proto::HintResult {
            hint: Some(hint_oneof),
        }
    }
}

pub fn position_from_index(index: u32) -> Position {
    Position::from_index(index as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state(hint_mode: HintMode) -> NumbersMatchGameState {
        let mut rng = SessionRng::new(12345);
        NumbersMatchGameState::new(&mut rng, hint_mode)
    }

    #[test]
    fn test_new_game_has_42_active_cells() {
        let state = create_test_state(HintMode::Limited);
        let proto_state = state.to_proto();

        let active_count = proto_state
            .cells
            .iter()
            .filter(|c| c.value > 0 && !c.removed)
            .count();

        assert_eq!(active_count, 42);
    }

    #[test]
    fn test_initial_refills_count() {
        let state = create_test_state(HintMode::Limited);

        assert_eq!(state.refills_remaining, INITIAL_REFILLS);
    }

    #[test]
    fn test_initial_hints_limited_mode() {
        let state = create_test_state(HintMode::Limited);

        assert_eq!(state.hints_remaining, Some(INITIAL_HINTS_LIMITED));
    }

    #[test]
    fn test_initial_hints_unlimited_mode() {
        let state = create_test_state(HintMode::Unlimited);

        assert_eq!(state.hints_remaining, None);
    }

    #[test]
    fn test_initial_hints_disabled_mode() {
        let state = create_test_state(HintMode::Disabled);

        assert_eq!(state.hints_remaining, Some(0));
    }

    #[test]
    fn test_request_hint_disabled_returns_error() {
        let mut state = create_test_state(HintMode::Disabled);

        let result = state.request_hint();

        assert!(result.is_err());
    }

    #[test]
    fn test_refill_decrements_counter() {
        let mut state = create_test_state(HintMode::Limited);
        let initial = state.refills_remaining;

        state.refill().unwrap();

        assert_eq!(state.refills_remaining, initial - 1);
    }

    #[test]
    fn test_refill_adds_hint_in_limited_mode() {
        let mut state = create_test_state(HintMode::Limited);
        let initial_hints = state.hints_remaining.unwrap();

        state.refill().unwrap();

        assert_eq!(
            state.hints_remaining.unwrap(),
            initial_hints + HINT_BONUS_PER_REFILL
        );
    }

    #[test]
    fn test_refill_fails_when_zero_remaining() {
        let mut state = create_test_state(HintMode::Limited);
        state.refills_remaining = 0;

        let result = state.refill();

        assert!(result.is_err());
    }

    #[test]
    fn test_hint_decrements_counter_limited() {
        let mut state = create_test_state(HintMode::Limited);
        let initial_hints = state.hints_remaining.unwrap();

        state.request_hint().unwrap();

        assert_eq!(state.hints_remaining.unwrap(), initial_hints - 1);
    }

    #[test] 
    fn test_hint_unlimited_never_decrements() {
        let mut state = create_test_state(HintMode::Unlimited);

        state.request_hint().unwrap();

        assert_eq!(state.hints_remaining, None);
    }

    #[test]
    fn test_pair_removed_event_generated() {
        let mut state = create_test_state(HintMode::Limited);

        if let Some((pos1, pos2)) = state.board.find_any_valid_pair() {
            state.remove_pair(pos1, pos2).unwrap();
            let events = state.take_events();

            assert!(events.iter().any(|e| matches!(e, GameEvent::PairRemoved { .. })));
        }
    }

    #[test]
    fn test_events_cleared_after_take() {
        let mut state = create_test_state(HintMode::Limited);

        if let Some((pos1, pos2)) = state.board.find_any_valid_pair() {
            state.remove_pair(pos1, pos2).unwrap();
            let _ = state.take_events();
            let events = state.take_events();

            assert!(events.is_empty());
        }
    }
}
