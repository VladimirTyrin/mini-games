use mg_numbers_match::{
    COLUMNS, Cell, CellIndex, CellPair, CellValue, Command, Hint, HintMode, HintsLeft, LastAction,
    LossReason, NumbersMatch, Phase, Settings, Stats, View, WorldEvent,
};

use crate::proto;
use crate::proto::{
    NumbersMatchCellState as WireCellState, NumbersMatchHintMode as WireHintMode,
    NumbersMatchLossReason as WireLossReason, NumbersMatchPhase as WirePhase,
};
use crate::wire_error::WireError;
use crate::wire_game::WireGame;

impl WireGame for NumbersMatch {
    /// The refill range is a game rule, checked by `Game::new`.
    fn settings_from_wire(settings: &proto::GameSettings) -> Result<Settings, WireError> {
        let settings = match settings.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::game_settings::Game::NumbersMatch(settings) => settings,
            _ => return Err(WireError::WrongGame),
        };
        Ok(Settings {
            hint_mode: hint_mode_from_wire(settings.hint_mode)?,
            refills: settings.refills,
        })
    }

    fn settings_to_wire(settings: &Settings) -> proto::GameSettings {
        proto::GameSettings {
            game: Some(proto::game_settings::Game::NumbersMatch(
                proto::NumbersMatchSettings {
                    hint_mode: hint_mode_to_wire(settings.hint_mode).into(),
                    refills: settings.refills,
                },
            )),
        }
    }

    /// Whether the indices are on the board is a game rule, checked when the command is applied.
    fn command_from_wire(command: &proto::GameCommand) -> Result<Command, WireError> {
        use proto::numbers_match_command::Command as WireCommand;
        let command = match command.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::game_command::Game::NumbersMatch(command) => command,
            _ => return Err(WireError::WrongGame),
        };
        match command
            .command
            .as_ref()
            .ok_or(WireError::Missing("command"))?
        {
            WireCommand::RemovePair(pair) => Ok(Command::RemovePair(pair_from_wire(pair))),
            WireCommand::Refill(_) => Ok(Command::Refill),
            WireCommand::RequestHint(_) => Ok(Command::RequestHint),
        }
    }

    fn command_to_wire(command: &Command) -> proto::GameCommand {
        use proto::numbers_match_command::Command as WireCommand;
        let command = match *command {
            Command::RemovePair(pair) => WireCommand::RemovePair(pair_to_wire(pair)),
            Command::Refill => WireCommand::Refill(proto::NumbersMatchRefillCommand {}),
            Command::RequestHint => {
                WireCommand::RequestHint(proto::NumbersMatchRequestHintCommand {})
            }
        };
        proto::GameCommand {
            game: Some(proto::game_command::Game::NumbersMatch(
                proto::NumbersMatchCommand {
                    command: Some(command),
                },
            )),
        }
    }

    /// Only the digits are checked here; the deal's size and layout are game rules.
    fn world_event_from_wire(event: &proto::GameWorldEvent) -> Result<WorldEvent, WireError> {
        use proto::numbers_match_world_event::Event as WireEvent;
        let event = match event.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::game_world_event::Game::NumbersMatch(event) => event,
            _ => return Err(WireError::WrongGame),
        };
        match event.event.as_ref().ok_or(WireError::Missing("event"))? {
            WireEvent::BoardDealt(dealt) => Ok(WorldEvent::BoardDealt {
                values: dealt
                    .values
                    .iter()
                    .map(|&value| cell_value_from_wire(value))
                    .collect::<Result<_, _>>()?,
            }),
        }
    }

    fn world_event_to_wire(event: &WorldEvent) -> proto::GameWorldEvent {
        use proto::numbers_match_world_event::Event as WireEvent;
        let event = match event {
            WorldEvent::BoardDealt { values } => {
                WireEvent::BoardDealt(proto::NumbersMatchBoardDealt {
                    values: values_to_wire(values),
                })
            }
        };
        proto::GameWorldEvent {
            game: Some(proto::game_world_event::Game::NumbersMatch(
                proto::NumbersMatchWorldEvent { event: Some(event) },
            )),
        }
    }

    fn view_to_wire(view: &View) -> proto::GameView {
        let (phase, loss_reason) = match view.phase {
            Phase::Dealing => (WirePhase::Dealing, WireLossReason::Unspecified),
            Phase::Playing => (WirePhase::Playing, WireLossReason::Unspecified),
            Phase::Won => (WirePhase::Won, WireLossReason::Unspecified),
            Phase::Lost(LossReason::NoMoves) => (WirePhase::Lost, WireLossReason::NoMoves),
            Phase::Lost(LossReason::Dropped) => (WirePhase::Lost, WireLossReason::Dropped),
        };
        proto::GameView {
            game: Some(proto::game_view::Game::NumbersMatch(
                proto::NumbersMatchView {
                    cells: view.cells.iter().map(|&cell| cell_to_wire(cell)).collect(),
                    rows: view.rows,
                    columns: COLUMNS as u32,
                    phase: phase.into(),
                    loss_reason: loss_reason.into(),
                    hint_mode: hint_mode_to_wire(view.hint_mode).into(),
                    hints_left: match view.hints_left {
                        HintsLeft::Unlimited => None,
                        HintsLeft::Count(count) => Some(count),
                    },
                    refills_left: view.refills_left,
                    hint: view.hint.map(hint_to_wire),
                    last_action: view.last_action.as_ref().map(last_action_to_wire),
                    stats: Some(stats_to_wire(view.stats)),
                },
            )),
        }
    }
}

