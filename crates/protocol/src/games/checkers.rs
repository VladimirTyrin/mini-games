use mg_checkers::{
    BotKind, CaptureRule, Checkers, Command, DrawReason, GameEnd, KING_MOVES_FOR_DRAW, LegalMove,
    Path, PlacedPiece, RawSettings, Refusal, Side, Square, View, WhitePlayer, WinReason,
    WorldEvent,
};

use crate::proto::{
    CheckersBotKind, CheckersCaptureRule, CheckersEndReason, CheckersSide, CheckersWhitePlayer,
    checkers_command, checkers_world_event, game_command, game_settings, game_view,
    game_world_event,
};
use crate::{WireBots, WireError, WireGame, proto, slot_from_wire, slot_to_wire};
use crate::{WireRefusal, malformed_refusal};

impl WireGame for Checkers {
    /// Shape only: the game's `Settings::parse` checks the values.
    fn settings_from_wire(settings: &proto::GameSettings) -> Result<RawSettings, WireError> {
        let game_settings::Game::Checkers(settings) =
            settings.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        let white_player = match CheckersWhitePlayer::try_from(settings.white_player) {
            Ok(CheckersWhitePlayer::Random) => WhitePlayer::Random,
            Ok(CheckersWhitePlayer::Host) => WhitePlayer::Host,
            Ok(CheckersWhitePlayer::Guest) => WhitePlayer::Guest,
            Ok(CheckersWhitePlayer::Unspecified) => {
                return Err(WireError::Missing("white_player"));
            }
            Err(_) => return Err(WireError::UnknownEnumValue("white_player")),
        };
        let capture_rule = match CheckersCaptureRule::try_from(settings.capture_rule) {
            // Unset in settings and replays from before the rule existed.
            Ok(CheckersCaptureRule::Unspecified | CheckersCaptureRule::ToTheEnd) => {
                CaptureRule::ToTheEnd
            }
            Ok(CheckersCaptureRule::MayStop) => CaptureRule::MayStop,
            Err(_) => return Err(WireError::UnknownEnumValue("capture_rule")),
        };
        Ok(RawSettings {
            white_player,
            capture_rule,
        })
    }

    fn settings_to_wire(settings: &RawSettings) -> proto::GameSettings {
        let white_player = match settings.white_player {
            WhitePlayer::Random => CheckersWhitePlayer::Random,
            WhitePlayer::Host => CheckersWhitePlayer::Host,
            WhitePlayer::Guest => CheckersWhitePlayer::Guest,
        };
        let capture_rule = match settings.capture_rule {
            CaptureRule::ToTheEnd => CheckersCaptureRule::ToTheEnd,
            CaptureRule::MayStop => CheckersCaptureRule::MayStop,
        };
        proto::GameSettings {
            game: Some(game_settings::Game::Checkers(proto::CheckersSettings {
                white_player: white_player.into(),
                capture_rule: capture_rule.into(),
            })),
        }
    }

    fn command_from_wire(command: &proto::GameCommand) -> Result<Command, WireError> {
        let game_command::Game::Checkers(command) =
            command.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        match command
            .command
            .as_ref()
            .ok_or(WireError::Missing("command"))?
        {
            checkers_command::Command::Move(chosen) => {
                // Bounded before anything is allocated from it.
                if chosen.path.len() > Path::MAX_SQUARES {
                    return Err(WireError::OutOfRange("path"));
                }
                let squares = chosen
                    .path
                    .iter()
                    .map(|index| Square::from_index(*index))
                    .collect::<Option<Vec<_>>>()
                    .ok_or(WireError::OutOfRange("path"))?;
                Path::new(squares)
                    .map(Command::Move)
                    .ok_or(WireError::OutOfRange("path"))
            }
        }
    }

    fn command_to_wire(command: &Command) -> proto::GameCommand {
        let Command::Move(path) = command;
        proto::GameCommand {
            game: Some(game_command::Game::Checkers(proto::CheckersCommand {
                command: Some(checkers_command::Command::Move(proto::CheckersMove {
                    path: path.squares().iter().map(|square| square.index()).collect(),
                })),
            })),
        }
    }

    fn world_event_from_wire(event: &proto::GameWorldEvent) -> Result<WorldEvent, WireError> {
        let game_world_event::Game::Checkers(event) =
            event.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        match event.event.as_ref().ok_or(WireError::Missing("event"))? {
            checkers_world_event::Event::WhiteChosen(chosen) => {
                Ok(WorldEvent::WhiteChosen(slot_from_wire(chosen.white_slot)?))
            }
        }
    }

    fn world_event_to_wire(event: &WorldEvent) -> proto::GameWorldEvent {
        let WorldEvent::WhiteChosen(white) = *event;
        proto::GameWorldEvent {
            game: Some(game_world_event::Game::Checkers(
                proto::CheckersWorldEvent {
                    event: Some(checkers_world_event::Event::WhiteChosen(
                        proto::CheckersWhiteChosen {
                            white_slot: slot_to_wire(white),
                        },
                    )),
                },
            )),
        }
    }

