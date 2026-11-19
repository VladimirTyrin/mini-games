use mg_tictactoe::{
    BotKind, Command, FirstPlayerMode, GameEnd, Mark, PLAYERS, Point, RawSettings, Refusal,
    TicTacToe, View, WinningLine, WorldEvent,
};

use crate::frame_codec::{slot_from_wire, slot_to_wire};
use crate::proto;
use crate::proto::{TicTacToeBotKind, TicTacToeEndReason, TicTacToeFirstPlayerMode, TicTacToeMark};
use crate::wire_error::WireError;
use crate::wire_game::{WireBots, WireGame};
use crate::{WireRefusal, malformed_refusal};

impl WireGame for TicTacToe {
    fn settings_from_wire(settings: &proto::GameSettings) -> Result<RawSettings, WireError> {
        let settings = match settings.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::game_settings::Game::Tictactoe(settings) => settings,
            _ => return Err(WireError::WrongGame),
        };
        let first_player = match TicTacToeFirstPlayerMode::try_from(settings.first_player) {
            Ok(TicTacToeFirstPlayerMode::Random) => FirstPlayerMode::Random,
            Ok(TicTacToeFirstPlayerMode::Host) => FirstPlayerMode::Host,
            Ok(TicTacToeFirstPlayerMode::Guest) => FirstPlayerMode::Guest,
            Ok(TicTacToeFirstPlayerMode::Unspecified) => {
                return Err(WireError::Missing("first_player"));
            }
            Err(_) => return Err(WireError::UnknownEnumValue("first_player")),
        };
        Ok(RawSettings {
            field_width: settings.field_width,
            field_height: settings.field_height,
            win_count: settings.win_count,
            first_player,
        })
    }

    fn settings_to_wire(settings: &RawSettings) -> proto::GameSettings {
        let first_player = match settings.first_player {
            FirstPlayerMode::Random => TicTacToeFirstPlayerMode::Random,
            FirstPlayerMode::Host => TicTacToeFirstPlayerMode::Host,
            FirstPlayerMode::Guest => TicTacToeFirstPlayerMode::Guest,
        };
        proto::GameSettings {
            game: Some(proto::game_settings::Game::Tictactoe(
                proto::TicTacToeSettings {
                    field_width: settings.field_width,
                    field_height: settings.field_height,
                    win_count: settings.win_count,
                    first_player: first_player.into(),
                },
            )),
        }
    }

    fn command_from_wire(command: &proto::GameCommand) -> Result<Command, WireError> {
        let command = match command.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::game_command::Game::Tictactoe(command) => command,
            _ => return Err(WireError::WrongGame),
        };
        match command
            .command
            .as_ref()
            .ok_or(WireError::Missing("command"))?
        {
            proto::tic_tac_toe_command::Command::Place(point) => {
                Ok(Command::Place(point_from_wire(point)))
            }
        }
    }

    fn command_to_wire(command: &Command) -> proto::GameCommand {
        let Command::Place(point) = *command;
        proto::GameCommand {
            game: Some(proto::game_command::Game::Tictactoe(
                proto::TicTacToeCommand {
                    command: Some(proto::tic_tac_toe_command::Command::Place(point_to_wire(
                        point,
                    ))),
                },
            )),
        }
    }

    fn world_event_from_wire(event: &proto::GameWorldEvent) -> Result<WorldEvent, WireError> {
        let event = match event.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::game_world_event::Game::Tictactoe(event) => event,
            _ => return Err(WireError::WrongGame),
        };
        match event.event.as_ref().ok_or(WireError::Missing("event"))? {
            proto::tic_tac_toe_world_event::Event::FirstPlayerChosen(chosen) => {
                let x_player = slot_from_wire(chosen.x_slot)?;
                if x_player.get() >= PLAYERS {
                    return Err(WireError::OutOfRange("x_slot"));
                }
                Ok(WorldEvent::FirstPlayerChosen { x_player })
            }
        }
    }

    fn world_event_to_wire(event: &WorldEvent) -> proto::GameWorldEvent {
        let WorldEvent::FirstPlayerChosen { x_player } = *event;
        proto::GameWorldEvent {
            game: Some(proto::game_world_event::Game::Tictactoe(
                proto::TicTacToeWorldEvent {
                    event: Some(proto::tic_tac_toe_world_event::Event::FirstPlayerChosen(
                        proto::TicTacToeFirstPlayerChosen {
                            x_slot: slot_to_wire(x_player),
                        },
                    )),
                },
            )),
        }
    }

    fn view_to_wire(view: &View) -> proto::GameView {
        proto::GameView {
            game: Some(proto::game_view::Game::Tictactoe(proto::TicTacToeView {
                field_width: view.field_width,
                field_height: view.field_height,
                win_count: view.win_count,
                cells: view
                    .cells
                    .iter()
                    .map(|cell| cell.map_or(TicTacToeMark::Unspecified, mark_to_wire).into())
                    .collect(),
                x_slot: view.x_player.map(slot_to_wire),
                o_slot: view.o_player.map(slot_to_wire),
                current_slot: view.current_player.map(slot_to_wire),
                last_move: view.last_move.map(point_to_wire),
                end: view.end.map(game_end_to_wire),
                you: view.you.map(slot_to_wire),
            })),
        }
    }
}

