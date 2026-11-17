use mg_core::conformance::{self, Conformance, Setup};
use mg_core::{BotSeat, CoreRng, HasBots, PlayerSpec, Slot};

use crate::{
    BotKind, Command, Language, MaxGuesses, PuzzleMode, RawSettings, WordHunt, WordLength,
};

const HUMAN: PlayerSpec = PlayerSpec::HUMAN;
const BOT: PlayerSpec = PlayerSpec::BOT;

fn raw(language: Language, word_length: u32, mode: PuzzleMode) -> RawSettings {
    RawSettings {
        language,
        word_length,
        mode,
        // Only the daily puzzle keeps a day; anything else would not round-trip.
        day: if mode == PuzzleMode::Daily { 20_000 } else { 0 },
        ..RawSettings::default()
    }
}

fn solo(settings: RawSettings) -> Setup<WordHunt> {
    Setup::new(settings, vec![PlayerSpec::HUMAN])
}

impl Conformance for WordHunt {
    fn sample_setups() -> Vec<Setup<Self>> {
        vec![
            solo(RawSettings::default()),
            solo(raw(
                Language::Russian,
                WordLength::MIN,
                PuzzleMode::Unlimited,
            )),
            solo(raw(
                Language::English,
                WordLength::MAX,
                PuzzleMode::Unlimited,
            )),
            solo(raw(Language::Russian, 5, PuzzleMode::Daily)),
            solo(RawSettings {
                hard_mode: true,
                max_guesses: MaxGuesses::MAX,
                ..raw(Language::Russian, 6, PuzzleMode::Unlimited)
            }),
            solo(RawSettings {
                max_guesses: MaxGuesses::MIN,
                ..raw(Language::English, 5, PuzzleMode::Daily)
            }),
            Setup::new(RawSettings::default(), vec![HUMAN, HUMAN]),
            Setup::new(
                raw(Language::Russian, 4, PuzzleMode::Unlimited),
                vec![HUMAN, BOT],
            ),
            Setup::new(
                RawSettings {
                    hard_mode: true,
                    ..raw(Language::English, 6, PuzzleMode::Unlimited)
                },
                vec![BOT, HUMAN, BOT, BOT],
            ),
            // The bot arena: bots alone.
            Setup::new(RawSettings::default(), vec![BOT, BOT]),
        ]
    }

    fn invalid_setups() -> Vec<Setup<Self>> {
        let english = |word_length| raw(Language::English, word_length, PuzzleMode::Unlimited);
        vec![
            solo(english(0)),
            solo(english(WordLength::MIN - 1)),
            solo(english(WordLength::MAX + 1)),
            solo(english(u32::MAX)),
            solo(RawSettings {
                max_guesses: MaxGuesses::MIN - 1,
                ..RawSettings::default()
            }),
            solo(RawSettings {
                max_guesses: u32::MAX,
                ..RawSettings::default()
            }),
            solo(raw(Language::Russian, 6, PuzzleMode::Daily)),
            solo(RawSettings {
                day: u32::MAX,
                ..raw(Language::English, 5, PuzzleMode::Daily)
            }),
            Setup::new(RawSettings::default(), vec![]),
            Setup::new(RawSettings::default(), vec![HUMAN; 5]),
            Setup::new(
                raw(Language::English, 5, PuzzleMode::Daily),
                vec![HUMAN, BOT],
            ),
        ]
    }

    /// Bot levels by seat, so every level plays.
    fn bots_for(players: &[PlayerSpec]) -> Vec<BotSeat<Self>> {
        const LEVELS: [BotKind; 4] = [
            BotKind::Easy,
            BotKind::Medium,
            BotKind::Hard,
            BotKind::Medium,
        ];
        Slot::all(players.len())
            .zip(LEVELS)
            .filter(|(slot, _)| {
                players
                    .get(slot.index())
                    .is_some_and(|spec| !spec.is_human())
            })
            .map(|(slot, kind)| BotSeat {
                slot,
                bot: WordHunt::make_bot(kind),
                budget_nodes: WordHunt::default_bot_budget(kind),
            })
            .collect()
    }

    /// Mostly well-formed guesses (random letters of the right length), sometimes the wrong
    /// length or a stray character.
    fn arbitrary_command(game: &Self, _slot: Slot, rng: &mut CoreRng) -> Command {
        if rng.below(40) == 0 {
            return Command::GiveUp;
        }
        let view = mg_core::Game::view(game, mg_core::Viewer::Player(Slot::new(0)));
        let letters: Vec<char> = view.settings.language.alphabet().collect();
        let mut length = usize::from(view.settings.word_length().to_u8());
        match rng.below(10) {
            0 => length += 1,
            1 => length = length.saturating_sub(1),
            _ => {}
        }
        let mut text: String = (0..length)
            .filter_map(|_| rng.pick(&letters).copied())
            .collect();
        if rng.below(20) == 0 {
            text.push('1');
        }
        Command::Guess(text)
    }
}

#[test]
fn passes_conformance() {
    conformance::run_all::<WordHunt>(20);
}

/// Pins the rules: if this value changes, a rule changed — update it deliberately.
#[test]
fn scripted_hash_is_pinned() {
    let setup = solo(RawSettings::default());
    let hash = conformance::scripted_hash(&setup, 42);
    assert_eq!(hash, 0x1470_156F_07C6_9754);
}
