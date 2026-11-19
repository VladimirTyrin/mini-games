# mini-games — guide for contributors and coding agents

Small multiplayer games (Snake, TicTacToe, Stack Attack, 2048, Numbers Match, Word Hunt, Checkers, Sea Battle). One deterministic
Rust core runs the rules everywhere: on the server, in the browser (WASM), in the Android app
(same WASM in a WebView), in network bots and in the local arena.

Design and status: `docs/refactoring-plan.md`. Architecture: `docs/architecture.md`.
Adding a game: `docs/adding-a-game.md`. Android app and releases: `docs/android.md`.
Bots (arena, Rust bot SDK, C ABI, C# bots): `docs/bots.md`. Word Hunt rules and word lists: `docs/word-hunt.md`. Checkers rules and bots: `docs/checkers.md`. Sea Battle rules, hidden fleets and bots: `docs/sea-battle.md`.
Push notifications (FCM, Web Push): `docs/push-notifications.md`. Localization (English, Russian): `docs/localization.md`.
Known gaps left for later: `docs/tech_debt.md`.

## Layout

- `crates/core` (`mg-core`) — `Game`/`Bot` traits, `Authority` (seals lockstep frames), `Match`
  (applies them), pinned RNG, state hashing, conformance suite.
- `crates/games/*` — one crate per game: rules + bots, pure Rust, depends only on `mg-core`.
  Same skeleton in every game: `game.rs`, `settings/`, `model/`, `rules/`, `io/`, `view/`,
  `bots/`, `tests/` (see `docs/adding-a-game.md`).
- `proto/` — wire contract (protobuf; JSON mapping on the WebSocket, binary in replay files).
- `client/` — Vue 3 + Pinia + Tailwind; `client/android` — Capacitor app.
- `deploy.sh`, `deploy/` — production deploy (`./deploy.sh`, see `docs/deploy.md`). Deploy right
  after pushing, CI runs alongside; `./deploy.sh --web-only` releases a client-only change in
  well under a minute (it refuses when the server or the app's native shell would have to change).

## Commands

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p mg-core --target wasm32-unknown-unknown
```

Zero warnings: clippy and rustc warnings fail CI.

## Core invariants (lockstep)

- Game state transitions are pure: no clocks, no global RNG, no `HashMap`/`HashSet` iteration, no
  floats, no IO or logging inside game crates.
- Randomness only in `Game::roll_world` (authority side) and in bots; it reaches clients as
  `WorldEvent`s. Clients never know the seed.
- A rejected command or world event leaves the state untouched.
- `hash_state` covers every gameplay-relevant field in a fixed order.
- Untrusted input (settings, replay files, commands) is validated before anything is allocated
  from it; nothing reachable from input may panic (`unwrap`/`expect`/indexing are denied in game
  crates outside tests).
- Every game passes `mg_core::conformance::run_all` and pins a `scripted_hash` golden value.

## Screens

We want to look good on every screen size: not the same everywhere, but good everywhere. A layout
may depend on the screen's shape (portrait, landscape, nearly square), not only on its width. On
small screens give up secondary interface (Stack Attack's stats, say) to make the game field
larger; on big screens use the room instead of leaving it empty.

Check every UI change at least on these, in CSS pixels (what the browser sees: a 4K screen at Windows' 150% scaling is 2560×1440), in every language of the app (English and Russian). With `npm run dev` running (and the server, for the online screens):

- `node tools/screens/fit.mjs --screens <the screens the change touches>` finds clipped, ellipsized
  or spilling text and sideways scrolling at all of them, in both languages, without pictures; it
  exits with 1 when anything is found. Run it before committing a UI change; CI (`screens.yml`)
  runs all of it on every push to `client/`.
- `node tools/screens/shoot.mjs` screenshots every screen (or `--screens`, `--sizes`, `--langs`)
  into `screens-out/` (`all.html` shows them by screen, the languages side by side): to look at a
  layout, and to show it to the user.

Scenarios (`tools/screens/scenarios.mjs`) press buttons by `data-testid`, never by their text.

| Screen | Viewport |
|---|---|
| Small phone (iPhone SE) | 375×667, and 320×568 for the oldest |
| iPhone 15 Pro | 393×852 |
| Google Pixel 10a | about 412×915, also held sideways |
| iPad 2018 (9.7") | 768×1024 and 1024×768 |
| New laptop, 2K 16" (scaled 150%) | about 1707×1067, and its half (a window beside another): 853×1067 |
| 4K 27–31" monitor, scaled 150% | 2560×1440, and its half: 1280×1440 |

No text may be clipped, ellipsized or spill out of its control, and nothing may scroll sideways.
One exception: screens other than a game itself (home, setup, lobby, online, settings) need not
work on a phone held sideways. Dialogs over a game (How to play, the result) still do.

## Coding rules

- **Comments only where they add something**: non-obvious contracts, synchronization and ordering
  subtleties, optimizations, the reason behind a surprising decision. Never restate what the code
  or the name already says; no boilerplate doc comments on obvious items.
- **Domain types over primitives** where they carry meaning (`Slot`, `Tick`, `Point`, `TileId`,
  `Direction`...). Primitives only at boundaries.
- **Parse, don't validate.** Unchecked input becomes a typed value through a fallible
  constructor (`Settings::parse(raw) -> Result<Settings, SettingsError>`, `Capped::<1, 100>::new`)
  at the boundary; there is no separate `validate()` that callers may forget, and a value of the
  type is valid by construction. Ranges are types (`Capped<MIN, MAX>`), not checks scattered over
  the code, and the UI reads limits from the same constants. Operations that rely on an
  invariant take the parsed type, never the raw one.
- **Descriptive names** — names are free: `has_pending_inputs`, not `has_pending`; no `cmd`, `h`,
  `idx` in APIs.
- One type (struct/enum/trait) per file where practical; `mod.rs` only declares and re-exports.
- A signature must say what it returns: no anonymous tuples in return types or public fields
  (`-> Result<LobbySetup, SettingsRejection>`, not `-> Result<(GameKind, u8), String>`), never
  tuples of more than two elements. The sender/receiver pair of a `channel()` constructor is the
  accepted exception. A bool only when both values are obvious from its name; otherwise an enum.
- Errors are typed: no `Result<_, String>` in APIs; plain strings only at the outermost boundary
  (logs, user-facing messages).
- Keep the underlying cause when wrapping errors. No `let _ = result`.
- `.clone()`, `.to_string()`, `.to_owned()` need a reason; prefer borrowing, `&'static str`,
  `Arc<str>` for shared strings.
- Don't duplicate logic — move shared code into a shared module.
- Tests check observable behaviour, not implementation shape. No sleeps or polling in tests; use
  events, controlled clocks or test hooks. A bug fix comes with a test reproducing the defect.
- Update the docs when behaviour or configuration changes.
- **Privacy policy** (`client/public/privacy.html` and `privacy-ru.html`): a feature that
  collects, stores, shows to others or sends out any player data (a new table or field, a new kind
  of notification, a new third-party service) updates both pages and their "Last updated" dates in
  the same change.
- **Texts** (`docs/localization.md`): every text a player reads comes from the client's
  dictionaries (`client/src/i18n`, English and Russian; the lint rule
  `vue/no-bare-strings-in-template` guards templates). The server and the Rust core never send
  words for players: error codes, refusal enums, values; push notifications go out as keys
  (`client/push-texts.json`).
