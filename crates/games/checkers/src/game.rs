use std::collections::BTreeMap;

use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, GameInfo, Outcome, OutcomeKind, PlayerSpec,
    ScoreKind, SetupError, Slot, StateHasher, Status, Viewer,
};

use crate::io::{Command, WorldEvent};
use crate::model::{Board, Rank, Side, Square};
use crate::rules::{
    DrawReason, GameEnd, KING_MOVES_FOR_DRAW, LegalMove, REPETITIONS_FOR_DRAW, Refusal, WinReason,
    apply_move, legal_moves, position_key,
};
use crate::settings::{CaptureRule, PLAYERS, Settings, WhitePlayer};
use crate::view::{PlacedPiece, View};

const HOST: Slot = Slot::new(0);
const GUEST: Slot = Slot::new(1);

fn other_seat(slot: Slot) -> Slot {
    if slot == HOST { GUEST } else { HOST }
}

/// Russian draughts for two seats (see `rules::move_generator` for the moves).
#[derive(Clone, Debug)]
pub struct Checkers {
    board: Board,
    white_mode: WhitePlayer,
    capture_rule: CaptureRule,
    /// `None` until drawn by lot.
    white_player: Option<Slot>,
    to_move: Side,
    /// Moves in a row with only kings moving and nothing captured.
    king_moves: u8,
    /// How often each position (with the side to move) came up since the last capture or man
    /// move: none of those can come back, so the map holds at most `KING_MOVES_FOR_DRAW` + 1.
    positions: BTreeMap<u64, u8>,
    last_move: Option<LegalMove>,
    end: Option<GameEnd>,
}

