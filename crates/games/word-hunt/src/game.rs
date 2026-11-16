use mg_core::{
    Awaiting, Clock, CommandError, CoreRng, Game, GameInfo, Outcome, OutcomeKind, PlayerSpec,
    SetupError, Slot, StateHasher, Status, Viewer,
};

use crate::io::{Command, WorldEvent};
use crate::model::{Answers, LetterMark, Phase, ScoredGuess, Word};
use crate::rules::{check_hard_mode, score};
use crate::settings::{Puzzle, Settings};
use crate::view::{KeyHint, View};

const PLAYER: Slot = Slot::new(0);

/// Guess a hidden word: every guess marks its letters as in place, elsewhere in the word, or
/// absent. Single player.
#[derive(Clone, Debug)]
pub struct WordHunt {
    settings: Settings,
    answer: Option<Word>,
    guesses: Vec<ScoredGuess>,
    phase: Phase,
}

impl WordHunt {
    fn answers(&self) -> Answers {
        Answers::new(self.settings.language, self.settings.word_length())
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
        max_players: 1,
        has_bots: false,
        cooperative: false,
    };

    fn new(settings: &Settings, players: &[PlayerSpec]) -> Result<Self, SetupError> {
        SetupError::check_player_count(
            players.len(),
            Self::INFO.min_players,
            Self::INFO.max_players,
        )?;
        if players.iter().any(|player| !player.is_human()) {
            return Err(SetupError::InvalidPlayers("this game has no bots".into()));
        }
        Ok(Self {
            settings: *settings,
            answer: None,
            guesses: Vec::new(),
            phase: Phase::Choosing,
        })
    }

    fn clock(&self) -> Clock {
        Clock::TurnBased
    }

    fn apply_command(&mut self, slot: Slot, command: &Command) -> Result<(), CommandError> {
        if slot != PLAYER {
            return Err(CommandError::NotYourTurn);
        }
        if self.phase.is_over() {
            return Err(CommandError::GameOver);
        }
        let Some(answer) = &self.answer else {
            return Err(CommandError::Illegal("the word is not chosen yet"));
        };
        let Command::Guess(text) = command;
        let guess = Word::parse(text, self.settings.language, self.settings.word_length())
            .map_err(|error| CommandError::Illegal(error.reason()))?;
        if self.settings.hard_mode {
            check_hard_mode(&guess, &self.guesses)
                .map_err(|violation| CommandError::Illegal(violation.reason()))?;
        }
        let scored = score(&guess, answer);
        let solved = scored.is_solved();
        self.guesses.push(scored);
        if solved {
            self.phase = Phase::Solved;
        } else if self.guesses.len() >= usize::from(self.settings.max_guesses.to_u8()) {
            self.phase = Phase::Failed;
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
        answer
            .map(|word| WorldEvent::AnswerChosen(word.text()))
            .into_iter()
            .collect()
    }

    fn apply_world(&mut self, event: &WorldEvent) -> Result<(), CommandError> {
        let WorldEvent::AnswerChosen(text) = event;
        if self.phase != Phase::Choosing {
            return Err(CommandError::Illegal("the answer is already chosen"));
        }
        let answer = Word::parse(text, self.settings.language, self.settings.word_length())
            .map_err(|error| CommandError::Illegal(error.reason()))?;
        self.answer = Some(answer);
        self.phase = Phase::Guessing;
        Ok(())
    }

    fn drop_player(&mut self, slot: Slot) {
        if slot == PLAYER && !self.phase.is_over() {
            self.phase = Phase::Abandoned;
        }
    }

    fn awaiting(&self) -> Awaiting {
        match self.phase {
            Phase::Guessing => Awaiting::Turn(PLAYER),
            Phase::Choosing | Phase::Solved | Phase::Failed | Phase::Abandoned => Awaiting::Nobody,
        }
    }

    fn status(&self) -> Status {
        match self.phase {
            Phase::Choosing | Phase::Guessing => Status::InProgress,
            // Guesses to spare plus one, so a win on the last guess still scores.
            Phase::Solved => {
                let used = u32::try_from(self.guesses.len()).unwrap_or(u32::MAX);
                let spare = self.settings.max_guesses.get().saturating_sub(used);
                Status::Finished(Outcome {
                    kind: OutcomeKind::Victory,
                    winners: Vec::new(),
                    scores: vec![i64::from(spare) + 1],
                })
            }
            Phase::Failed | Phase::Abandoned => Status::Finished(Outcome {
                kind: OutcomeKind::Defeat,
                winners: Vec::new(),
                scores: vec![0],
            }),
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
    }
}
