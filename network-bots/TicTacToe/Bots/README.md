# TicTacToe Bots

## Bot Interface

All bots implement the `IBot` interface:

```csharp
public interface IBot
{
    string Name { get; }
    (int X, int Y) GetMove(IBoardView board);
}
```

## Available Bots

### MinimaxBot

Classic minimax with alpha-beta pruning.

```csharp
var bot = new MinimaxBot(maxDepth: 2);
```

- Works with any board size
- Default depth: 2

### MctsBot

Monte Carlo Tree Search with UCB1.

```csharp
var bot = new MctsBot(simulationsPerMove: 10000);
```

- Works with any board size
- Default simulations: 10000

### RandomBot

Random move selection from available moves.

```csharp
var bot = new RandomBot();
```

### HybridNeuralBot

Combination of tactics, neural network and shallow search.

```csharp
var network = new PolicyValueNetwork();
network.LoadModel("neural_model.dat");
var bot = new HybridNeuralBot(network, searchDepth: 2);
```

**Constraints:**
- 15x15 board only
- 5 in a row to win only
- Throws `ArgumentException` on violation

## Creating Your Own Bot

```csharp
using MiniGameNetworkBot.TicTacToe.Core;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public sealed class MyBot : IBot
{
    public string Name => "MyBot";

    public (int X, int Y) GetMove(IBoardView board)
    {
        // Use TacticsEngine for basic checks
        var tactical = TacticsEngine.GetTacticalMove(board, board.CurrentPlayer);
        if (tactical.HasValue)
            return tactical.Value;

        // Your logic
        var moves = board.GetAvailableMoves();
        return moves[0];
    }
}
```

## IBoardView API

```csharp
int Width { get; }           // Board width
int Height { get; }          // Board height
int WinCount { get; }        // Marks in a row to win
Mark CurrentPlayer { get; }  // Whose turn (Mark.X or Mark.O)

Mark GetCell(int x, int y);              // Cell contents
List<(int X, int Y)> GetAvailableMoves(); // Available moves
Mark[,] ToArray();                        // Copy of board as array
```

## TacticsEngine

Static methods for tactical checks:

```csharp
// Full tactics check (returns best move or null)
var move = TacticsEngine.GetTacticalMove(board, myMark);

// Check for winning move
bool wins = TacticsEngine.IsWinningMove(board, x, y, mark);
bool wins = TacticsEngine.IsWinningMove(cells, winCount, x, y, mark);
```
