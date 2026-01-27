# TicTacToe Bot Framework

Framework for creating and testing bots for TicTacToe (15x15, 5 in a row).

## Quick Start

### Playing against server

```bash
# Hybrid bot vs server minimax
dotnet run -- --bot-type hybrid --model neural_model.dat

# Against local server
dotnet run -- --local --bot-type hybrid

# Against random bot
dotnet run -- --opponent random --bot-type hybrid
```

### Local benchmark

```bash
# Hybrid vs Minimax, 100 games
dotnet run -- --benchmark --bot1 hybrid --bot2 minimax --games 100

# Minimax(d=3) vs Minimax(d=2)
dotnet run -- --benchmark --bot1 minimax --bot2 minimax --minimax-depth 3 --games 50
```

### Common parameters

| Flag | Description | Default |
|------|-------------|---------|
| `--local` | Connect to localhost:5001 | - |
| `--no-ui` | Disable GUI | - |
| `--model PATH` | Path to neural network model | neural_model.dat |

### Game parameters

| Flag | Description | Default |
|------|-------------|---------|
| `--bot-type`, `-b` | Bot type: Minimax, Mcts, Hybrid | Minimax |
| `--opponent`, `-o` | Server opponent: Minimax, Random | Minimax |

### Benchmark parameters

| Flag | Description | Default |
|------|-------------|---------|
| `--bot1` | First bot | Hybrid |
| `--bot2` | Second bot | Minimax |
| `--games`, `-g` | Number of games | 100 |
| `--threads`, `-t` | Thread count | CPU count |
| `--minimax-depth`, `-d` | Minimax search depth | 2 |

## Running local server

```bash
# From project root
cargo run -p mini_games_server
```

Server starts at `http://localhost:5001`.

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