    fn view_to_wire(view: &View) -> proto::GameView {
        proto::GameView {
            game: Some(game_view::Game::Checkers(proto::CheckersView {
                pieces: view.pieces.iter().map(piece_to_wire).collect(),
                white_slot: view.white_player.map(slot_to_wire),
                black_slot: view.black_player.map(slot_to_wire),
                current_slot: view.current_player.map(slot_to_wire),
                side_to_move: side_to_wire(view.side_to_move).into(),
                you: view.you.map(slot_to_wire),
                legal_moves: view.legal_moves.iter().map(legal_move_to_wire).collect(),
                last_move: view.last_move.as_ref().map(legal_move_to_wire),
                end: view.end.map(game_end_to_wire),
                king_moves: u32::from(view.king_moves),
                king_moves_for_draw: u32::from(KING_MOVES_FOR_DRAW),
            })),
        }
    }
}

impl WireBots for Checkers {
    fn bot_kind_from_wire(kind: &proto::BotKind) -> Result<BotKind, WireError> {
        let kind = match kind.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::bot_kind::Game::Checkers(kind) => *kind,
            _ => return Err(WireError::WrongGame),
        };
        match CheckersBotKind::try_from(kind) {
            Ok(CheckersBotKind::Easy) => Ok(BotKind::Easy),
            Ok(CheckersBotKind::Medium) => Ok(BotKind::Medium),
            Ok(CheckersBotKind::Hard) => Ok(BotKind::Hard),
            Ok(CheckersBotKind::Master) => Ok(BotKind::Master),
            Ok(CheckersBotKind::Unspecified) => Err(WireError::Missing("bot_kind")),
            Err(_) => Err(WireError::UnknownEnumValue("bot_kind")),
        }
    }

    fn bot_kind_to_wire(kind: BotKind) -> proto::BotKind {
        let kind = match kind {
            BotKind::Easy => CheckersBotKind::Easy,
            BotKind::Medium => CheckersBotKind::Medium,
            BotKind::Hard => CheckersBotKind::Hard,
            BotKind::Master => CheckersBotKind::Master,
        };
        proto::BotKind {
            game: Some(proto::bot_kind::Game::Checkers(kind.into())),
        }
    }
}

fn side_to_wire(side: Side) -> CheckersSide {
    match side {
        Side::White => CheckersSide::White,
        Side::Black => CheckersSide::Black,
    }
}

fn piece_to_wire(placed: &PlacedPiece) -> proto::CheckersPiece {
    proto::CheckersPiece {
        id: u32::from(placed.piece.id.get()),
        square: placed.square.index(),
        side: side_to_wire(placed.piece.side).into(),
        king: placed.piece.rank == mg_checkers::Rank::King,
    }
}

fn legal_move_to_wire(legal: &LegalMove) -> proto::CheckersLegalMove {
    proto::CheckersLegalMove {
        path: legal
            .path
            .squares()
            .iter()
            .map(|square| square.index())
            .collect(),
        captured: legal.captured.iter().map(|square| square.index()).collect(),
        crowns: legal.crowns,
    }
}

fn game_end_to_wire(end: GameEnd) -> proto::CheckersGameEnd {
    let reason = match end {
        GameEnd::Win {
            reason: WinReason::NoPieces,
            ..
        } => CheckersEndReason::NoPieces,
        GameEnd::Win {
            reason: WinReason::NoMoves,
            ..
        } => CheckersEndReason::NoMoves,
        GameEnd::Win {
            reason: WinReason::OpponentLeft,
            ..
        } => CheckersEndReason::OpponentLeft,
        GameEnd::Draw(DrawReason::Repetition) => CheckersEndReason::Repetition,
        GameEnd::Draw(DrawReason::KingMoves) => CheckersEndReason::KingMoves,
    };
    proto::CheckersGameEnd {
        reason: reason.into(),
        winner_slot: end.winner().map(slot_to_wire),
    }
}

impl WireRefusal for Refusal {
    fn refusal_to_wire(self) -> proto::refusal::Reason {
        let refusal = match self {
            Refusal::MustCapture => proto::CheckersRefusal::MustCapture,
            Refusal::NotALegalMove => proto::CheckersRefusal::NotALegalMove,
            Refusal::WhiteAlreadyKnown | Refusal::NoSuchSeat => return malformed_refusal(),
        };
        proto::refusal::Reason::Checkers(refusal.into())
    }

    fn refusal_from_wire(value: i32) -> Option<Self> {
        Some(match proto::CheckersRefusal::try_from(value).ok()? {
            proto::CheckersRefusal::Unspecified => return None,
            proto::CheckersRefusal::MustCapture => Refusal::MustCapture,
            proto::CheckersRefusal::NotALegalMove => Refusal::NotALegalMove,
        })
    }
}

#[cfg(test)]
#[path = "checkers_tests.rs"]
mod tests;
