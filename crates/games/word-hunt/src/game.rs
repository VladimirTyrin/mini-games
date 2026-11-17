use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, GameInfo, Outcome, OutcomeKind, PlayerSpec,
    SetupError, Slot, StateHasher, Status, Viewer,
};

use crate::io::{Command, WorldEvent};
use crate::model::{
    Answers, HiddenWord, LetterMark, PendingGuess, Phase, ScoredGuess, Seats, Word,
};
use crate::rules::{GROUP_GUESS_CAP, check_hard_mode, mark_guess};
use crate::settings::{FirstTurn, Puzzle, Settings};
use crate::view::{KeyHint, View};

pub(crate) const MAX_PLAYERS: u8 = 4;
const SOLO_PLAYER: Slot = Slot::new(0);

/// Guess a hidden word: every guess marks its letters as in place, elsewhere in the word, or
/// absent. Alone, within the guesses the settings allow; in a group (2–4 seats, people or bots)
/// the players take turns on one shared board and whoever finds the word wins.
///
/// The word is the authority's secret ([`HiddenWord`]): a guess waits for the authority to mark
/// it, and the word itself is revealed only once the game is over.
#[derive(Clone, Debug)]
pub struct WordHunt {
    settings: Settings,
    seats: Seats,
    pending_guess: Option<PendingGuess>,
    /// Known once the game is over.
    revealed_answer: Option<Word>,
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

    /// The seat the settings make start; `None` when it is drawn at random.
    fn fixed_first_player(&self) -> Option<Slot> {
        match self.settings.first_turn {
            FirstTurn::Random => None,
            FirstTurn::HostFirst => self.seats.in_play().next(),
            // Turns go up the table, so the host closes every round when the seat after it starts.
            FirstTurn::HostLast => {
                let host = self.seats.in_play().next()?;
                self.seats.next_after(host)
            }
        }
    }

    fn end(&mut self, phase: Phase, winner: Option<Slot>) {
        self.phase = phase;
        self.winner = winner;
        self.turn = None;
    }

    /// A seat leaves the table (gave up, left, or was dropped). Whoever is left alone wins; with
    /// no person left the game stops; otherwise the turn moves on if it was the leaver's.
    fn leave_table(&mut self, slot: Slot) {
        self.seats.remove(slot);
        let last_standing = {
            let mut remaining = self.seats.in_play();
            match (remaining.next(), remaining.next()) {
                (Some(last), None) => Some(last),
                _ => None,
            }
        };
        if let Some(last) = last_standing.filter(|_| self.seats.is_group()) {
            self.end(Phase::Forfeited, Some(last));
        } else if self.seats.in_play().next().is_none()
            || (self.seats.has_people() && !self.seats.any_human_in_play())
        {
            // Nobody left to guess (a lone bot dropped), or only bots left with people gone.
            self.end(Phase::Abandoned, None);
        } else if self.turn == Some(slot) {
            self.turn = self.seats.next_after(slot);
        }
    }

    fn record_marks(&mut self, marks: &[LetterMark]) -> Result<(), CommandError> {
        if self.phase != Phase::Guessing {
            return Err(CommandError::Illegal("no guess is being made"));
        }
        let pending = match self.pending_guess.take() {
            Some(pending) if marks.len() == pending.word.letters().len() => pending,
            unfit => {
                // A refused event leaves the state untouched: the guess goes back.
                self.pending_guess = unfit;
                return Err(CommandError::Illegal("there is no guess these marks fit"));
            }
        };
        let scored = ScoredGuess {
            word: pending.word,
            marks: marks.to_vec(),
            by: pending.by,
        };
        let by = scored.by;
        let solved = scored.is_solved();
        self.guesses.push(scored);
        if solved {
            self.end(Phase::Solved, Some(by));
        } else if self.guesses.len() >= self.guess_limit() {
            self.end(Phase::Failed, None);
        } else {
            self.turn = self.seats.next_after(by);
        }
        Ok(())
    }

    fn give_up(&mut self, slot: Slot) -> Result<(), CommandError> {
        if self.phase != Phase::Guessing || !self.seats.is_in_play(slot) {
            return Err(CommandError::Illegal(
                "only a player still guessing can give up",
            ));
        }
        if self.seats.is_group() {
            self.leave_table(slot);
        } else {
            // Alone it is a loss like running out of guesses: the word is shown.
            self.end(Phase::Failed, None);
        }
        Ok(())
    }
}

