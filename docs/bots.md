# Bots

Every bot, wherever it runs, plays on the same Rust core as the server:

| What | Where | Rules from |
|------|-------|------------|
| Built-in bots (`minimax`, `random`, `efficient`) | `crates/games/*/src/bots` | the game crate; run by the authority (server, browser, arena) |
| Local tournaments | `crates/arena` (`mg-arena`) | registry authority |
| Rust network bots | `crates/bot-sdk` (`mg-bot-sdk`) | registry / typed mirror of the server's frames |
| Bots in other languages | `crates/ffi` (`mg-ffi`, library `mini_games`) | C ABI over authorities and mirrors |
| C# bots (ML, MCTS) | `network-bots` | `mg-ffi` via P/Invoke |

## Arena

```bash
cargo run --release -p mg-arena -- --game tictactoe --bots minimax,random --games 200 --seed 42
```

```text
tictactoe: 200 games, seed 42
seat  bot            wins  draws  losses  unfinished
#0    minimax         ...
#1    random          ...
average length: ... ticks
```

- `--bots` lists one built-in bot per seat in slot order (names: the game's bot kinds, e.g.
  `minimax`, `random` for TicTacToe, `efficient`, `random` for Snake).
- Results depend only on `--seed`: game `i` uses a seed derived from the base seed and `i`, so
  `--threads N` changes speed, not results.
- `--settings-json` takes protobuf JSON of `GameSettings`; the default is the game's own
  settings (15x15 TicTacToe, five in a row, which makes minimax slow). For a quick run:
  `--settings-json '{"tictactoe":{"fieldWidth":3,"fieldHeight":3,"winCount":3,"firstPlayer":"TIC_TAC_TOE_FIRST_PLAYER_MODE_RANDOM"}}'`.
- `--replays DIR` writes `.mgreplay` files of the games the first bot lost; open them in the web
  client's replay viewer.
- `--max-ticks` (default 100000) ends real-time games that never finish; they count as
  unfinished.

## Rust network bots (`mg-bot-sdk`)

