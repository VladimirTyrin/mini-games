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
| `settings/` | `Settings`, its option enums and validated limit types |
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

- **Settings** — a plain struct with `Default` (what the UI offers first) and limit consts.
  `Game::new` validates everything, *before* allocating anything sized by a setting.
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
  bot seats), `invalid_setups` (0, max+1, `u32::MAX`, wrong seat counts), `arbitrary_command`
  biased towards legal moves, `check_invariants`.
- Pin the golden hash: run `scripted_hash` once and `assert_eq!` the value. It must only change
  when you change a rule on purpose.
- One test per rule, testing behaviour through `Authority`/`Game`, not private helpers.
- CI also runs the game tests on `wasm32-wasip1`, so the golden hash proves native and WASM
  agree.

## 4. Wire (`proto` + `crates/protocol`)

- Prefix every message and enum with the game name (one shared package); enum zero values are
  `<GAME>_<ENUM>_UNSPECIFIED`. `cd web-client && npx buf lint ../proto` must pass.
- `*_from_wire` validates ranges and enum values and returns `WireError`; it never clamps or
  panics. Leave game-rule validation to the game.
- The view is what the board renders: include stable ids for anything that moves, and what
  happened last (for animations) so the UI never re-implements rules.
- Add round-trip tests and one end-to-end test (authority → wire → JSON → wire → `Match`, equal
  hashes); the other games have examples.

## 5. UI (`web-client/src/games/<game>`)

- `index.ts` — title, icon, tagline, `describeSettings`, `botLabel`, `controlsHint`.
- Settings form — shared `NumberField`/`SelectField`, immutable updates, **no hardcoded
  limits** (the shell shows the core's validation message).
- Board — render `controller.current.value.view`. Real-time games draw one tick behind and
  interpolate from `controller.previous` with `tickProgress`. Input through `useGameKeys`
  (filters repeats, modifiers, typing) and `useSwipe`; ignore input when
  `controller.mySlot === null` (observers, replays).
- Mobile: fits portrait phones, large touch targets, no page scroll on swipe.

## 6. Before the PR

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd web-client && npm run lint && npm run typecheck && npx vitest run && npm run build
```

Then play it: locally against bots, online in two browser tabs, and watch the saved replay.
Add the rule summary to `docs/` if players need one.
