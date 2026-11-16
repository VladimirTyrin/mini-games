# mini-games — guide for contributors and coding agents

Small multiplayer games (Snake, TicTacToe, Stack Attack, 2048, Numbers Match). One deterministic
Rust core runs the rules everywhere: on the server, in the browser (WASM), in the Android app
(same WASM in a WebView), in network bots and in the local arena.

Design and status: `docs/refactoring-plan.md`. Architecture: `docs/architecture.md`.
Adding a game: `docs/adding-a-game.md`.

## Layout

- `crates/core` (`mg-core`) — `Game`/`Bot` traits, `Authority` (seals lockstep frames), `Match`
  (applies them), pinned RNG, state hashing, conformance suite.
- `crates/games/*` — one crate per game: rules + bots, pure Rust, depends only on `mg-core`.
- `proto/` — wire contract (protobuf; JSON mapping on the WebSocket, binary in replay files).
- `web-client/` — Vue 3 + Pinia + Tailwind; `web-client/android` — Capacitor app.
- `server/` — legacy v1 server, excluded from the workspace, kept only as porting reference.
- `deploy.sh`, `deploy/` — production deploy (`./deploy.sh`).

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

## Coding rules

- **Comments only where they add something**: non-obvious contracts, synchronization and ordering
  subtleties, optimizations, the reason behind a surprising decision. Never restate what the code
  or the name already says; no boilerplate doc comments on obvious items.
- **Domain types over primitives** where they carry meaning (`Slot`, `Tick`, `Point`, `TileId`,
  `Direction`...). Primitives only at boundaries. Convert unchecked input into a validated type
  and expose operations that rely on its invariants only through that type.
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
