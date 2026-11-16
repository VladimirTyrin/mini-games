use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, GameInfo, Outcome, OutcomeKind, PlayerSpec,
    SetupError, Slot, StateHasher, Status, Viewer,
};

use crate::io::{Command, WorldEvent};
use crate::model::{Answers, LetterMark, Phase, ScoredGuess, Seats, Word};
use crate::rules::{GROUP_GUESS_CAP, check_hard_mode, score};
use crate::settings::{Puzzle, Settings};
use crate::view::{KeyHint, View};

pub(crate) const MAX_PLAYERS: u8 = 4;
const SOLO_PLAYER: Slot = Slot::new(0);

/// Guess a hidden word: every guess marks its letters as in place, elsewhere in the word, or
/// absent. Alone, within the guesses the settings allow; in a group (2–4 seats, people or bots)
/// the players take turns on one shared board and whoever finds the word wins.
#[derive(Clone, Debug)]
pub struct WordHunt {
    settings: Settings,
    seats: Seats,
    answer: Option<Word>,
    guesses: Vec<ScoredGuess>,
    phase: Phase,
    /// Whose guess is next; `None` before the first player is chosen and after the end.
    turn: Option<Slot>,
    winner: Option<Slot>,
}

impl WordHunt {
    pub(crate) fn answers(&self) -> Answers {
        Answers::new(self.settings.language, self.settings.word_length())
    }

    pub(crate) fn settings(&self) -> &Settings {
        &self.settings
    }

    pub(crate) fn guesses(&self) -> &[ScoredGuess] {
        &self.guesses
    }

    /// Alone the settings decide; a group plays until someone finds the word, with a cap only
    /// so that a game cannot grow without end.
    fn guess_limit(&self) -> usize {
        if self.seats.is_group() {
            GROUP_GUESS_CAP
        } else {
            usize::from(self.settings.max_guesses.to_u8())
        }
    }

    fn keyboard(&self) -> Vec<KeyHint> {
        self.settings
            .language
            .alphabet()
            .filter_map(|letter| {
                let mark = self
                    .guesses
                    .iter()
                    .flat_map(|guess| guess.word.letters().iter().zip(&guess.marks))
                    .filter(|(guessed, _)| **guessed == letter)
                    .map(|(_, mark)| *mark)
                    .max()?;
                Some(KeyHint { letter, mark })
            })
            .collect()
    }

    fn end(&mut self, phase: Phase, winner: Option<Slot>) {
        self.phase = phase;
        self.winner = winner;
        self.turn = None;
    }
}

impl Game for WordHunt {
    type Settings = Settings;
    type Command = Command;
    type WorldEvent = WorldEvent;
    type View = View;

    const INFO: GameInfo = GameInfo {
        id: "word_hunt",
        name: "Word Hunt",
        min_players: 1,
        max_players: MAX_PLAYERS,
        has_bots: true,
        cooperative: false,
    };

    fn new(settings: &Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        SetupError::check_player_count(
            players.len(),
            Self::INFO.min_players,
            Self::INFO.max_players,
        )?;
        if players.len() > 1 && matches!(settings.puzzle(), Puzzle::Daily(_)) {
            return Err(SetupError::InvalidPlayers(
                "the word of the day is played alone".into(),
            ));
        }
        if !players.iter().any(|player| player.is_human()) {
            return Err(SetupError::InvalidPlayers(
                "a person must play, not only bots".into(),
            ));
        }
        Ok(Self {
            settings: *settings,
            seats: Seats::new(players),
            answer: None,
            guesses: Vec::new(),
            phase: Phase::Choosing,
            turn: None,
            winner: None,
        })
    }

    fn clock(&self) -> Clock {
        Clock::TurnBased
    }

    fn apply_command(&mut self, slot: Slot, command: &Command) -> Result<(), CommandError> {
        if self.phase.is_over() {
            return Err(CommandError::GameOver);
        }
        if self.turn != Some(slot) {
            return Err(CommandError::NotYourTurn);
        }
        let Some(answer) = &self.answer else {
            return Err(CommandError::Illegal("the word is not chosen yet"));
        };
        let Command::Guess(text) = command;
        let guess = Word::parse(text, self.settings.language, self.settings.word_length())
            .map_err(|error| CommandError::Illegal(error.reason()))?;
        if self.guesses.iter().any(|earlier| earlier.word == guess) {
            return Err(CommandError::Illegal("you have already tried this word"));
        }
        if self.settings.hard_mode {
            check_hard_mode(&guess, &self.guesses)
                .map_err(|violation| CommandError::Illegal(violation.reason()))?;
        }
        let scored = score(&guess, answer, slot);
        let solved = scored.is_solved();
        self.guesses.push(scored);
        if solved {
            self.end(Phase::Solved, Some(slot));
        } else if self.guesses.len() >= self.guess_limit() {
            self.end(Phase::Failed, None);
        } else {
            self.turn = self.seats.next_after(slot);
        }
        Ok(())
    }

    fn roll_world(&self, rng: &mut CoreRng) -> Vec<WorldEvent> {
        if self.phase != Phase::Choosing {
            return Vec::new();
        }
        let answer = match self.settings.puzzle() {
            Puzzle::Unlimited => self.answers().random(rng),
            Puzzle::Daily(day) => self.answers().daily(day),
        };
        let Some(answer) = answer else {
            return Vec::new();
        };
        let mut events = vec![WorldEvent::AnswerChosen(answer.text())];
        if self.seats.is_group() {
            let seats: Vec<Slot> = self.seats.in_play().collect();
            if let Some(first) = rng.pick(&seats) {
                events.push(WorldEvent::FirstPlayerChosen(*first));
            }
        }
        events
    }

