using MiniGameNetworkBot.TicTacToe.Core;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe.Adapters;

public sealed class ProtoStateView : IBoardView
{
    private readonly TicTacToeGameState _state;
    private readonly Mark[,] _cells;

    public int Width { get; }
    public int Height { get; }
    public int WinCount { get; }
    public Mark CurrentPlayer { get; }

    public ProtoStateView(TicTacToeGameState state)
    {
        _state = state;
        Width = (int)state.FieldWidth;
        Height = (int)state.FieldHeight;
        WinCount = (int)state.WinCount;

        var xCount = state.Board.Count(c => c.Mark == MarkType.X);
        var oCount = state.Board.Count(c => c.Mark == MarkType.O);
        CurrentPlayer = xCount <= oCount ? Mark.X : Mark.O;

        _cells = new Mark[Height, Width];
        foreach (var cell in state.Board)
        {
            _cells[cell.Y, cell.X] = cell.Mark switch
            {
                MarkType.X => Mark.X,
                MarkType.O => Mark.O,
                _ => Mark.Empty
            };
        }
    }

    public Mark GetCell(int x, int y) => _cells[y, x];

    public List<(int X, int Y)> GetAvailableMoves()
    {
        if (_state.Board.Count == 0)
            return [(Width / 2, Height / 2)];

        var moves = new HashSet<(int, int)>();

        foreach (var cell in _state.Board)
        {
            for (var dy = -2; dy <= 2; dy++)
            {
                for (var dx = -2; dx <= 2; dx++)
                {
                    var nx = (int)cell.X + dx;
                    var ny = (int)cell.Y + dy;
                    if (nx >= 0 && ny >= 0 && nx < Width && ny < Height && _cells[ny, nx] == Mark.Empty)
                        moves.Add((nx, ny));
                }
            }
        }

        return moves.ToList();
    }

    public Mark[,] ToArray() => (Mark[,])_cells.Clone();
}
