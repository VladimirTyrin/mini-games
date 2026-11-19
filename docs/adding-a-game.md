# Adding a game

This is the recipe for people and coding agents. Read `AGENTS.md` first (rules and
invariants), then `docs/architecture.md` for how lockstep works.

## 1. Scaffold

```bash
cargo xtask new-game connect_four --title "Connect Four"
```

It creates a game that already works end to end (a placeholder "reach the target" race):

| Where | What |
|---|---|
| `crates/games/connect-four/` | game crate: `Game` impl, settings, command, world event, view, tests |
| `proto/minigames/v2/connect_four.proto` | `ConnectFourSettings/Command/WorldEvent/View` |
| `crates/protocol/src/games/connect_four.rs` | `WireGame` and `WireRefusal` mapping |
| `crates/registry/src/games.rs` | one line in the game list (all dispatch is generated from it) |
| `proto/minigames/v2/game.proto` | one case in each envelope oneof |
| `proto/minigames/v2/refusal.proto` | one case in `Refusal` for the game's `ConnectFourRefusal` |
| `crates/protocol/src/refusal_english.rs` (and its test) | the game's case of the English lookup |
| `client/src/games/connect-four/` | plugin: settings form, board, `ConnectFourHelp.en.vue` / `.ru.vue` |
| `client/src/i18n/{en,ru}/games/connect-four.ts` | the game's texts, registered in `i18n/{en,ru}/index.ts` |
| `client/src/i18n/{en,ru}/refusals.ts`, `client/src/util/refusal-text.ts` | words for `ConnectFourRefusal` |
| `client/push-texts.json` | the invitation's body, `push_invite_body_connect_four` |

The Russian title, the push text's Russian game name and the help pages start as placeholders
to rewrite (see [localization.md](localization.md)).

Check that it runs before changing anything:

```bash
cargo test -p mg-connect-four
cd client && npm run dev    # Home → Connect Four, plays locally with no server
```

## 2. Rules (`crates/games/<game>`)

Every game crate has the same role-based layout, so you know where to look in any of them:

| Path | What lives there |
|---|---|
| `lib.rs` | module declarations and the crate's public re-exports, nothing else |
| `game.rs` | the game struct, `impl Game` and, with bots, `impl HasBots` |
| `settings/` | `Settings` (parsed), `RawSettings` (unchecked), option enums, limit types |
| `model/` | domain values and state pieces: points, boards, cells, ids, scores |
| `rules/` | pure rule logic and its outcomes: moves, win/end detection, end reasons, phase, rule constants |
| `io/` | what enters the game: `Command`, `WorldEvent` |
| `view/` | `View` and types that exist only to be shown (animation hints, last action, end-of-game stats) |
| `bots/` | bot kinds and bot implementations (games with bots only) |
| `tests/` | unit and conformance tests |

Each directory's `mod.rs` only declares its files and re-exports what the rest of the crate (or,
via `lib.rs`, other crates) uses; one type per file. A type the rules consult belongs to
`model/` even when the view also shows it.

Replace the sample rules. Keep the shape:

