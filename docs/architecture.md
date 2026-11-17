# Architecture

## Big picture

```
            ┌───────────────────────── one Rust core ─────────────────────────┐
            │ mg-core: Game/Bot traits, Authority, Match, RNG, state hashing   │
            │ crates/games/*: rules + bots (pure, deterministic)               │
            │ mg-protocol: proto types + domain↔wire mapping                   │
            │ mg-registry: type-erased access to every game                    │
            └──────────────┬───────────────────────┬──────────────────────────┘
                           │ native                │ wasm32 (mg-wasm)
                 ┌─────────▼────────┐    ┌─────────▼──────────────────────────┐
                 │ mg-server         │    │ web client (Vue) — browser & Android│
                 │ authority online  │    │ • MatchMirror: replays frames       │
                 └─────────▲────────┘    │ • LocalHost in a Web Worker:         │
                           │ WebSocket   │   authority for games without other  │
                           │ protobuf    │   humans (offline, no ping)          │
                           │ JSON        └─────────────────────────────────────┘
                           └──────────── web / Android clients, bots
```

## Lockstep

Every match has exactly one **authority**: the server for online games, the in-browser
`LocalHost` (Web Worker) for games without other humans, a test harness or the arena
elsewhere. It owns the only `CoreRng` (seeded from a CSPRNG, never sent anywhere).

Per frame (= one tick of a real-time game, or one accepted move of a turn-based one) the
authority:

1. applies human commands as they arrive (`Authority::submit`);
2. lets bots decide (`run_bots`);
3. settles its secret (`Game::roll_secret`), then rolls randomness (`Game::roll_world`), and
   applies both as `WorldEvent`s;
4. in the frame the game finishes, reveals the secret (`Game::reveal`) as `WorldEvent`s;
5. steps real-time games (`Game::step`);
6. seals a `Frame { tick, inputs, hash? }` — the hash every N frames and on the final frame.

Receivers (`Match`) apply frames in the same order and compare hashes. A mismatch or a gap
means desync: the client asks for a catch-up (header + compact log) and rebuilds.

Why: frames are tiny (usually empty), clients never learn the seed (bot competitions stay
fair), replays are just the frame log, and late joiners, observers and reconnects all reuse
the catch-up path.

**Secrets.** What no player may see until the end (Word Hunt's word) is the game's
`Game::Secret`. The authority keeps it beside the game, outside the state that clients replay
and hash. `roll_secret` is the only place that reads it, and it tells everyone only what the
rules make public: a guess's marks, say, never the word. At the end `reveal` sends the secret
itself, so replays are complete. Bots decide from the public game like everyone else, so they
cannot peek either.

Determinism rules live in `AGENTS.md`; `mg_core::conformance` enforces them per game, and
every game pins a `scripted_hash`.

Settings are parsed once at the boundary: the wire gives a game's `RawSettings`,
`GameSettings::parse` turns them into `Settings` whose ranges are `Capped<MIN, MAX>` types, and
`Game::new`, the `Authority` and every `Match` only ever see parsed settings. The bounds the UI
shows come from the same types.

## Server (`crates/server`)

Actors, no shared locks:

- **Connection** (one task per WebSocket): JSON decode, size and rate limits, heartbeat,
  sign-in. Outbound queue is bounded; a client that cannot keep up is disconnected (it may
  resume) instead of slowing others.
- **Hub** (one task): sessions, accounts (one session per account), resume tokens, lobbies (explicit phases: open →
  in match → post match / replay). Handles one command at a time.
- **Match actor** (one task per match): owns the `DynAuthority`, ticks real-time games,
  runs bot searches on the blocking pool under a CPU semaphore, broadcasts each frame encoded
  once. A turn-based bot thinks on a copy of the game (`Authority::begin_bot_turn`) while the
  actor goes on answering; its move is played when it comes back unless the match moved on
  meanwhile (then it thinks again), handles SC2-style pauses (60 s budget per seat, drop vote after 20 s, 3 s countdown).

- **Social actor** (one task): friends, friend links, push targets and invitations, kept in a
  SQLite database (`MG_DATABASE`, migrated at start by `PRAGMA user_version`; a database written
  by a newer server is refused). It asks the hub who is online and has the hub deliver to every
  device of an account; the hub only ever sends it fire-and-forget commands, so the two cannot
  deadlock. Push notifications go out from blocking tasks. See
  [push-notifications.md](push-notifications.md).

Authentication needs no database: guests (if allowed), Google ID tokens verified against
Google's JWKS, bot keys from configuration; afterwards a stateless HMAC session token. Players
are told apart by an opaque id (`PlayerId`; for accounts an HMAC of the account under the same
secret), never by display name, which may repeat.

## Web client (`client`)

- `wasm/core.ts` — typed wrapper over `mg-wasm` (binary protobuf across the boundary).
- `net/session.ts` — WebSocket session: sign-in on every (re)connect, request ids, ping,
  backoff with jitter, resume token.
- `match/` — `MatchSource` (online / local worker / replay file) and `MatchController`
  (mirror, views for rendering, desync recovery). The match screen does not know which source
  it plays.
- `games/<id>/` — one plugin per game: settings form + board. Rules and limits come from the
  core; plugins only render and turn input into commands. A board that wants to know whether
  an action is allowed, or what it leads to, asks the rules instead of repeating them:
  `controller.refusal(command)` (why the game would refuse it, in the rules' words, or `null`)
  and `controller.preview(command)` (refused / the game goes on / how it ends). Both run
  `Game::check_command` / `Game::preview_command` on the state on screen, which try the command
  on a copy; the conformance suite checks that the answer always matches applying it. The
  authority still decides, since the screen may be a frame behind.
- `components/form/` — the setup rows every settings form is built from (`StepperField`,
  `SegmentedField`, `ChipsField` presets, `MoreOptions`). `SettingsEditor` asks the WASM core
  for the numeric bounds (`settingRanges`, from each game's `GameSettings::ranges`) and for the
  verdict (`settingsProblem`: field + message), so a stepper stops at the real limit and an
  error appears under the row it belongs to.

The Android app is the same web build inside Capacitor (built with `VITE_PLATFORM=android`, which
adds the native back button and Google sign-in); see `docs/android.md`.

## Bots (`crates/arena`, `crates/bot-sdk`, `crates/ffi`, `network-bots`)

Built-in bots run inside the authority. Everything else that plays reuses the same core
instead of re-implementing rules: `mg-arena` runs bot-vs-bot matches through the registry
authority, `mg-bot-sdk` is a Rust WebSocket client that mirrors server matches with the core,
and `mg-ffi` exposes local authorities and mirrors through a C ABI, which the C# bots in
`network-bots` load via P/Invoke. See `docs/bots.md`.
