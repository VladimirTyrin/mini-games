use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, GameInfo, Outcome, OutcomeKind, PlayerSpec,
    SettingRange, SetupError, Slot, StateHasher, Status, Viewer,
};

use crate::io::{Command, WorldEvent};
use crate::model::{Board, Mark, Point};
use crate::rules::GameEnd;
use crate::settings::{FirstPlayerMode, Settings, WinLength};
use crate::view::View;

#[derive(Clone, Debug)]
pub struct TicTacToe {
    board: Board,
    win_length: WinLength,
    first_player: FirstPlayerMode,
    x_player: Option<Slot>,
    to_move: Mark,
    marks_placed: usize,
    last_move: Option<Point>,
    end: Option<GameEnd>,
}

fn other_seat(slot: Slot) -> Slot {
    Slot::new(1 - slot.get().min(1))
}

impl TicTacToe {
    pub(crate) fn board(&self) -> &Board {
        &self.board
    }

    pub(crate) fn win_length(&self) -> WinLength {
        self.win_length
    }

    pub fn end(&self) -> Option<GameEnd> {
        self.end
    }

    pub fn player_with(&self, mark: Mark) -> Option<Slot> {
        self.x_player.map(|x_player| match mark {
            Mark::X => x_player,
            Mark::O => other_seat(x_player),
        })
    }

    pub fn mark_of(&self, slot: Slot) -> Option<Mark> {
        [Mark::X, Mark::O]
            .into_iter()
            .find(|&mark| self.player_with(mark) == Some(slot))
    }

    pub fn current_player(&self) -> Option<Slot> {
        if self.end.is_some() {
            return None;
        }
        self.player_with(self.to_move)
    }

    /// The mark `slot` places right now, if it is that seat's turn.
    pub(crate) fn mark_to_place(&self, slot: Slot) -> Option<Mark> {
        (self.current_player() == Some(slot)).then_some(self.to_move)
    }
}

impl Game for TicTacToe {
    type Settings = Settings;
    type Command = Command;
    type WorldEvent = WorldEvent;
    type View = View;

    const INFO: GameInfo = GameInfo {
        id: "tictactoe",
        name: "TicTacToe",
        min_players: crate::settings::PLAYERS,
        max_players: crate::settings::PLAYERS,
        has_bots: true,
        cooperative: false,
    };

    fn new(settings: &Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        let rules = settings.rules(players)?;
        let x_player = match settings.first_player {
            FirstPlayerMode::Host => Some(Slot::new(0)),
            FirstPlayerMode::Random => None,
        };
        Ok(Self {
            board: Board::new(rules.dimensions),
            win_length: rules.win_length,
            first_player: settings.first_player,
            x_player,
            to_move: Mark::X,
            marks_placed: 0,
            last_move: None,
            end: None,
        })
    }

    fn setting_ranges(settings: &Settings) -> Vec<SettingRange> {
        settings.ranges()
    }

    fn clock(&self) -> Clock {
        Clock::TurnBased
    }

    fn apply_command(&mut self, slot: Slot, command: &Command) -> Result<(), CommandError> {
        if self.end.is_some() {
            return Err(CommandError::GameOver);
        }
        if slot.get() >= crate::settings::PLAYERS {
            return Err(CommandError::NotPlaying);
        }
        let mark = self.mark_to_place(slot).ok_or(CommandError::NotYourTurn)?;
        let Command::Place(point) = *command;
        self.board.place(point, mark)?;
        self.marks_placed += 1;
        self.last_move = Some(point);
        if let Some(line) = self.board.line_through(point, self.win_length) {
            self.end = Some(GameEnd::Win { winner: slot, line });
        } else if self.marks_placed == self.board.cell_count() {
            self.end = Some(GameEnd::Draw);
        } else {
            self.to_move = mark.opponent();
        }
        Ok(())
    }

    fn roll_world(&self, rng: &mut CoreRng) -> Vec<WorldEvent> {
        if self.x_player.is_some() || self.end.is_some() {
            return Vec::new();
        }
        let x_player = Slot::new(if rng.chance(1, 2) { 1 } else { 0 });
        vec![WorldEvent::FirstPlayerChosen { x_player }]
    }

    fn apply_world(&mut self, event: &WorldEvent) -> Result<(), CommandError> {
        let WorldEvent::FirstPlayerChosen { x_player } = *event;
        if self.first_player != FirstPlayerMode::Random
            || self.x_player.is_some()
            || self.end.is_some()
        {
            return Err(CommandError::Illegal("the first player is already known"));
        }
        if x_player.get() >= crate::settings::PLAYERS {
            return Err(CommandError::Illegal("no such seat"));
        }
        self.x_player = Some(x_player);
        Ok(())
    }

    /// Leaving is a technical loss, also before the first move.
    fn drop_player(&mut self, slot: Slot) {
        if self.end.is_none() && slot.get() < crate::settings::PLAYERS {
            self.end = Some(GameEnd::OpponentLeft {
                winner: other_seat(slot),
            });
        }
    }

    /// `Nobody` also before a random first player is announced: the authority resolves that in
    /// the setup frame, before anyone can act.
    fn awaiting(&self) -> Awaiting {
        self.current_player()
            .map_or(Awaiting::Nobody, Awaiting::Turn)
    }

    fn status(&self) -> Status {
        let Some(end) = self.end else {
            return Status::InProgress;
        };
        let winner = end.winner();
        let scores = Slot::all(usize::from(crate::settings::PLAYERS))
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
        let dimensions = self.board.dimensions();
        View {
            field_width: dimensions.width(),
            field_height: dimensions.height(),
            win_count: self.win_length.get(),
            cells: self.board.cells().to_vec(),
            x_player: self.player_with(Mark::X),
            o_player: self.player_with(Mark::O),
            current_player: self.current_player(),
            last_move: self.last_move,
            end: self.end,
            you: match viewer {
                Viewer::Player(slot) => (slot.get() < crate::settings::PLAYERS).then_some(slot),
                Viewer::Spectator => None,
            },
        }
    }

    fn hash_state(&self, hasher: &mut StateHasher) {
        let dimensions = self.board.dimensions();
        hasher.u32(dimensions.width());
        hasher.u32(dimensions.height());
        hasher.u32(self.win_length.get());
        hasher.u8(match self.first_player {
            FirstPlayerMode::Random => 0,
            FirstPlayerMode::Host => 1,
        });
        for cell in self.board.cells() {
            hasher.u8(cell.map_or(0, Mark::code));
        }
        hash_slot(hasher, self.x_player);
        hasher.u8(self.to_move.code());
        hasher.length(self.marks_placed);
        hash_point(hasher, self.last_move);
        match self.end {
            None => hasher.u8(0),
            Some(GameEnd::Win { winner, line }) => {
                hasher.u8(1);
                hasher.u8(winner.get());
                hasher.u8(line.mark.code());
                hash_point(hasher, Some(line.start));
                hash_point(hasher, Some(line.end));
            }
            Some(GameEnd::Draw) => hasher.u8(2),
            Some(GameEnd::OpponentLeft { winner }) => {
                hasher.u8(3);
                hasher.u8(winner.get());
            }
        }
    }
}

fn hash_slot(hasher: &mut StateHasher, slot: Option<Slot>) {
    hasher.bool(slot.is_some());
    hasher.u8(slot.map_or(0, Slot::get));
}

fn hash_point(hasher: &mut StateHasher, point: Option<Point>) {
    hasher.bool(point.is_some());
    let point = point.unwrap_or(Point::new(0, 0));
    hasher.u32(point.x);
    hasher.u32(point.y);
}