impl Game for WordHunt {
    type Settings = Settings;
    type Command = Command;
    type WorldEvent = WorldEvent;
    type View = View;
    type Secret = HiddenWord;

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
        Ok(Self {
            settings: *settings,
            seats: Seats::new(players),
            pending_guess: None,
            revealed_answer: None,
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
        let text = match command {
            Command::GiveUp => return self.give_up(slot),
            Command::Guess(text) => text,
        };
        if self.phase != Phase::Guessing {
            return Err(CommandError::Illegal("the word is not chosen yet"));
        }
        if self.pending_guess.is_some() {
            return Err(CommandError::Illegal("the last guess is not marked yet"));
        }
        if self.turn != Some(slot) {
            return Err(CommandError::NotYourTurn);
        }
        let guess = Word::parse(text, self.settings.language, self.settings.word_length())
            .map_err(|error| CommandError::Illegal(error.reason()))?;
        if self.guesses.iter().any(|earlier| earlier.word == guess) {
            return Err(CommandError::Illegal("this word was already tried"));
        }
        if self.settings.hard_mode {
            check_hard_mode(&guess, &self.guesses)
                .map_err(|violation| CommandError::Illegal(violation.reason()))?;
        }
        self.pending_guess = Some(PendingGuess {
            word: guess,
            by: slot,
        });
        Ok(())
    }

    fn roll_secret(&self, secret: &mut HiddenWord, rng: &mut CoreRng) -> Vec<WorldEvent> {
        match self.phase {
            Phase::Choosing => {
                let answer = match self.settings.puzzle() {
                    Puzzle::Unlimited => self.answers().random(rng),
                    Puzzle::Daily(day) => self.answers().daily(day),
                };
                let Some(answer) = answer else {
                    return Vec::new();
                };
                secret.answer = Some(answer);
                vec![WorldEvent::WordChosen]
            }
            Phase::Guessing => match (&self.pending_guess, &secret.answer) {
                (Some(pending), Some(answer)) => {
                    vec![WorldEvent::GuessMarked(mark_guess(&pending.word, answer))]
                }
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    fn roll_world(&self, rng: &mut CoreRng) -> Vec<WorldEvent> {
        let is_starting_group = self.phase == Phase::Guessing
            && self.seats.is_group()
            && self.turn.is_none()
            && self.guesses.is_empty()
            && self.pending_guess.is_none();
        if !is_starting_group {
            return Vec::new();
        }
        let seats: Vec<Slot> = self.seats.in_play().collect();
        // Chosen here even when the settings fix it, so every group game starts the same way on
        // the wire and a replay holds the first player.
        let first = match self.fixed_first_player() {
            Some(fixed) => Some(fixed),
            None => rng.pick(&seats).copied(),
        };
        first
            .map(WorldEvent::FirstPlayerChosen)
            .into_iter()
            .collect()
    }

    fn reveal(&self, secret: &HiddenWord) -> Vec<WorldEvent> {
        secret
            .answer
            .iter()
            .map(|answer| WorldEvent::WordRevealed(answer.text()))
            .collect()
    }

    fn apply_world(&mut self, event: &WorldEvent) -> Result<(), CommandError> {
        match event {
            WorldEvent::WordChosen => {
                if self.phase != Phase::Choosing {
                    return Err(CommandError::Illegal("the word is already chosen"));
                }
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
                if self
                    .fixed_first_player()
                    .is_some_and(|fixed| fixed != *slot)
                {
                    return Err(CommandError::Illegal(
                        "the settings name another first player",
                    ));
                }
                self.turn = Some(*slot);
                Ok(())
            }
            WorldEvent::GuessMarked(marks) => self.record_marks(marks),
            WorldEvent::WordRevealed(text) => {
                if !self.phase.is_over() || self.revealed_answer.is_some() {
                    return Err(CommandError::Illegal("the word is revealed at the end"));
                }
                let answer = Word::parse(text, self.settings.language, self.settings.word_length())
                    .map_err(|error| CommandError::Illegal(error.reason()))?;
                self.revealed_answer = Some(answer);
                Ok(())
            }
        }
    }

    fn drop_player(&mut self, slot: Slot) {
        if self.phase.is_over() || !self.seats.is_in_play(slot) {
            return;
        }
        self.leave_table(slot);
    }

    fn awaiting(&self) -> Awaiting {
        match (self.phase, self.turn) {
            (Phase::Guessing, Some(slot)) if self.pending_guess.is_none() => Awaiting::Turn(slot),
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
            answer: self.revealed_answer.as_ref().map(Word::text),
            turn: self.turn,
            winner: self.winner,
            guess_limit: u32::try_from(self.guess_limit()).unwrap_or(u32::MAX),
            playing: self.seats.in_play().collect(),
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
        hasher.bool(self.revealed_answer.is_some());
        for letter in self.revealed_answer.iter().flat_map(Word::letters) {
            hasher.u32(u32::from(*letter));
        }
        hasher.bool(self.pending_guess.is_some());
        if let Some(pending) = &self.pending_guess {
            for letter in pending.word.letters() {
                hasher.u32(u32::from(*letter));
            }
            hasher.u8(pending.by.get());
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
            hasher.u8(settings.first_turn.hash_code());
            self.seats.hash(hasher);
            hasher.u8(self.turn.map_or(u8::MAX, Slot::get));
            hasher.u8(self.winner.map_or(u8::MAX, Slot::get));
            for guess in &self.guesses {
                hasher.u8(guess.by.get());
            }
        }
    }
}
