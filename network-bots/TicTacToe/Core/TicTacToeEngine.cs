using Minigames.V2;
using MiniGameNetworkBot.Native;

namespace MiniGameNetworkBot.TicTacToe.Core;

/// <summary>
/// A local TicTacToe game whose rules run in the Rust core (the same code the server runs).
/// Two human seats; the seat in slot 0 plays X and moves first.
/// </summary>
public sealed class TicTacToeEngine : IBoardView, IDisposable
{
    private readonly LocalMatch _match;
    private TicTacToeView _view;
    private Mark[,] _cells;

    public TicTacToeEngine(int width = 15, int height = 15, int winCount = 5)
    {
        _match = new LocalMatch(Header(width, height, winCount), seed: 0);
        _view = ReadView(_match);
        _cells = TicTacToeViews.Cells(_view);
    }

    private TicTacToeEngine(LocalMatch match, TicTacToeView view, Mark[,] cells, int moveCount)
    {
        _match = match;
        _view = view;
        _cells = cells;
        MoveCount = moveCount;
    }

    public int Width => (int)_view.FieldWidth;
    public int Height => (int)_view.FieldHeight;
    public int WinCount => (int)_view.WinCount;
    public Mark CurrentPlayer => TicTacToeViews.CurrentMark(_view);
    public Mark? Winner => TicTacToeViews.Winner(_view);
    public bool IsGameOver => _view.End is not null;
    public int MoveCount { get; private set; }
    public TicTacToeView View => _view;

    public Mark GetCell(int x, int y) => _cells[y, x];

    /// <summary>Plays for the side to move. False when the core refuses the move.</summary>
    public bool Place(int x, int y)
    {
        if (x < 0 || y < 0 || !_view.HasCurrentSlot)
            return false;
        var command = new GameCommand
        {
            Tictactoe = new TicTacToeCommand { Place = new TicTacToePoint { X = (uint)x, Y = (uint)y } }
        };
        if (!_match.TrySubmit(_view.CurrentSlot, command))
            return false;
        _match.SealFrame();
        _view = ReadView(_match);
        _cells = TicTacToeViews.Cells(_view);
        MoveCount++;
        return true;
    }

    public (int X, int Y)? GetLastMove() =>
        MoveCount > 0 && _view.LastMove is { } last ? ((int)last.X, (int)last.Y) : null;

    public List<(int X, int Y)> GetAvailableMoves() => CandidateMoves.Around(_cells);

    public float[] GetBoardState(Mark perspective) => this.GetBoardStateForNetwork(perspective);

    public Mark[,] ToArray() => (Mark[,])_cells.Clone();

    /// <summary>Costs time proportional to the moves so far (the core replays them).</summary>
    public TicTacToeEngine Clone() => new(_match.Clone(), _view, _cells, MoveCount);

    public ReplayFile ReplayFile() => _match.ReplayFile();

    /// <summary>
    /// Replays a position by alternating X and O marks, so only positions an alternating game
    /// can reach (as many O as X marks, or one more X) come out right.
    /// </summary>
    public static TicTacToeEngine FromBoard(IBoardView board)
    {
        var engine = new TicTacToeEngine(board.Width, board.Height, board.WinCount);
        var xMoves = new List<(int X, int Y)>();
        var oMoves = new List<(int X, int Y)>();
        for (var y = 0; y < board.Height; y++)
        {
            for (var x = 0; x < board.Width; x++)
            {
                var cell = board.GetCell(x, y);
                if (cell == Mark.X)
                    xMoves.Add((x, y));
                else if (cell == Mark.O)
                    oMoves.Add((x, y));
            }
        }

        for (var i = 0; i < Math.Max(xMoves.Count, oMoves.Count); i++)
        {
            if (i < xMoves.Count)
                engine.Place(xMoves[i].X, xMoves[i].Y);
            if (i < oMoves.Count)
                engine.Place(oMoves[i].X, oMoves[i].Y);
        }

        return engine;
    }

    public void Dispose() => _match.Dispose();

    private static MatchHeader Header(int width, int height, int winCount) => new()
    {
        MatchId = "local",
        Settings = new GameSettings
        {
            Tictactoe = new TicTacToeSettings
            {
                FieldWidth = (uint)width,
                FieldHeight = (uint)height,
                WinCount = (uint)winCount,
                FirstPlayer = TicTacToeFirstPlayerMode.Host
            }
        },
        Seats = { new MatchSeat { Slot = 0 }, new MatchSeat { Slot = 1 } }
    };

    private static TicTacToeView ReadView(LocalMatch match) =>
        match.View().Tictactoe ?? throw new InvalidOperationException("the core returned a non-TicTacToe view");
}
