namespace MiniGameNetworkBot.TicTacToe.Core;

public enum Mark : byte
{
    Empty = 0,
    X = 1,
    O = 2
}

public sealed class GameEngine : IBoardView
{
    private readonly Mark[,] _cells;
    private readonly Stack<(int X, int Y)> _moveHistory = new();

    public int Width { get; }
    public int Height { get; }
    public int WinCount { get; }
    public Mark CurrentPlayer { get; private set; } = Mark.X;
    public Mark? Winner { get; private set; }
    public bool IsGameOver => Winner != null || _moveHistory.Count == Width * Height;
    public int MoveCount => _moveHistory.Count;

    public GameEngine(int width = 15, int height = 15, int winCount = 5)
    {
        Width = width;
        Height = height;
        WinCount = winCount;
        _cells = new Mark[height, width];
    }

    public Mark GetCell(int x, int y) => _cells[y, x];

    public bool CanPlace(int x, int y) =>
        x >= 0 && x < Width && y >= 0 && y < Height && _cells[y, x] == Mark.Empty;

    public bool Place(int x, int y)
    {
        if (!CanPlace(x, y) || IsGameOver)
            return false;

        _cells[y, x] = CurrentPlayer;
        _moveHistory.Push((x, y));

        if (CheckWinAt(x, y))
            Winner = CurrentPlayer;

        CurrentPlayer = CurrentPlayer == Mark.X ? Mark.O : Mark.X;
        return true;
    }

    public (int X, int Y)? GetLastMove() =>
        _moveHistory.Count > 0 ? _moveHistory.Peek() : null;

    public List<(int X, int Y)> GetAvailableMoves()
    {
        if (_moveHistory.Count == 0)
            return [(Width / 2, Height / 2)];

        var moves = new HashSet<(int, int)>();

        for (var y = 0; y < Height; y++)
        {
            for (var x = 0; x < Width; x++)
            {
                if (_cells[y, x] == Mark.Empty)
                    continue;

                for (var dy = -2; dy <= 2; dy++)
                {
                    for (var dx = -2; dx <= 2; dx++)
                    {
                        var nx = x + dx;
                        var ny = y + dy;
                        if (nx >= 0 && ny >= 0 && nx < Width && ny < Height && _cells[ny, nx] == Mark.Empty)
                            moves.Add((nx, ny));
                    }
                }
            }
        }

        return moves.ToList();
    }

    public float[] GetBoardState(Mark perspective) => this.GetBoardStateForNetwork(perspective);

    public GameEngine Clone()
    {
        var clone = new GameEngine(Width, Height, WinCount);
        for (var y = 0; y < Height; y++)
            for (var x = 0; x < Width; x++)
                clone._cells[y, x] = _cells[y, x];

        foreach (var move in _moveHistory.Reverse())
            clone._moveHistory.Push(move);

        clone.CurrentPlayer = CurrentPlayer;
        clone.Winner = Winner;
        return clone;
    }

    public Mark[,] ToArray()
    {
        var result = new Mark[Height, Width];
        for (var y = 0; y < Height; y++)
            for (var x = 0; x < Width; x++)
                result[y, x] = _cells[y, x];
        return result;
    }

    public void Reset()
    {
        for (var y = 0; y < Height; y++)
            for (var x = 0; x < Width; x++)
                _cells[y, x] = Mark.Empty;

        _moveHistory.Clear();
        CurrentPlayer = Mark.X;
        Winner = null;
    }

    public static GameEngine FromBoard(IBoardView board)
    {
        var engine = new GameEngine(board.Width, board.Height, board.WinCount);

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

        var maxMoves = Math.Max(xMoves.Count, oMoves.Count);
        for (var i = 0; i < maxMoves; i++)
        {
            if (i < xMoves.Count)
                engine.Place(xMoves[i].X, xMoves[i].Y);
            if (i < oMoves.Count)
                engine.Place(oMoves[i].X, oMoves[i].Y);
        }

        return engine;
    }

    private bool CheckWinAt(int x, int y)
    {
        var mark = _cells[y, x];
        if (mark == Mark.Empty)
            return false;

        ReadOnlySpan<(int dx, int dy)> directions = [(1, 0), (0, 1), (1, 1), (1, -1)];

        foreach (var (dx, dy) in directions)
        {
            var count = 1;

            for (var i = 1; i < WinCount; i++)
            {
                var nx = x + dx * i;
                var ny = y + dy * i;
                if (nx < 0 || ny < 0 || nx >= Width || ny >= Height || _cells[ny, nx] != mark)
                    break;
                count++;
            }

            for (var i = 1; i < WinCount; i++)
            {
                var nx = x - dx * i;
                var ny = y - dy * i;
                if (nx < 0 || ny < 0 || nx >= Width || ny >= Height || _cells[ny, nx] != mark)
                    break;
                count++;
            }

            if (count >= WinCount)
                return true;
        }

        return false;
    }
}
