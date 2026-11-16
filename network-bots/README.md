# TicTacToe Bot Framework

C# bots for TicTacToe (15x15, 5 in a row) that play on the mini-games server over protocol v2
(WebSocket, protobuf JSON). The game rules come from the Rust core through the `mini_games`
native library (`crates/ffi`), the same code the server runs; the bots, MCTS and the neural
network stay in C#. Background: `../docs/bots.md`.

## Build

```bash
dotnet build network-bots          # from the repository root
```

The build compiles `../proto/minigames/v2/*.proto`, runs `cargo build -p mg-ffi --release` and
copies `mini_games.dll` (`libmini_games.so` on Linux) next to the output. Options:

| Property | Default | Meaning |
|----------|---------|---------|
| `CargoCommand` | `cargo` | e.g. `"cargo +1.100.0"` |
| `CargoTargetDir` | `../target` | cargo `--target-dir` |
| `SkipNativeBuild` | - | `true`: do not run cargo; copy the library into `bin/...` yourself |

The first restore downloads TorchSharp with CUDA (several GB).

## Quick Start

### Playing against a server

A server accepts network bots by key (`MG_BOT_KEYS=name:sha256(key)`, see `../docs/bots.md`).

```bash
# Local server with bot `alpha`, key `secret`
MG_BOT_KEYS=alpha:2bb80d537b1da3e38bd30361aa855686bde0eacd7162fef6a25fe97bf527a25b cargo run -p mg-server

# Hybrid bot vs the server's minimax
dotnet run -- --local --bot-key secret --bot-type hybrid --model neural_model.dat

# Against the server's random bot, without the window
dotnet run -- --local --bot-key secret --opponent random --no-ui

# Production server; the key can come from the environment
MG_BOT_KEY=... dotnet run -- --bot-type minimax
```

The bot hosts a lobby, seats the server bot, starts the match and mirrors it with the core.
A dropped connection is resumed with the resume token.

### Local benchmark

Runs on the Rust core through the native library; an illegal move forfeits the game.

```bash
# Hybrid vs Minimax, 100 games
dotnet run -- --benchmark --bot1 hybrid --bot2 minimax --games 100

# Minimax(d=3) vs Minimax(d=2) on a smaller board
dotnet run -- --benchmark --bot1 minimax --bot2 minimax --minimax-depth 3 --games 50 --board-size 9
```

Built-in Rust bots play each other with `mg-arena` (`../docs/bots.md`).

### Common parameters

| Flag | Description | Default |
|------|-------------|---------|
| `--server URL` | Server WebSocket | `wss://braintvsminigames.xyz/ws` |
| `--local` | Use `ws://127.0.0.1:5000/ws` | - |
| `--bot-key KEY` | Bot key (or `MG_BOT_KEY`) | - |
| `--no-ui` | Disable GUI | - |
| `--model PATH` | Path to neural network model | neural_model.dat |

### Game parameters

| Flag | Description | Default |
|------|-------------|---------|
| `--bot-type`, `-b` | Bot type: Minimax, Mcts, Hybrid, Random | Minimax |
| `--opponent`, `-o` | Server opponent: Minimax, Random | Minimax |

### Benchmark parameters

| Flag | Description | Default |
|------|-------------|---------|
| `--bot1` | First bot | Hybrid |
| `--bot2` | Second bot | Minimax |
| `--games`, `-g` | Number of games | 100 |
| `--threads`, `-t` | Thread count | CPU count |
| `--minimax-depth`, `-d` | Minimax search depth | 2 |
| `--board-size` | Board side | 15 |
| `--win-count` | Marks in a row to win | 5 |

## Layout

- `Native/` — P/Invoke into `mini_games` (`LocalMatch`: a local match; `MatchMirror`: a server
  match replayed from its frames).
- `Network/` — WebSocket connection, bot-key sign-in and resume.
- `TicTacToe/Core/TicTacToeEngine.cs` — the game for bots, tournaments and training, backed by
  the core. `Clone()` asks the core to replay the game so far, so it costs more than copying an
  array.
- `TicTacToe/TicTacToeRunner.cs` — one match against a server bot.

## Adding your own bot

1. Add a class to `TicTacToe/Bots/`:

```csharp
using MiniGameNetworkBot.TicTacToe.Core;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public sealed class MyBot : IBot
{
    public string Name => "MyBot";

    public (int X, int Y) GetMove(IBoardView board)
    {
        // board.Width, board.Height - board dimensions
        // board.WinCount - marks in a row to win
        // board.CurrentPlayer - whose turn (Mark.X or Mark.O)
        // board.GetCell(x, y) - cell contents
        // board.GetAvailableMoves() - available moves

        var moves = board.GetAvailableMoves();

        // Your move selection logic

        return moves[0]; // Return (x, y) of the move
    }
}
```

2. Add to `BotType` enum in `Settings.cs`:

```csharp
public enum BotType
{
    Minimax,
    Mcts,
    Hybrid,
    MyBot  // <--
}
```

3. Register in `BotFactory.cs`:

```csharp
public static IBot Create(BotType type, ...)
{
    return type switch
    {
        BotType.MyBot => new MyBot(),
        // ...
    };
}
```

## Available bots

| Bot | Description | Constraints |
|-----|-------------|-------------|
| `MinimaxBot` | Classic minimax with alpha-beta | Any board size |
| `MctsBot` | Monte Carlo Tree Search | Any board size |
| `RandomBot` | Random move selection | Any board size |
| `HybridNeuralBot` | Tactics + neural network + shallow search | 15x15 only, 5 in a row |

## Tactics Engine

`TacticsEngine` checks positions in priority order.

1. **Win** - win the game
2. **Block win** - block opponent's winning move
3. **Create open four** - create 4 in a row with both ends open
4. **Block open four** - block opponent's open four
5. **Block four** - block any four in a row
6. **Block double threat** - block opponent's fork
7. **Create double threat** - create own fork
8. **Create open three** - create open three
9. **Block open three** - block opponent's open three
