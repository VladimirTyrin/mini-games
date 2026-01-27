using MiniGameNetworkBot.TicTacToe.Core;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public sealed class MctsBot : IBot
{
    private readonly int _simulationsPerMove;
    private readonly double _explorationConstant;
    private readonly Random _random = new();

    public MctsBot(int simulationsPerMove = 10000, double explorationConstant = 1.41)
    {
        _simulationsPerMove = simulationsPerMove;
        _explorationConstant = explorationConstant;
    }

    public string Name => "MCTS";

    public (int X, int Y) GetMove(IBoardView boardView)
    {
        var board = new Board(boardView);
        return CalculateMctsMove(board);
    }

    private (int X, int Y) CalculateMctsMove(Board board)
    {
        var root = new MctsNode(null, (-1, -1), Opponent(board.CurrentMark));
        root.ExpandMoves(board.AvailableMoves);

        for (var i = 0; i < _simulationsPerMove; i++)
        {
            var node = root;

            while (node.UntriedMoves.Count == 0 && node.Children.Count > 0)
            {
                node = SelectChild(node);
                board.Place(node.Move.X, node.Move.Y, node.PlayerMark);
            }

            if (node.UntriedMoves.Count > 0)
            {
                var moveIdx = _random.Next(node.UntriedMoves.Count);
                var move = node.UntriedMoves[moveIdx];
                node.UntriedMoves.RemoveAt(moveIdx);

                var childMark = Opponent(node.PlayerMark);
                board.Place(move.X, move.Y, childMark);

                var child = new MctsNode(node, move, childMark);

                if (board.CheckWinAt(move.X, move.Y))
                {
                    child.IsTerminal = true;
                    child.TerminalWinner = childMark;
                }
                else if (board.AvailableMoves.Count == 0)
                {
                    child.IsTerminal = true;
                    child.TerminalWinner = null;
                }
                else
                {
                    child.ExpandMoves(board.AvailableMoves);
                }

                node.Children.Add(child);
                node = child;
            }

            Mark? winner;
            if (node.IsTerminal)
            {
                winner = node.TerminalWinner;
            }
            else
            {
                winner = Simulate(board);
            }

            while (node != null)
            {
                node.Visits++;
                if (winner == board.BotMark)
                    node.Wins++;
                else if (winner == null)
                    node.Wins += 0.5;

                if (node.Parent != null)
                    board.Undo();

                node = node.Parent;
            }
        }

        var bestChild = root.Children.MaxBy(c => c.Visits);
        return bestChild?.Move ?? board.AvailableMoves[0];
    }

    private static Mark Opponent(Mark mark) => mark == Mark.X ? Mark.O : Mark.X;

    private MctsNode SelectChild(MctsNode node)
    {
        var logParentVisits = Math.Log(node.Visits);
        return node.Children.MaxBy(c =>
            c.Wins / c.Visits + _explorationConstant * Math.Sqrt(logParentVisits / c.Visits))!;
    }

    private Mark? Simulate(Board board)
    {
        var depth = 0;

        while (board.AvailableMoves.Count > 0)
        {
            var moves = board.AvailableMoves;
            var moveIdx = _random.Next(moves.Count);
            var (x, y) = moves[moveIdx];
            var mark = board.CurrentMark;

            board.Place(x, y, mark);
            depth++;

            if (board.CheckWinAt(x, y))
            {
                for (var i = 0; i < depth; i++)
                    board.Undo();
                return mark;
            }
        }

        for (var i = 0; i < depth; i++)
            board.Undo();

        return null;
    }

    private sealed class MctsNode
    {
        public MctsNode? Parent { get; }
        public (int X, int Y) Move { get; }
        public Mark PlayerMark { get; }
        public List<MctsNode> Children { get; } = [];
        public List<(int X, int Y)> UntriedMoves { get; } = [];
        public int Visits { get; set; }
        public double Wins { get; set; }
        public bool IsTerminal { get; set; }
        public Mark? TerminalWinner { get; set; }

        public MctsNode(MctsNode? parent, (int X, int Y) move, Mark playerMark)
        {
            Parent = parent;
            Move = move;
            PlayerMark = playerMark;
        }

        public void ExpandMoves(List<(int X, int Y)> moves)
        {
            UntriedMoves.AddRange(moves);
        }
    }

    private sealed class Board
    {
        private readonly Mark[,] _cells;
        private readonly int _width;
        private readonly int _height;
        private readonly int _winCount;
        private readonly Stack<(int X, int Y, List<(int, int)> AddedMoves, List<(int, int)> RemovedMoves)> _history = new();

        public Mark BotMark { get; }
        public Mark CurrentMark { get; private set; }
        public List<(int X, int Y)> AvailableMoves { get; }

        public Board(IBoardView view)
        {
            _width = view.Width;
            _height = view.Height;
            _winCount = view.WinCount;
            _cells = new Mark[_height, _width];

            for (var y = 0; y < _height; y++)
                for (var x = 0; x < _width; x++)
                    _cells[y, x] = view.GetCell(x, y);

            BotMark = view.CurrentPlayer;
            CurrentMark = view.CurrentPlayer;

            AvailableMoves = view.GetAvailableMoves();
        }

        public void Place(int x, int y, Mark mark)
        {
            var removedMoves = new List<(int, int)>();
            var addedMoves = new List<(int, int)>();

            AvailableMoves.Remove((x, y));
            removedMoves.Add((x, y));

            _cells[y, x] = mark;

            for (var dy = -2; dy <= 2; dy++)
            {
                for (var dx = -2; dx <= 2; dx++)
                {
                    var nx = x + dx;
                    var ny = y + dy;
                    if (nx >= 0 && ny >= 0 && nx < _width && ny < _height &&
                        _cells[ny, nx] == Mark.Empty && !AvailableMoves.Contains((nx, ny)))
                    {
                        AvailableMoves.Add((nx, ny));
                        addedMoves.Add((nx, ny));
                    }
                }
            }

            _history.Push((x, y, addedMoves, removedMoves));
            CurrentMark = Opponent(CurrentMark);
        }

        public void Undo()
        {
            var (x, y, addedMoves, removedMoves) = _history.Pop();

            _cells[y, x] = Mark.Empty;

            foreach (var move in addedMoves)
                AvailableMoves.Remove(move);

            foreach (var move in removedMoves)
                AvailableMoves.Add(move);

            CurrentMark = Opponent(CurrentMark);
        }

        public bool CheckWinAt(int x, int y)
        {
            var mark = _cells[y, x];
            if (mark == Mark.Empty)
                return false;

            ReadOnlySpan<(int dx, int dy)> directions = [(1, 0), (0, 1), (1, 1), (1, -1)];

            foreach (var (dx, dy) in directions)
            {
                var count = 1;

                for (var i = 1; i < _winCount; i++)
                {
                    var nx = x + dx * i;
                    var ny = y + dy * i;
                    if (nx < 0 || ny < 0 || nx >= _width || ny >= _height || _cells[ny, nx] != mark)
                        break;
                    count++;
                }

                for (var i = 1; i < _winCount; i++)
                {
                    var nx = x - dx * i;
                    var ny = y - dy * i;
                    if (nx < 0 || ny < 0 || nx >= _width || ny >= _height || _cells[ny, nx] != mark)
                        break;
                    count++;
                }

                if (count >= _winCount)
                    return true;
            }

            return false;
        }
    }
}
