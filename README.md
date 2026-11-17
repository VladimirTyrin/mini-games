# Mini Games

Small games to play with friends or against bots: **Snake**, **Tic-Tac-Toe** (N in a row),
**Stack Attack** (co-op crates), **2048**, **Numbers Match**, **Word Hunt** (guess a Russian
or English word; word of the day or unlimited), **Checkers** (Russian draughts) and **Sea
Battle** (place a fleet unseen, then sink the other one).

## Play

- **In the browser:** <https://braintvsminigames.xyz>
- **On Android:** download the app —
  [mini-games.apk](https://braintvsminigames.xyz/downloads/mini-games.apk) (or the APK from the
  latest [GitHub release](https://github.com/VladimirTyrin/mini-games/releases)). Android asks to
  allow installing from your browser the first time. The app updates itself afterwards.

Games against bots run entirely on your device and work offline. **Play online** opens lobbies:
play with friends, chat, watch matches and replays. Sign in as a guest or with Google.

## How it works

One deterministic Rust core runs the rules everywhere — on the server, in the browser and in the
Android app (as WebAssembly), in network bots and in the bot arena. Matches are lockstep: the
server (or the device, offline) seals each tick and every client replays the same frames, so
replays are just the frame log.

- `crates/` — the core, the games, protocol, server, WASM bindings, bot arena, bot SDK and a C ABI
- `client/` — Vue web client, also packaged as the Android app (Capacitor)
- `network-bots/` — C# bots (rules from the core through the C ABI)
- `proto/` — the wire protocol (protobuf)

More: [architecture](docs/architecture.md), [protocol](docs/protocol.md),
[adding a game](docs/adding-a-game.md), [Android app and releases](docs/android.md),
[bots](docs/bots.md). Contributors and coding agents start with [AGENTS.md](AGENTS.md).

## Develop

```bash
cargo test --workspace                 # core, games, server
cd client && npm ci && npm run dev # web client on http://localhost:5173 (local games need no server)
cargo run -p mg-server -- --listen 127.0.0.1:5000 --static-dir client/dist   # online play
```

Toolchain: Rust (stable), Node.js LTS, `wasm-bindgen-cli`, `binaryen` (`wasm-opt`). See
[AGENTS.md](AGENTS.md) for the checks CI runs.

## Privacy

See the [privacy policy](https://braintvsminigames.xyz/privacy.html).
