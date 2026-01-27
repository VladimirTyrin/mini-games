using MiniGameNetworkBot.TicTacToe.Core;
using MiniGameNetworkBot.TicTacToe.Training;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public sealed class HybridNeuralBot : IBot
{
    private const int ExpectedWidth = 15;
    private const int ExpectedHeight = 15;
    private const int ExpectedWinCount = 5;

    private readonly PolicyValueNetwork _network;
    private readonly int _searchDepth;

    public string Name => "HybridNeural";

    public HybridNeuralBot(PolicyValueNetwork network, int searchDepth = 2)
    {
        _network = network;
        _searchDepth = searchDepth;
    }

    public (int X, int Y) GetMove(IBoardView board)
    {
        if (board.Width != ExpectedWidth || board.Height != ExpectedHeight)
            throw new ArgumentException($"HybridNeuralBot requires {ExpectedWidth}x{ExpectedHeight} board, got {board.Width}x{board.Height}");

        if (board.WinCount != ExpectedWinCount)
            throw new ArgumentException($"HybridNeuralBot requires win count of {ExpectedWinCount}, got {board.WinCount}");

        var availableMoves = board.GetAvailableMoves();

        if (availableMoves.Count == 0)
            return (0, 0);

        if (availableMoves.Count == 1)
            return availableMoves[0];

        var tacticalMove = TacticsEngine.GetTacticalMove(board, board.CurrentPlayer);
        if (tacticalMove.HasValue)
            return tacticalMove.Value;

        return SearchBestMove(board, availableMoves);
    }

    private (int X, int Y) SearchBestMove(IBoardView board, List<(int X, int Y)> availableMoves)
    {
        var cells = board.ToArray();
        var myMark = board.CurrentPlayer;
        var oppMark = myMark == Mark.X ? Mark.O : Mark.X;

        var bestMove = availableMoves[0];
        var bestScore = float.MinValue;

        var state = board.GetBoardStateForNetwork();
        var (policy, _) = _network.Predict(state);

        foreach (var (x, y) in availableMoves)
        {
            var prior = policy[y * board.Width + x];

            cells[y, x] = myMark;
            var score = -AlphaBeta(
                cells, board.Width, board.Height, board.WinCount,
                _searchDepth - 1, float.MinValue, float.MaxValue,
                oppMark, myMark);
            cells[y, x] = Mark.Empty;

            var combinedScore = score + prior * 50;

            if (combinedScore > bestScore)
            {
                bestScore = combinedScore;
                bestMove = (x, y);
            }
        }

        return bestMove;
    }

    private float AlphaBeta(Mark[,] cells, int width, int height, int winCount,
        int depth, float alpha, float beta, Mark currentPlayer, Mark rootPlayer)
    {
        var availableMoves = GetAvailableMoves(cells, width, height);

        if (availableMoves.Count == 0)
            return 0;

        foreach (var (x, y) in availableMoves)
        {
            if (TacticsEngine.IsWinningMove(cells, winCount, x, y, currentPlayer))
            {
                return currentPlayer == rootPlayer ? 1000 + depth : -1000 - depth;
            }
        }

        if (depth <= 0)
        {
            return EvaluateWithNetwork(cells, width, height, winCount, rootPlayer);
        }

        var opponent = currentPlayer == Mark.X ? Mark.O : Mark.X;
        var bestScore = float.MinValue;

        foreach (var (x, y) in availableMoves)
        {
            cells[y, x] = currentPlayer;

            var score = -AlphaBeta(cells, width, height, winCount,
                depth - 1, -beta, -alpha, opponent, rootPlayer);

            cells[y, x] = Mark.Empty;

            bestScore = Math.Max(bestScore, score);
            alpha = Math.Max(alpha, score);

            if (alpha >= beta)
                break;
        }

        return bestScore;
    }

    private float EvaluateWithNetwork(Mark[,] cells, int width, int height, int winCount, Mark perspective)
    {
        var state = GetBoardStateFromCells(cells, width, height, perspective);
        var (_, value) = _network.Predict(state);

        var tacticalScore = PositionEvaluator.Evaluate(cells, winCount, perspective);

        return value * 100 + tacticalScore * 0.1f;
    }

    private static float[] GetBoardStateFromCells(Mark[,] cells, int width, int height, Mark perspective)
    {
        var state = new float[3 * height * width];
        var oppMark = perspective == Mark.X ? Mark.O : Mark.X;

        for (var y = 0; y < height; y++)
        {
            for (var x = 0; x < width; x++)
            {
                var idx = y * width + x;
                var cell = cells[y, x];

                if (cell == perspective)
                    state[idx] = 1f;
                else if (cell == oppMark)
                    state[height * width + idx] = 1f;
                else
                    state[2 * height * width + idx] = 1f;
            }
        }

        return state;
    }

    private static List<(int X, int Y)> GetAvailableMoves(Mark[,] cells, int width, int height)
    {
        var moves = new HashSet<(int, int)>();
        var hasAny = false;

        for (var y = 0; y < height; y++)
        {
            for (var x = 0; x < width; x++)
            {
                if (cells[y, x] == Mark.Empty)
                    continue;

                hasAny = true;

                for (var dy = -2; dy <= 2; dy++)
                {
                    for (var dx = -2; dx <= 2; dx++)
                    {
                        var nx = x + dx;
                        var ny = y + dy;
                        if (nx >= 0 && ny >= 0 && nx < width && ny < height && cells[ny, nx] == Mark.Empty)
                            moves.Add((nx, ny));
                    }
                }
            }
        }

        if (!hasAny)
            return [(width / 2, height / 2)];

        return moves.ToList();
    }
}
