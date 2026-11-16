use mg_core::GameSettings;
use mg_word_hunt::{
    BotKind, Command, KeyHint, Language, LetterMark, Phase, PuzzleMode, RawSettings, ScoredGuess,
    View, WordHunt, WorldEvent,
};

use crate::proto::{
    game_command, game_settings, game_view, game_world_event, word_hunt_command,
    word_hunt_world_event,
};
use crate::{WireBots, WireError, WireGame, proto, slot_from_wire, slot_to_wire};

/// Longer than any word of any language (7 letters of at most 2 UTF-8 bytes each); a longer
/// string is refused before the game copies it.
const MAX_WORD_BYTES: usize = 32;

impl WireGame for WordHunt {
    /// Shape only: the game's `Settings::parse` checks the values.
    fn settings_from_wire(settings: &proto::GameSettings) -> Result<RawSettings, WireError> {
        match settings.game.as_ref().ok_or(WireError::Missing("game"))? {
            game_settings::Game::WordHunt(settings) => raw_settings_from_wire(settings),
            _ => Err(WireError::WrongGame),
        }
    }

    fn settings_to_wire(settings: &RawSettings) -> proto::GameSettings {
        proto::GameSettings {
            game: Some(game_settings::Game::WordHunt(raw_settings_to_wire(
                settings,
            ))),
        }
    }

    fn command_from_wire(command: &proto::GameCommand) -> Result<Command, WireError> {
        let game_command::Game::WordHunt(command) =
            command.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        match command
            .command
            .as_ref()
            .ok_or(WireError::Missing("command"))?
        {
            word_hunt_command::Command::Guess(guess) => {
                Ok(Command::Guess(bounded_word(&guess.word)?))
            }
            word_hunt_command::Command::GiveUp(_) => Ok(Command::GiveUp),
        }
    }

    fn command_to_wire(command: &Command) -> proto::GameCommand {
        let command = match command {
            Command::Guess(word) => {
                word_hunt_command::Command::Guess(proto::WordHuntGuess { word: word.clone() })
            }
            Command::GiveUp => word_hunt_command::Command::GiveUp(proto::WordHuntGiveUp {}),
        };
        proto::GameCommand {
            game: Some(game_command::Game::WordHunt(proto::WordHuntCommand {
                command: Some(command),
            })),
        }
    }

    fn world_event_from_wire(event: &proto::GameWorldEvent) -> Result<WorldEvent, WireError> {
        let game_world_event::Game::WordHunt(event) =
            event.game.as_ref().ok_or(WireError::Missing("game"))?
        else {
            return Err(WireError::WrongGame);
        };
        match event.event.as_ref().ok_or(WireError::Missing("event"))? {
            word_hunt_world_event::Event::AnswerChosen(chosen) => {
                Ok(WorldEvent::AnswerChosen(bounded_word(&chosen.word)?))
            }
            word_hunt_world_event::Event::FirstPlayerChosen(chosen) => {
                Ok(WorldEvent::FirstPlayerChosen(slot_from_wire(chosen.slot)?))
            }
        }
    }

    fn world_event_to_wire(event: &WorldEvent) -> proto::GameWorldEvent {
        let event = match event {
            WorldEvent::AnswerChosen(word) => {
                word_hunt_world_event::Event::AnswerChosen(proto::WordHuntAnswerChosen {
                    word: word.clone(),
                })
            }
            WorldEvent::FirstPlayerChosen(slot) => {
                word_hunt_world_event::Event::FirstPlayerChosen(proto::WordHuntFirstPlayerChosen {
                    slot: slot_to_wire(*slot),
                })
            }
        };
        proto::GameWorldEvent {
            game: Some(game_world_event::Game::WordHunt(
                proto::WordHuntWorldEvent { event: Some(event) },
            )),
        }
    }

    fn view_to_wire(view: &View) -> proto::GameView {
        proto::GameView {
            game: Some(game_view::Game::WordHunt(proto::WordHuntView {
                settings: Some(raw_settings_to_wire(&view.settings.to_raw())),
                phase: phase_to_wire(view.phase).into(),
                guesses: view.guesses.iter().map(scored_guess_to_wire).collect(),
                keyboard: view.keyboard.iter().map(key_hint_to_wire).collect(),
                answer: view.answer.clone(),
                turn_slot: view.turn.map(slot_to_wire),
                winner_slot: view.winner.map(slot_to_wire),
                guess_limit: view.guess_limit,
            })),
        }
    }
}

fn bounded_word(word: &str) -> Result<String, WireError> {
    if word.len() > MAX_WORD_BYTES {
        return Err(WireError::OutOfRange("word"));
    }
    Ok(word.to_owned())
}

