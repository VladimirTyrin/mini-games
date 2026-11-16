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
3. rolls randomness (`Game::roll_world`) and applies it as `WorldEvent`s;
4. steps real-time games (`Game::step`);
5. seals a `Frame { tick, inputs, hash? }` — the hash every N frames and on the final frame.

Receivers (`Match`) apply frames in the same order and compare hashes. A mismatch or a gap
means desync: the client asks for a catch-up (header + compact log) and rebuilds.

Why: frames are tiny (usually empty), clients never learn the seed (bot competitions stay
fair), replays are just the frame log, and late joiners, observers and reconnects all reuse
the catch-up path.

Determinism rules live in `AGENTS.md`; `mg_core::conformance` enforces them per game, and
every game pins a `scripted_hash`.

## Server (`crates/server`)

Actors, no shared locks:

- **Connection** (one task per WebSocket): JSON decode, size and rate limits, heartbeat,
  sign-in. Outbound queue is bounded; a client that cannot keep up is disconnected (it may
  resume) instead of slowing others.
- **Hub** (one task): sessions, names, resume tokens, lobbies (explicit phases: open →
  in match → post match / replay). Handles one command at a time.
- **Match actor** (one task per match): owns the `DynAuthority`, ticks real-time games,
  runs bot searches on the blocking pool under a CPU semaphore, broadcasts each frame encoded
  once, handles SC2-style pauses (60 s budget per seat, drop vote after 20 s, 3 s countdown).

Authentication without a database: guests (if allowed), Google ID tokens verified against
Google's JWKS, bot keys from configuration; afterwards a stateless HMAC session token.

## Web client (`web-client`)

- `wasm/core.ts` — typed wrapper over `mg-wasm` (binary protobuf across the boundary).
- `net/session.ts` — WebSocket session: sign-in on every (re)connect, request ids, ping,
  backoff with jitter, resume token.
- `match/` — `MatchSource` (online / local worker / replay file) and `MatchController`
  (mirror, views for rendering, desync recovery). The match screen does not know which source
  it plays.
- `games/<id>/` — one plugin per game: settings form + board. Rules and limits come from the
  core; plugins only render and turn input into commands.

The Android app is the same web build inside Capacitor (built with `VITE_PLATFORM=android`, which
adds the native back button and Google sign-in); see `docs/android.md`.

## Bots (`crates/arena`, `crates/bot-sdk`, `crates/ffi`, `network-bots`)

Built-in bots run inside the authority. Everything else that plays reuses the same core
instead of re-implementing rules: `mg-arena` runs bot-vs-bot matches through the registry
authority, `mg-bot-sdk` is a Rust WebSocket client that mirrors server matches with the core,
and `mg-ffi` exposes local authorities and mirrors through a C ABI, which the C# bots in
`network-bots` load via P/Invoke. See `docs/bots.md`.