- **Settings** — parse, don't validate. Two types:
  - `RawSettings`: plain `u32`s and enums, field for field the wire message; what clients,
    replay files and forms carry. Unchecked.
  - `Settings`: implements `mg_core::GameSettings`; valid by construction. A numeric range is
    a type, `pub type FieldSize = Capped<3, 20>;`, used for the field. A rule a range cannot
    express gets its own type with a fallible constructor (2048's `Target`: a power of two);
    a rule between fields keeps the fields private (TicTacToe's win length fits the board).
  - `GameSettings::parse(raw) -> Result<Settings, SettingsError>` is the only way in: one
    `Capped::new(raw.x).map_err(SettingsError::at("x"))?` per field (the wire field name, which
    the form shows the error under), then the rules between fields. No `validate()` anywhere.
  - `GameSettings::ranges(&raw)` publishes the bounds to the UI from the same types
    (`FieldSize::range("field_width")`); a bound may follow another field, and `raw` may be
    invalid while a form is edited.
  - `Default` for `Settings` uses `Capped::constant::<15>()` (an out-of-range default fails the
    build); `RawSettings::default()` is `Settings::default().to_raw()`.
  - `Game::new(&Settings, players)` gets parsed settings and only checks what depends on the
    players. It may rely on the types: `FieldSize::to_u8()` is lossless, checked at compile time.
- **Command** — what a seat can do. `apply_command` validates and applies atomically; a
  rejected command must not change state.
- **Refusal** — `rules/refusal.rs`, `type Refusal = Refusal;`: why the game refuses a command
  or a world event, returned as `CommandError::Illegal(Refusal::…)`. No rejection carries a
  string. One variant per reason a player can meet in the real UI (`MustCapture`,
  `AlreadyTried`): it travels to the client as an enum and the client words it in the player's
  language. Then, grouped under a comment, the input no honest client or authority sends (a
  world event that does not fit, a cell off the board, a seat that does not exist): those
  travel as `COMMON_REFUSAL_MALFORMED` and clients show a generic "not allowed"; when in doubt,
  give a reason a wire value. `impl EnglishText` gives each variant's English (logs, bot
  clients, `mg_protocol::refusal_english`) and `Display` writes it. Common reasons are
  `CommandError`'s own (`GameOver`, `NotYourTurn`, `NotPlaying`, `Busy`); the core's own
  misuse is `CommandError::Malformed(ProtocolMisuse)`. A nested reason type
  (Word Hunt's `WordError`) converts into `Refusal` with `From`, so the wording stays in one
  place.
  A setting rule a range cannot express uses a generic `mg_core::SettingRule` in
  `SettingProblem::RuleBroken`; add a kind there (and to `SettingRule` in `refusal.proto`) if
  none fits.
- **WorldEvent** — every random outcome (dealt cards, spawned food, who starts). Produce them
  in `roll_world(&self, rng)`, apply them in `apply_world`. `apply_*` and `step` never see an
  RNG — the type system enforces it.
- **Secret** — `type Secret = ();` unless the game hides something from every player (a word to
  guess). Then the secret type holds it.
  - `roll_secret(&self, &mut secret, rng)` chooses it and answers from it as events (the marks
    of a guess).
  - `reveal(&self, &secret)` sends it as events once the game is over; `apply_world` must accept
    them on a finished game.
  - The game state, its hash and its view never hold the secret before the reveal. See Word Hunt.
  - Seats that add to the secret themselves (a fleet to place) send a private command: give
    the secret type a `GameSecret::Command`, apply it in `apply_secret_command` (validated
    against the public state, so a client's mirror can check it with an empty secret), say
    when it is due in `awaits_secret`, offer `default_secret_command` for a seat out of time, and
    return the seat's own share in `seat_secret`. Wire it with `GameSecretCommand` /
    `GameSeatSecret` and `WireGame::secret_command_*` / `seat_secret_to_wire`. Bots answer it in
    `Bot::decide_secret`. See Sea Battle.
- **Clock** — `TurnBased` (one frame per accepted move) or `Realtime { tick_ms }` (implement
  `step`). Real-time input that must wait for the next tick goes into a small per-seat queue
  inside the state (see Snake, Stack Attack).
- **drop_player** — a seat leaves for good (disconnect budget spent, left). Decide what
  it means: forfeit, removed piece, game over.
- **has_left_play** (optional) — a seat that is out while the game goes on without it (gave up
  in a group): the server then does not pause the match when its player's connection drops.
- **status / awaiting / view / hash_state** — `hash_state` must cover all gameplay state in a
  fixed order.

Determinism checklist (lockstep breaks silently otherwise):

- [ ] no `HashMap`/`HashSet` iteration, no floats, no clocks, no global RNG, no IO or logging;
- [ ] `usize` only for indexing; lengths go into the hash via `hasher.length(..)`;
- [ ] nothing reachable from input panics (`unwrap`, `expect`, indexing are denied by lints);
- [ ] every allocation is bounded by validated settings.

Bots: implement `mg_core::Bot` and `HasBots`, measure work with `Budget` (not time), and change
the registry entry from `no_bots` to `bots`. A bot must return a legal move on its turn.

## 3. Tests

- `tests/conformance.rs` implements `Conformance`: rich `sample_setups` (edge sizes, every mode,
  bot seats), `invalid_setups` (0, max+1, `u32::MAX`, broken rules between fields, wrong seat
  counts), `arbitrary_command` biased towards legal moves, `check_invariants`. Setups hold
  `RawSettings`; the suite parses them like any boundary and checks that they round-trip.
- Pin the golden hash: a scaffolded game starts with `GOLDEN_HASH = 0`, so
  `scripted_hash_is_pinned` fails and prints the value to copy there. It must only change when
  you change a rule on purpose.
- One test per rule, testing behaviour through `Authority`/`Game`, not private helpers.
- CI also runs the game tests on `wasm32-wasip1`, so the golden hash proves native and WASM
  agree.

## 4. Wire (`proto` + `crates/protocol`)

- Prefix every message and enum with the game name (one shared package); enum zero values are
  `<GAME>_<ENUM>_UNSPECIFIED`. `cd client && npx buf lint ../proto` must pass.
- The game's player-facing refusals have a mirror `enum <Game>Refusal` in its proto file and a
  case in `Refusal` (`refusal.proto`). `impl WireRefusal for Refusal` in
  `crates/protocol/src/games/<game>.rs` maps every variant with an exhaustive `match`, either to
  its wire value or to `return malformed_refusal()`, so a new variant forces that choice; its
  `refusal_from_wire` maps the wire values back (the English lookup and its test use it). The
  server and the WASM core send it in `ErrorResponse.refusal` / `refusalOf`; the client's
  dictionaries word each value. A game whose refusals are all malformed input (Stack Attack)
  has no proto enum and no case in `Refusal`.
- `*_from_wire` checks the shape (right game, required fields, known enum values) and returns
  `WireError`; it never clamps or panics. Settings come out as `RawSettings`: ranges and other
  game rules are the game's (`GameSettings::parse`, called by `mg_registry::parse_settings`).
- The view is what the board renders: include stable ids for anything that moves, and what
  happened last (for animations) so the UI never re-implements rules.
- Add round-trip tests and one end-to-end test (authority → wire → JSON → wire → `Match`, equal
  hashes); the other games have examples.

## 5. UI (`client/src/games/<game>`)

- `index.ts` — title, icon, tagline, `describeSettings`, `botLabel`, `controls` (a keyboard and a touch hint),
  `Help` and optionally `describeEnd` (why the match ended, shown with the result). Every text
  comes from the game's dictionary, `t.games.<game>` (getters for the plugin's own fields, so
  they follow a language switch).
- Texts — `client/src/i18n/en/games/<game>.ts` and `ru/games/<game>.ts`, added to `games` in
  both `index.ts` under the plugin's `settingsCase`; the game's proto `<Game>Refusal`, if it has
  one, gets a record in `en/refusals.ts` and `ru/refusals.ts` and a case in
  `util/refusal-text.ts`. The scaffold's client files still carry English placeholders: move them
  into the dictionary (the lint rule fails on text in a template).
- `<Game>Help.en.vue` and `<Game>Help.ru.vue` — "How to play", one page per language
  (`Help: { en: HelpEn, ru: HelpRu }`; a missing one does not compile): the goal and rules
  written out whole as `h3` sections with `p`/`ul` (the `HowToPlay` sheet styles them and adds
  the controls). Numbers come from the game's constants and names of buttons or settings from the
  dictionary; the rest is plain text in the page (see [localization.md](localization.md)). It
  opens from the "?" in the header on the setup screen, in the lobby and in a match; a local
  real-time match pauses meanwhile. A game whose rules are not obvious sets
  `opensHelpFirstTime` (Numbers Match): the help then opens by itself before the first match on
  the device.
- Settings form — rows from `components/form` (`StepperField`, `SegmentedField`, `ChipsField`
  presets, `MoreOptions`) through `useGameSettingsForm`: immutable updates, **no hardcoded
  limits** (`form.field('<wire name>')` binds the core's bounds and its error to a row; presets
  the core rejects are dropped). Optional `narrowScreenSettings` picks phone-friendly defaults.
- Board — render `controller.current.value.view`. Real-time games draw one tick behind and
  interpolate from `controller.previous` with `tickProgress`. Input through `useGameKeys`
  (filters repeats, modifiers, typing; `release`/`releaseAll` for held keys) and `useSwipe`;
  ignore input when `controller.mySlot === null` (observers, replays). Send real-time commands
  no faster than the game consumes them (Stack Attack's `WorkerInputPacer`): a backlog is lag.
- The result overlay waits for the ending to play out (`MatchController.resultRevealAt`: 1.2 s
  after the last tick's motion for real-time games, 0.8 s for turn-based), so the final crash or
  winning move is visible. Local real-time matches start after a 3-2-1 countdown.
- Mobile: fits portrait phones, large touch targets (on-screen buttons for games that need
  holding, ≥ 64 px), no page scroll on swipe.
- Respect `prefersReducedMotion()` (`render/reduced-motion.ts`) for decorative motion: squash,
  shake, particles.

## 6. Before the PR

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd client && npm run lint && npm run typecheck && npx vitest run && npm run build
```

Then play it: locally against bots, online in two browser tabs, and watch the saved replay.
Add the rule summary to `docs/` if players need one.