fn raw_settings_from_wire(settings: &proto::WordHuntSettings) -> Result<RawSettings, WireError> {
    let language = match proto::WordHuntLanguage::try_from(settings.language) {
        Ok(proto::WordHuntLanguage::Russian) => Language::Russian,
        Ok(proto::WordHuntLanguage::English) => Language::English,
        Ok(proto::WordHuntLanguage::Unspecified) => return Err(WireError::Missing("language")),
        Err(_) => return Err(WireError::UnknownEnumValue("language")),
    };
    let mode = match proto::WordHuntPuzzleMode::try_from(settings.mode) {
        Ok(proto::WordHuntPuzzleMode::Unlimited) => PuzzleMode::Unlimited,
        Ok(proto::WordHuntPuzzleMode::Daily) => PuzzleMode::Daily,
        Ok(proto::WordHuntPuzzleMode::Unspecified) => return Err(WireError::Missing("mode")),
        Err(_) => return Err(WireError::UnknownEnumValue("mode")),
    };
    Ok(RawSettings {
        language,
        word_length: settings.word_length,
        max_guesses: settings.max_guesses,
        hard_mode: settings.hard_mode,
        mode,
        day: settings.day,
    })
}

fn raw_settings_to_wire(settings: &RawSettings) -> proto::WordHuntSettings {
    proto::WordHuntSettings {
        language: match settings.language {
            Language::Russian => proto::WordHuntLanguage::Russian,
            Language::English => proto::WordHuntLanguage::English,
        }
        .into(),
        word_length: settings.word_length,
        max_guesses: settings.max_guesses,
        hard_mode: settings.hard_mode,
        mode: match settings.mode {
            PuzzleMode::Unlimited => proto::WordHuntPuzzleMode::Unlimited,
            PuzzleMode::Daily => proto::WordHuntPuzzleMode::Daily,
        }
        .into(),
        day: settings.day,
    }
}

fn mark_to_wire(mark: LetterMark) -> proto::WordHuntLetterMark {
    match mark {
        LetterMark::Absent => proto::WordHuntLetterMark::Absent,
        LetterMark::Present => proto::WordHuntLetterMark::Present,
        LetterMark::Correct => proto::WordHuntLetterMark::Correct,
    }
}

fn phase_to_wire(phase: Phase) -> proto::WordHuntPhase {
    match phase {
        Phase::Choosing => proto::WordHuntPhase::Choosing,
        Phase::Guessing => proto::WordHuntPhase::Guessing,
        Phase::Solved => proto::WordHuntPhase::Solved,
        Phase::Failed => proto::WordHuntPhase::Failed,
        Phase::Abandoned => proto::WordHuntPhase::Abandoned,
        Phase::Forfeited => proto::WordHuntPhase::Forfeited,
    }
}

fn scored_guess_to_wire(guess: &ScoredGuess) -> proto::WordHuntScoredGuess {
    proto::WordHuntScoredGuess {
        word: guess.word.text(),
        marks: guess
            .marks
            .iter()
            .map(|mark| i32::from(mark_to_wire(*mark)))
            .collect(),
        by_slot: slot_to_wire(guess.by),
    }
}

fn key_hint_to_wire(hint: &KeyHint) -> proto::WordHuntKeyHint {
    proto::WordHuntKeyHint {
        letter: hint.letter.to_string(),
        mark: mark_to_wire(hint.mark).into(),
    }
}

impl WireBots for WordHunt {
    fn bot_kind_from_wire(kind: &proto::BotKind) -> Result<BotKind, WireError> {
        let kind = match kind.game.as_ref().ok_or(WireError::Missing("game"))? {
            proto::bot_kind::Game::WordHunt(kind) => *kind,
            _ => return Err(WireError::WrongGame),
        };
        match proto::WordHuntBotKind::try_from(kind) {
            Ok(proto::WordHuntBotKind::Easy) => Ok(BotKind::Easy),
            Ok(proto::WordHuntBotKind::Medium) => Ok(BotKind::Medium),
            Ok(proto::WordHuntBotKind::Hard) => Ok(BotKind::Hard),
            Ok(proto::WordHuntBotKind::Unspecified) => Err(WireError::Missing("bot_kind")),
            Err(_) => Err(WireError::UnknownEnumValue("bot_kind")),
        }
    }

    fn bot_kind_to_wire(kind: BotKind) -> proto::BotKind {
        let kind = match kind {
            BotKind::Easy => proto::WordHuntBotKind::Easy,
            BotKind::Medium => proto::WordHuntBotKind::Medium,
            BotKind::Hard => proto::WordHuntBotKind::Hard,
        };
        proto::BotKind {
            game: Some(proto::bot_kind::Game::WordHunt(kind.into())),
        }
    }
}

#[cfg(test)]
#[path = "word_hunt_tests.rs"]
mod tests;