impl Checkers {
    /// A game in any position: the host plays white, `to_move` is to move.
    #[cfg(test)]
    pub(crate) fn from_position(
        pieces: &[(Square, Side, Rank)],
        to_move: Side,
        king_moves: u8,
    ) -> Self {
        let mut board = Board::empty();
        for (id, (square, side, rank)) in (0u8..).zip(pieces) {
            board.put(
                *square,
                Some(crate::model::Piece {
                    id: crate::model::PieceId::new(id),
                    side: *side,
                    rank: *rank,
                }),
            );
        }
        let mut positions = BTreeMap::new();
        positions.insert(position_key(&board, to_move), 1);
        Self {
            board,
            white_mode: WhitePlayer::Host,
            capture_rule: CaptureRule::ToTheEnd,
            white_player: Some(HOST),
            to_move,
            king_moves,
            positions,
            last_move: None,
            end: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_capture_rule(mut self, rule: CaptureRule) -> Self {
        self.capture_rule = rule;
        self
    }

    pub(crate) fn capture_rule(&self) -> CaptureRule {
        self.capture_rule
    }

    pub(crate) fn board(&self) -> &Board {
        &self.board
    }

    pub(crate) fn to_move(&self) -> Side {
        self.to_move
    }

    pub fn end(&self) -> Option<GameEnd> {
        self.end
    }

    pub fn player_with(&self, side: Side) -> Option<Slot> {
        self.white_player.map(|white| match side {
            Side::White => white,
            Side::Black => other_seat(white),
        })
    }

    pub fn side_of(&self, slot: Slot) -> Option<Side> {
        [Side::White, Side::Black]
            .into_iter()
            .find(|side| self.player_with(*side) == Some(slot))
    }

    pub fn current_player(&self) -> Option<Slot> {
        if self.end.is_some() {
            return None;
        }
        self.player_with(self.to_move)
    }

    /// What the side to move may do now (nothing once the game is over).
    pub fn legal_moves(&self) -> Vec<LegalMove> {
        if self.end.is_some() {
            return Vec::new();
        }
        legal_moves(&self.board, self.to_move, self.capture_rule)
    }

    /// Plays a move known to be legal: the bots search with this.
    pub(crate) fn play(&mut self, chosen: &LegalMove) {
        let man_moved = chosen
            .path
            .from()
            .and_then(|from| self.board.at(from))
            .is_some_and(|piece| piece.rank == Rank::Man);
        apply_move(&mut self.board, chosen);
        let mover = self.to_move;
        self.to_move = mover.opponent();
        self.last_move = Some(chosen.clone());

        if chosen.captured.is_empty() && !man_moved {
            self.king_moves = self.king_moves.saturating_add(1);
        } else {
            // Nothing before this can come back: the repetition rule starts afresh.
            self.king_moves = 0;
            self.positions.clear();
        }
        let seen = self
            .positions
            .entry(position_key(&self.board, self.to_move))
            .or_insert(0);
        *seen = seen.saturating_add(1);
        let repeated = *seen >= REPETITIONS_FOR_DRAW;

        self.end = self.end_after_move(mover, repeated);
    }

    fn end_after_move(&self, mover: Side, repeated: bool) -> Option<GameEnd> {
        let loser = mover.opponent();
        let winner = self.player_with(mover)?;
        if self.board.count(loser) == 0 {
            return Some(GameEnd::Win {
                winner,
                reason: WinReason::NoPieces,
            });
        }
        if legal_moves(&self.board, loser, self.capture_rule).is_empty() {
            return Some(GameEnd::Win {
                winner,
                reason: WinReason::NoMoves,
            });
        }
        if repeated {
            return Some(GameEnd::Draw(DrawReason::Repetition));
        }
        if self.king_moves >= KING_MOVES_FOR_DRAW {
            return Some(GameEnd::Draw(DrawReason::KingMoves));
        }
        None
    }
}

impl Game for Checkers {
    type Settings = Settings;
    type Command = Command;
    type WorldEvent = WorldEvent;
    type View = View;
    type Secret = ();
    type Refusal = Refusal;

    const INFO: GameInfo = GameInfo {
        id: "checkers",
        name: "Checkers",
        min_players: PLAYERS,
        max_players: PLAYERS,
        has_bots: true,
        cooperative: false,
        scores: ScoreKind::WinFlag,
    };

    fn new(settings: &Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        SetupError::check_player_count(players.len(), PLAYERS, PLAYERS)?;
        let white_player = match settings.white_player {
            WhitePlayer::Host => Some(HOST),
            WhitePlayer::Guest => Some(GUEST),
            WhitePlayer::Random => None,
        };
        let board = Board::starting();
        let mut positions = BTreeMap::new();
        positions.insert(position_key(&board, Side::White), 1);
        Ok(Self {
            board,
            white_mode: settings.white_player,
            capture_rule: settings.capture_rule,
            white_player,
            to_move: Side::White,
            king_moves: 0,
            positions,
            last_move: None,
            end: None,
        })
    }

    fn clock(&self) -> Clock {
        Clock::TurnBased
    }

    fn apply_command(
        &mut self,
        slot: Slot,
        command: &Command,
    ) -> Result<(), CommandError<Refusal>> {
        if self.end.is_some() {
            return Err(CommandError::GameOver);
        }
        if slot.get() >= PLAYERS {
            return Err(CommandError::NotPlaying);
        }
        if self.current_player() != Some(slot) {
            return Err(CommandError::NotYourTurn);
        }
        let Command::Move(path) = command;
        let moves = legal_moves(&self.board, self.to_move, self.capture_rule);
        let Some(chosen) = moves.iter().find(|candidate| &candidate.path == path) else {
            let must_capture = moves.iter().any(|candidate| !candidate.captured.is_empty());
            return Err(CommandError::Illegal(if must_capture {
                Refusal::MustCapture
            } else {
                Refusal::NotALegalMove
            }));
        };
        let chosen = chosen.clone();
        self.play(&chosen);
        Ok(())
    }

    fn roll_world(&self, rng: &mut CoreRng) -> Vec<WorldEvent> {
        if self.white_player.is_some() || self.end.is_some() {
            return Vec::new();
        }
        let white = if rng.chance(1, 2) { GUEST } else { HOST };
        vec![WorldEvent::WhiteChosen(white)]
    }

    fn apply_world(&mut self, event: &WorldEvent) -> Result<(), CommandError<Refusal>> {
        let WorldEvent::WhiteChosen(white) = *event;
        if self.white_mode != WhitePlayer::Random
            || self.white_player.is_some()
            || self.end.is_some()
        {
            return Err(CommandError::Illegal(Refusal::WhiteAlreadyKnown));
        }
        if white.get() >= PLAYERS {
            return Err(CommandError::Illegal(Refusal::NoSuchSeat));
        }
        self.white_player = Some(white);
        Ok(())
    }

    /// Leaving is a technical loss, also before the first move.
    fn drop_player(&mut self, slot: Slot) {
        if self.end.is_none() && slot.get() < PLAYERS {
            self.end = Some(GameEnd::Win {
                winner: other_seat(slot),
                reason: WinReason::OpponentLeft,
            });
        }
    }

    /// `Nobody` also before white is drawn: the authority resolves that in the setup frame.
    fn awaiting(&self) -> Awaiting {
        self.current_player()
            .map_or(Awaiting::Nobody, Awaiting::Turn)
    }

    fn status(&self) -> Status {
        let Some(end) = self.end else {
            return Status::InProgress;
        };
        let winner = end.winner();
        let scores = Slot::all(usize::from(PLAYERS))
            .map(|slot| i64::from(winner == Some(slot)))
            .collect();
        Status::Finished(Outcome {
            kind: if winner.is_some() {
                OutcomeKind::Winner
            } else {
                OutcomeKind::Draw
            },
            winners: winner.into_iter().collect(),
            scores,
        })
    }

    fn view(&self, viewer: Viewer) -> View {
        View {
            pieces: self
                .board
                .pieces()
                .map(|(square, piece)| PlacedPiece { square, piece })
                .collect(),
            white_player: self.player_with(Side::White),
            black_player: self.player_with(Side::Black),
            current_player: self.current_player(),
            side_to_move: self.to_move,
            you: match viewer {
                Viewer::Player(slot) => (slot.get() < PLAYERS).then_some(slot),
                Viewer::Spectator => None,
            },
            legal_moves: if self.current_player().is_some() {
                self.legal_moves()
            } else {
                Vec::new()
            },
            last_move: self.last_move.clone(),
            end: self.end,
            king_moves: self.king_moves,
        }
    }

    fn hash_state(&self, hasher: &mut StateHasher) {
        hasher.u8(self.white_mode.code());
        // Only the house rule is hashed: games under the default rule (all of them before it
        // existed, replays included) keep the hashes they were recorded with.
        if self.capture_rule == CaptureRule::MayStop {
            hasher.u8(1);
        }
        for square in Square::all() {
            match self.board.at(square) {
                None => hasher.u8(0),
                Some(piece) => {
                    hasher.u8(piece.code());
                    hasher.u8(piece.id.get());
                }
            }
        }
        hasher.bool(self.white_player.is_some());
        hasher.u8(self.white_player.map_or(0, Slot::get));
        hasher.u8(self.to_move.code());
        hasher.u8(self.king_moves);
        hasher.length(self.positions.len());
        for (key, seen) in &self.positions {
            hasher.u64(*key);
            hasher.u8(*seen);
        }
        match &self.last_move {
            None => hasher.bool(false),
            Some(last) => {
                hasher.bool(true);
                hasher.length(last.path.squares().len());
                for square in last.path.squares() {
                    hasher.u32(square.index());
                }
                hasher.length(last.captured.len());
                for square in &last.captured {
                    hasher.u32(square.index());
                }
                hasher.bool(last.crowns);
            }
        }
        match self.end {
            None => hasher.u8(0),
            Some(end) => {
                hasher.u8(end.code());
                hasher.u8(end.winner().map_or(u8::MAX, Slot::get));
            }
        }
    }
}
