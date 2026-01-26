using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe.Bots;

internal static class MarkTypeExtensions
{
    public static MarkType Opponent(this MarkType mark) =>
        mark == MarkType.X ? MarkType.O : MarkType.X;
}

public sealed class MctsBot : ITicTacToeBot
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

    public PlaceMarkCommand Move(TicTacToeGameState gameState)
    {
        var board = new Board(gameState);
        var bestMove = CalculateMctsMove(board);
        return new PlaceMarkCommand { X = (uint)bestMove.X, Y = (uint)bestMove.Y };
    }

    private (int X, int Y) CalculateMctsMove(Board board)
    {
        var root = new MctsNode(null, (-1, -1), board.CurrentMark.Opponent());
        root.ExpandMoves(board.AvailableMoves);

        for (var i = 0; i < _simulationsPerMove; i++)
        {
            var node = root;

            // Selection - traverse to leaf
            while (node.UntriedMoves.Count == 0 && node.Children.Count > 0)
            {
                node = SelectChild(node);
                var mark = node.PlayerMark;
                board.Place(node.Move.X, node.Move.Y, mark);
            }

            // Expansion
            if (node.UntriedMoves.Count > 0)
            {
                var moveIdx = _random.Next(node.UntriedMoves.Count);
                var move = node.UntriedMoves[moveIdx];
                node.UntriedMoves.RemoveAt(moveIdx);

                var childMark = node.PlayerMark.Opponent();
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

            // Simulation
            MarkType? winner;
            if (node.IsTerminal)
            {
                winner = node.TerminalWinner;
            }
            else
            {
                winner = Simulate(board);
            }

            // Backpropagation - unwind and update scores
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

    private MctsNode SelectChild(MctsNode node)
    {
        var logParentVisits = Math.Log(node.Visits);
        return node.Children.MaxBy(c =>
            c.Wins / c.Visits + _explorationConstant * Math.Sqrt(logParentVisits / c.Visits))!;
    }

    private MarkType? Simulate(Board board)
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
        public MarkType PlayerMark { get; }
        public List<MctsNode> Children { get; } = [];
        public List<(int X, int Y)> UntriedMoves { get; } = [];
        public int Visits { get; set; }
        public double Wins { get; set; }
        public bool IsTerminal { get; set; }
        public MarkType? TerminalWinner { get; set; }

        public MctsNode(MctsNode? parent, (int X, int Y) move, MarkType playerMark)
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
        private readonly MarkType[,] _cells;
        private readonly int _width;
        private readonly int _height;
        private readonly int _winCount;
        private readonly Stack<(int X, int Y, List<(int, int)> AddedMoves, List<(int, int)> RemovedMoves)> _history = new();

        public MarkType BotMark { get; }
        public MarkType CurrentMark { get; private set; }
        public List<(int X, int Y)> AvailableMoves { get; }

        public Board(TicTacToeGameState state)
        {
            _width = (int)state.FieldWidth;
            _height = (int)state.FieldHeight;
            _winCount = (int)state.WinCount;
            _cells = new MarkType[_height, _width];

            for (var y = 0; y < _height; y++)
                for (var x = 0; x < _width; x++)
                    _cells[y, x] = MarkType.Empty;

            foreach (var cell in state.Board)
                _cells[cell.Y, cell.X] = cell.Mark;

            BotMark = state.CurrentPlayer?.PlayerId == state.PlayerX?.PlayerId ? MarkType.X : MarkType.O;
            CurrentMark = BotMark;

            AvailableMoves = ComputeAvailableMoves();
        }

        public void Place(int x, int y, MarkType mark)
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
                        _cells[ny, nx] == MarkType.Empty && !AvailableMoves.Contains((nx, ny)))
                    {
                        AvailableMoves.Add((nx, ny));
                        addedMoves.Add((nx, ny));
                    }
                }
            }

            _history.Push((x, y, addedMoves, removedMoves));
            CurrentMark = CurrentMark.Opponent();
        }

        public void Undo()
        {
            var (x, y, addedMoves, removedMoves) = _history.Pop();

            _cells[y, x] = MarkType.Empty;

            foreach (var move in addedMoves)
                AvailableMoves.Remove(move);

            foreach (var move in removedMoves)
                AvailableMoves.Add(move);

            CurrentMark = CurrentMark.Opponent();
        }

        public bool CheckWinAt(int x, int y)
        {
            var mark = _cells[y, x];
            if (mark == MarkType.Empty)
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

        private List<(int X, int Y)> ComputeAvailableMoves()
        {
            var hasAnyMark = false;
            var moves = new List<(int X, int Y)>();
            var seen = new HashSet<(int, int)>();

            for (var y = 0; y < _height; y++)
            {
                for (var x = 0; x < _width; x++)
                {
                    if (_cells[y, x] == MarkType.Empty)
                        continue;

                    hasAnyMark = true;

                    for (var dy = -2; dy <= 2; dy++)
                    {
                        for (var dx = -2; dx <= 2; dx++)
                        {
                            var nx = x + dx;
                            var ny = y + dy;
                            if (nx >= 0 && ny >= 0 && nx < _width && ny < _height &&
                                _cells[ny, nx] == MarkType.Empty && seen.Add((nx, ny)))
                            {
                                moves.Add((nx, ny));
                            }
                        }
                    }
                }
            }

            if (!hasAnyMark)
                return [(_width / 2, _height / 2)];

            return moves;
        }
    }
}