fn hint_mode_from_wire(value: i32) -> Result<HintMode, WireError> {
    match WireHintMode::try_from(value) {
        Ok(WireHintMode::Limited) => Ok(HintMode::Limited),
        Ok(WireHintMode::Unlimited) => Ok(HintMode::Unlimited),
        Ok(WireHintMode::Disabled) => Ok(HintMode::Disabled),
        Ok(WireHintMode::Unspecified) => Err(WireError::Missing("hint_mode")),
        Err(_) => Err(WireError::UnknownEnumValue("hint_mode")),
    }
}

fn hint_mode_to_wire(mode: HintMode) -> WireHintMode {
    match mode {
        HintMode::Limited => WireHintMode::Limited,
        HintMode::Unlimited => WireHintMode::Unlimited,
        HintMode::Disabled => WireHintMode::Disabled,
    }
}

fn cell_value_from_wire(value: u32) -> Result<CellValue, WireError> {
    let value = u8::try_from(value).map_err(|source| WireError::IntegerOutOfRange {
        field: "values",
        source,
    })?;
    CellValue::new(value).ok_or(WireError::OutOfRange("values"))
}

fn values_to_wire(values: &[CellValue]) -> Vec<u32> {
    values.iter().map(|value| u32::from(value.get())).collect()
}

fn pair_from_wire(pair: &proto::NumbersMatchCellPair) -> CellPair {
    CellPair::new(
        CellIndex::new(pair.first_index),
        CellIndex::new(pair.second_index),
    )
}

fn pair_to_wire(pair: CellPair) -> proto::NumbersMatchCellPair {
    proto::NumbersMatchCellPair {
        first_index: pair.first.get(),
        second_index: pair.second.get(),
    }
}

fn cell_to_wire(cell: Cell) -> proto::NumbersMatchCell {
    let (value, state) = match cell {
        Cell::Empty => (0, WireCellState::Empty),
        Cell::Active(value) => (value.get(), WireCellState::Active),
        Cell::Removed(value) => (value.get(), WireCellState::Removed),
    };
    proto::NumbersMatchCell {
        value: u32::from(value),
        state: state.into(),
    }
}

fn hint_to_wire(hint: Hint) -> proto::NumbersMatchHint {
    use proto::numbers_match_hint::Hint as WireHint;
    let hint = match hint {
        Hint::Pair(pair) => WireHint::Pair(pair_to_wire(pair)),
        Hint::SuggestRefill => WireHint::SuggestRefill(proto::NumbersMatchSuggestRefillHint {}),
    };
    proto::NumbersMatchHint { hint: Some(hint) }
}

fn last_action_to_wire(action: &LastAction) -> proto::NumbersMatchLastAction {
    use proto::numbers_match_last_action::Action as WireAction;
    let action = match action {
        LastAction::PairRemoved { pair, deleted_rows } => {
            WireAction::PairRemoved(proto::NumbersMatchPairRemoved {
                pair: Some(pair_to_wire(*pair)),
                deleted_rows: deleted_rows.iter().map(|row| row.get()).collect(),
            })
        }
        LastAction::Refilled { start, values } => {
            WireAction::Refilled(proto::NumbersMatchRefilled {
                start_index: start.get(),
                values: values_to_wire(values),
            })
        }
        LastAction::HintShown => WireAction::HintShown(proto::NumbersMatchHintShown {}),
    };
    proto::NumbersMatchLastAction {
        action: Some(action),
    }
}

fn stats_to_wire(stats: Stats) -> proto::NumbersMatchStats {
    proto::NumbersMatchStats {
        pairs_removed: stats.pairs_removed,
        refills_used: stats.refills_used,
        hints_used: stats.hints_used,
    }
}

#[cfg(test)]
#[path = "numbers_match_tests.rs"]
mod tests;