impl WireBots for TicTacToe {
    fn bot_kind_from_wire(kind: &proto::BotKind) -> Result<BotKind, WireError> {
        let kind = match kind.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::bot_kind::Game::Tictactoe(kind) => *kind,
            _ => return Err(WireError::WrongGame),
        };
        match TicTacToeBotKind::try_from(kind) {
            Ok(TicTacToeBotKind::Minimax) => Ok(BotKind::Minimax),
            Ok(TicTacToeBotKind::Random) => Ok(BotKind::Random),
            Ok(TicTacToeBotKind::Unspecified) => Err(WireError::Missing("bot_kind")),
            Err(_) => Err(WireError::UnknownEnumValue("bot_kind")),
        }
    }

    fn bot_kind_to_wire(kind: BotKind) -> proto::BotKind {
        let kind = match kind {
            BotKind::Minimax => TicTacToeBotKind::Minimax,
            BotKind::Random => TicTacToeBotKind::Random,
        };
        proto::BotKind {
            game: Some(proto::bot_kind::Game::Tictactoe(kind.into())),
        }
    }
}

/// Coordinates stay unchecked here: whether they are on the board is a game rule.
fn point_from_wire(point: &proto::TicTacToePoint) -> Point {
    Point::new(point.x, point.y)
}

fn point_to_wire(point: Point) -> proto::TicTacToePoint {
    proto::TicTacToePoint {
        x: point.x,
        y: point.y,
    }
}

fn mark_to_wire(mark: Mark) -> TicTacToeMark {
    match mark {
        Mark::X => TicTacToeMark::X,
        Mark::O => TicTacToeMark::O,
    }
}

fn game_end_to_wire(end: GameEnd) -> proto::TicTacToeGameEnd {
    let (reason, winning_line) = match end {
        GameEnd::Win { line, .. } => (TicTacToeEndReason::Win, Some(winning_line_to_wire(line))),
        GameEnd::Draw => (TicTacToeEndReason::Draw, None),
        GameEnd::OpponentLeft { .. } => (TicTacToeEndReason::PlayerDisconnected, None),
    };
    proto::TicTacToeGameEnd {
        reason: reason.into(),
        winner_slot: end.winner().map(slot_to_wire),
        winning_line,
    }
}

fn winning_line_to_wire(line: WinningLine) -> proto::TicTacToeWinningLine {
    proto::TicTacToeWinningLine {
        mark: mark_to_wire(line.mark).into(),
        start: Some(point_to_wire(line.start)),
        end: Some(point_to_wire(line.end)),
    }
}

impl WireRefusal for Refusal {
    fn refusal_to_wire(self) -> proto::refusal::Reason {
        let refusal = match self {
            Refusal::CellTaken => proto::TicTacToeRefusal::CellTaken,
            Refusal::FirstPlayerAlreadyKnown | Refusal::NoSuchSeat | Refusal::CellOutsideBoard => {
                return malformed_refusal();
            }
        };
        proto::refusal::Reason::Tictactoe(refusal.into())
    }

    fn refusal_from_wire(value: i32) -> Option<Self> {
        Some(match proto::TicTacToeRefusal::try_from(value).ok()? {
            proto::TicTacToeRefusal::Unspecified => return None,
            proto::TicTacToeRefusal::CellTaken => Refusal::CellTaken,
        })
    }
}

#[cfg(test)]
#[path = "tictactoe_tests.rs"]
mod tests;
