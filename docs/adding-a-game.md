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
| `crates/protocol/src/games/connect_four.rs` | `WireGame` mapping |
| `crates/registry/src/games.rs` | one line in the game list (all dispatch is generated from it) |
| `proto/minigames/v2/game.proto` | one case in each envelope oneof |
| `web-client/src/games/connect-four/` | plugin: settings form + board |

Check that it runs before changing anything:

```bash
cargo test -p mg-connect-four
cd web-client && npm run dev    # Home → Connect Four, plays locally with no server
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
- **WorldEvent** — every random outcome (dealt cards, spawned food, who starts). Produce them
  in `roll_world(&self, rng)`, apply them in `apply_world`. `apply_*` and `step` never see an
  RNG — the type system enforces it.
- **Clock** — `TurnBased` (one frame per accepted move) or `Realtime { tick_ms }` (implement
  `step`). Real-time input that must wait for the next tick goes into a small per-seat queue
  inside the state (see Snake, Stack Attack).
- **drop_player** — a seat leaves for good (disconnect budget spent, left, kicked). Decide what
  it means: forfeit, removed piece, game over.
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
- Pin the golden hash: run `scripted_hash` once and `assert_eq!` the value. It must only change
  when you change a rule on purpose.
- One test per rule, testing behaviour through `Authority`/`Game`, not private helpers.
- CI also runs the game tests on `wasm32-wasip1`, so the golden hash proves native and WASM
  agree.

## 4. Wire (`proto` + `crates/protocol`)

- Prefix every message and enum with the game name (one shared package); enum zero values are
  `<GAME>_<ENUM>_UNSPECIFIED`. `cd web-client && npx buf lint ../proto` must pass.
- `*_from_wire` checks the shape (right game, required fields, known enum values) and returns
  `WireError`; it never clamps or panics. Settings come out as `RawSettings`: ranges and other
  game rules are the game's (`GameSettings::parse`, called by `mg_registry::parse_settings`).
- The view is what the board renders: include stable ids for anything that moves, and what
  happened last (for animations) so the UI never re-implements rules.
- Add round-trip tests and one end-to-end test (authority → wire → JSON → wire → `Match`, equal
  hashes); the other games have examples.

## 5. UI (`web-client/src/games/<game>`)

- `index.ts` — title, icon, tagline, `describeSettings`, `botLabel`, `controls` (a keyboard and a touch hint), and
  optionally `describeEnd` (why the match ended, shown with the result).
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
cd web-client && npm run lint && npm run typecheck && npx vitest run && npm run build
```

Then play it: locally against bots, online in two browser tabs, and watch the saved replay.
Add the rule summary to `docs/` if players need one.
