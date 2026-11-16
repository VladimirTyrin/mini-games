using Minigames.V2;

namespace MiniGameNetworkBot.TicTacToe.Core;

/// <summary>An immutable board read from a core view (for example a network match mirror).</summary>
public sealed class BoardSnapshot : IBoardView
{
    private readonly Mark[,] _cells;

    public BoardSnapshot(TicTacToeView view)
    {
        _cells = TicTacToeViews.Cells(view);
        Width = (int)view.FieldWidth;
        Height = (int)view.FieldHeight;
        WinCount = (int)view.WinCount;
        CurrentPlayer = TicTacToeViews.CurrentMark(view);
    }

    public int Width { get; }
    public int Height { get; }
    public int WinCount { get; }
    public Mark CurrentPlayer { get; }

    public Mark GetCell(int x, int y) => _cells[y, x];

    public List<(int X, int Y)> GetAvailableMoves() => CandidateMoves.Around(_cells);

    public Mark[,] ToArray() => (Mark[,])_cells.Clone();
}