A `BotSession` signs in with a bot key, hosts or joins a lobby and plays matches. It mirrors
every match with the core (the server's frames, catch-ups after a gap, desync or reconnect) and
asks a `Strategy` for commands once per new state.

Network bots cannot send private commands yet (Sea Battle's fleet): in Sea Battle the server places
a random fleet for them when the time to place runs out, so such a match starts after 90 s (see
[tech_debt.md](tech_debt.md)).

```rust
use mg_bot_sdk::{BotConfig, BotSession, LobbySetup, ViewBot, ViewStrategy};
use mg_core::Slot;
use mg_protocol::proto;

struct FirstFreeCell;

impl ViewStrategy for FirstFreeCell {
    fn decide(&mut self, view: &proto::GameView, me: Slot) -> Option<proto::GameCommand> {
        // Read the rendered view; return None when it is not your turn.
        todo!()
    }
}

let mut session = BotSession::connect(BotConfig::new("ws://127.0.0.1:5000/ws", key)).await?;
let header = session.host_match(&LobbySetup { /* settings, seats, built-in opponents */ }).await?;
let summary = session.play_match(header, &mut ViewBot(FirstFreeCell)).await?;
```

- `ViewStrategy` sees what a player sees (`GameView`); `ViewBot` adapts it to any game.
- `BuiltinBot<G>` plays with one of the game's own bots over the network (needs the concrete
  game type for its full state; `TypedMirror<G>` provides it).
- Implement `Strategy` directly for your own mirror type.
- Flows: `host_match` (create, add built-in bots, wait for every seat to be taken and ready,
  start), `create_lobby` + `start_when_ready`, `join_match(lobby_id)`, `wait_for_match`.
- `play_match` submits at most one command per state and waits until the server has
  sealed it into a frame before asking again (the server acknowledges a command before the
  frame with it arrives). Commands are not sent during a pause or the resume countdown.
- A dropped connection is resumed with the resume token (`ReconnectPolicy`: immediate retry,
  then exponential backoff); the server pushes a catch-up and the match continues.
  `ERROR_CODE_SIGNED_IN_ELSEWHERE` and server shutdown end the session.
- Only `ws://` URLs for now (no TLS in the SDK yet).

Example: `crates/bot-sdk/examples/network_bot.rs`.

```bash
MG_BOT_KEY=secret cargo run -p mg-bot-sdk --example network_bot -- \
    --url ws://127.0.0.1:5000/ws --game tictactoe --bot minimax --opponents random --matches 3
```

## Bot keys on the server

Network bots sign in with `Hello { bot_client { key } }`. The server knows only SHA-256 hashes:

```bash
# Make a key and its hash.
key=$(openssl rand -hex 24)
hash=$(printf %s "$key" | sha256sum | cut -d' ' -f1)
# Server: MG_BOT_KEYS=name:hash,name2:hash2 (or --bot-keys).
MG_BOT_KEYS="alpha:$hash" cargo run -p mg-server
```

The bot appears online under `name`; a second sign-in with the same key takes the session over
(the first connection gets `ERROR_CODE_SIGNED_IN_ELSEWHERE`). Removing a key from
`MG_BOT_KEYS`, or giving the name a new key, also revokes the session tokens issued for the old
one (a token names the key it was issued for).

## C ABI (`mg-ffi`)

`cargo build -p mg-ffi --release` produces `mini_games.dll` / `libmini_games.so` /
`libmini_games.dylib` (and a static library). Declarations: `crates/ffi/include/mini_games.h`.

- Messages cross as binary protobuf of `proto/minigames/v2` (compile the protos for your
  language).
- `MgAuthority`: a local match for any game (`mg_authority_new(MatchHeader, seed)`); seats with
  a `bot_kind` are played by the built-in bots (`mg_authority_run_bots`). Submit commands, seal
  frames, render views, get the result or a replay file. `mg_authority_clone` copies the state
  for search (it replays the match so far, so it costs time proportional to its length).
- `MgMirror`: the receiving side for network play: `mg_mirror_new(MatchHeader)`, then
  `mg_mirror_apply_frame` per `TickFrame`; on `MG_STATUS_DESYNC`, build a new mirror from the
  catch-up's header and `mg_mirror_apply_catch_up`. Also replays (`mg_mirror_from_replay_file`,
  `mg_mirror_advance_to`).
- Every call returns an `MgStatus`; `mg_last_error_message` explains a failure (per thread).
  Results come in library-allocated `MgBuffer`s that the caller frees with `mg_buffer_free`.
  Handles are freed with `*_free`, are not thread-safe, and panics never cross the boundary.
- `mg_abi_version` lets a caller refuse an incompatible library.

## C# bots (`network-bots`)

`dotnet build network-bots` restores the NuGet packages, compiles the protos from
`proto/minigames/v2` (Google.Protobuf + Grpc.Tools, no gRPC), runs
`cargo build -p mg-ffi --release` and copies the native library next to the output. Override
`-p:CargoCommand="cargo +1.100.0"` or `-p:CargoTargetDir=...`, or pass `-p:SkipNativeBuild=true`
and copy the library yourself. See `network-bots/README.md` for the command line.

- `Native/` — P/Invoke (`LibraryImport`) over `mini_games`: `LocalMatch` (authority) and
  `MatchMirror`.
- `TicTacToe/Core/TicTacToeEngine` — the local game for bots, tournaments and training; the
  rules (legal moves, wins, turn order) come from the core.
- `Network/` — `ClientWebSocket` with protobuf JSON, bot-key sign-in, resume.
- `TicTacToeRunner` — hosts a lobby against a server bot and plays it, mirroring the match.
- ML (TorchSharp, ILGPU, `NeuralMcts`, `PolicyValueNetwork`, `SelfPlayTrainer`) stays in C#;
  boards come from the core's `TicTacToeView`.