    fn apply_world(&mut self, event: &WorldEvent) -> Result<(), CommandError> {
        match event {
            WorldEvent::AnswerChosen(text) => {
                if self.phase != Phase::Choosing {
                    return Err(CommandError::Illegal("the answer is already chosen"));
                }
                let answer = Word::parse(text, self.settings.language, self.settings.word_length())
                    .map_err(|error| CommandError::Illegal(error.reason()))?;
                self.answer = Some(answer);
                self.phase = Phase::Guessing;
                // Alone there is nobody to choose: the one seat starts.
                if !self.seats.is_group() {
                    self.turn = Some(SOLO_PLAYER);
                }
                Ok(())
            }
            WorldEvent::FirstPlayerChosen(slot) => {
                if self.phase != Phase::Guessing
                    || self.turn.is_some()
                    || !self.guesses.is_empty()
                    || !self.seats.is_in_play(*slot)
                {
                    return Err(CommandError::Illegal("the first player is already chosen"));
                }
                self.turn = Some(*slot);
                Ok(())
            }
        }
    }

    fn drop_player(&mut self, slot: Slot) {
        if self.phase.is_over() || !self.seats.is_in_play(slot) {
            return;
        }
        self.seats.remove(slot);
        if !self.seats.any_human_in_play() {
            self.end(Phase::Abandoned, None);
            return;
        }
        let last_standing = {
            let mut remaining = self.seats.in_play();
            match (remaining.next(), remaining.next()) {
                (Some(last), None) => Some(last),
                _ => None,
            }
        };
        if let Some(last) = last_standing {
            self.end(Phase::Forfeited, Some(last));
            return;
        }
        if self.turn == Some(slot) {
            self.turn = self.seats.next_after(slot);
        }
    }

    fn awaiting(&self) -> Awaiting {
        match (self.phase, self.turn) {
            (Phase::Guessing, Some(slot)) => Awaiting::Turn(slot),
            _ => Awaiting::Nobody,
        }
    }

    fn status(&self) -> Status {
        let seats = self.seats.count();
        let finished = |kind, winners: Vec<Slot>, scores: Vec<i64>| {
            Status::Finished(Outcome {
                kind,
                winners,
                scores,
            })
        };
        let one_point_for = |winner: Slot| {
            Slot::all(seats)
                .map(|slot| i64::from(slot == winner))
                .collect()
        };
        match (self.phase, self.winner) {
            (Phase::Choosing | Phase::Guessing, _) => Status::InProgress,
            (Phase::Solved | Phase::Forfeited, Some(winner)) if self.seats.is_group() => {
                finished(OutcomeKind::Winner, vec![winner], one_point_for(winner))
            }
            // Alone: the guesses to spare plus one, so a win on the last guess still scores.
            (Phase::Solved, _) => {
                let used = u32::try_from(self.guesses.len()).unwrap_or(u32::MAX);
                let spare = self.settings.max_guesses.get().saturating_sub(used);
                finished(OutcomeKind::Victory, Vec::new(), vec![i64::from(spare) + 1])
            }
            _ if self.seats.is_group() => finished(OutcomeKind::Draw, Vec::new(), vec![0; seats]),
            _ => finished(OutcomeKind::Defeat, Vec::new(), vec![0]),
        }
    }

    fn view(&self, _viewer: Viewer) -> View {
        View {
            settings: self.settings,
            phase: self.phase,
            guesses: self.guesses.clone(),
            keyboard: self.keyboard(),
            answer: self
                .answer
                .as_ref()
                .filter(|_| self.phase.is_over())
                .map(Word::text),
            turn: self.turn,
            winner: self.winner,
            guess_limit: u32::try_from(self.guess_limit()).unwrap_or(u32::MAX),
        }
    }

    fn hash_state(&self, hasher: &mut StateHasher) {
        let settings = self.settings;
        hasher.u8(settings.language.hash_code());
        hasher.u8(settings.word_length().to_u8());
        hasher.u8(settings.max_guesses.to_u8());
        hasher.bool(settings.hard_mode);
        match settings.puzzle() {
            Puzzle::Unlimited => hasher.u32(u32::MAX),
            Puzzle::Daily(day) => hasher.u32(day.get()),
        }
        hasher.u8(self.phase.hash_code());
        hasher.bool(self.answer.is_some());
        for letter in self.answer.iter().flat_map(Word::letters) {
            hasher.u32(u32::from(*letter));
        }
        hasher.length(self.guesses.len());
        for guess in &self.guesses {
            for letter in guess.word.letters() {
                hasher.u32(u32::from(*letter));
            }
            for mark in &guess.marks {
                hasher.u8(match mark {
                    LetterMark::Absent => 0,
                    LetterMark::Present => 1,
                    LetterMark::Correct => 2,
                });
            }
        }
        // Alone the seat, its turn and the authors never vary; hashing them only for groups
        // keeps single-player replays recorded before groups existed valid.
        if self.seats.is_group() {
            self.seats.hash(hasher);
            hasher.u8(self.turn.map_or(u8::MAX, Slot::get));
            hasher.u8(self.winner.map_or(u8::MAX, Slot::get));
            for guess in &self.guesses {
                hasher.u8(guess.by.get());
            }
        }
    }
}
