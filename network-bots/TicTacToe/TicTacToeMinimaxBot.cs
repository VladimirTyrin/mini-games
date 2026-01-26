using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe;

public sealed class TicTacToeMinimaxBot : ITicTacToeBot
{
    public PlaceMarkCommand Move(TicTacToeGameState gameState)
    {
        var board = ConvertBoard(gameState);
        var botMark = GetCurrentMark(gameState);
        var winCount = (int)gameState.WinCount;

        var bestMove = CalculateMinimaxMove(board, botMark, winCount);

        return new PlaceMarkCommand { X = (uint)bestMove.X, Y = (uint)bestMove.Y };
    }

    private static MarkType[,] ConvertBoard(TicTacToeGameState state)
    {
        var width = (int)state.FieldWidth;
        var height = (int)state.FieldHeight;
        var board = new MarkType[height, width];

        for (var y = 0; y < height; y++)
        {
            for (var x = 0; x < width; x++)
            {
                board[y, x] = MarkType.Empty;
            }
        }

        foreach (var cell in state.Board)
        {
            board[cell.Y, cell.X] = cell.Mark;
        }

        return board;
    }

    private static MarkType GetCurrentMark(TicTacToeGameState state)
    {
        var currentPlayerId = state.CurrentPlayer?.PlayerId;
        if (currentPlayerId == state.PlayerX?.PlayerId)
            return MarkType.X;
        return MarkType.O;
    }

    private static MarkType GetOpponent(MarkType mark)
    {
        return mark == MarkType.X ? MarkType.O : MarkType.X;
    }

    private static (int X, int Y) CalculateMinimaxMove(MarkType[,] board, MarkType botMark, int winCount)
    {
        var availableMoves = GetAvailableMoves(board);

        if (availableMoves.Count == 0)
            throw new InvalidOperationException("No available moves");

        var depthLimit = CalculateDepthLimit(board);
        var initialScore = EvaluateBoard(board, botMark, winCount);

        var results = new (int X, int Y, int Score)[availableMoves.Count];

        Parallel.For(0, availableMoves.Count, i =>
        {
            var (x, y) = availableMoves[i];
            var boardCopy = (MarkType[,])board.Clone();

            var delta = EvalDeltaBeforeMove(boardCopy, botMark, winCount, x, y, botMark);
            boardCopy[y, x] = botMark;

            var score = Minimax(
                boardCopy, winCount, 0, depthLimit, false, botMark,
                int.MinValue, int.MaxValue, x, y, initialScore + delta);

            results[i] = (x, y, score);
        });

        var bestResult = results[0];
        for (var i = 1; i < results.Length; i++)
        {
            if (results[i].Score > bestResult.Score)
                bestResult = results[i];
        }

        return (bestResult.X, bestResult.Y);
    }

    private static List<(int X, int Y)> GetAvailableMoves(MarkType[,] board)
    {
        var moves = new List<(int, int)>();
        var height = board.GetLength(0);
        var width = board.GetLength(1);

        for (var y = 0; y < height; y++)
        {
            for (var x = 0; x < width; x++)
            {
                if (board[y, x] == MarkType.Empty)
                    moves.Add((x, y));
            }
        }

        return moves;
    }

    private static int CalculateDepthLimit(MarkType[,] board)
    {
        var emptyCells = 0;
        var height = board.GetLength(0);
        var width = board.GetLength(1);

        for (var y = 0; y < height; y++)
        {
            for (var x = 0; x < width; x++)
            {
                if (board[y, x] == MarkType.Empty)
                    emptyCells++;
            }
        }

        return emptyCells switch
        {
            <= 4 => emptyCells,
            <= 9 => 5,
            <= 16 => 4,
            <= 49 => 3,
            _ => 2
        };
    }

    private static MarkType? CheckWinAt(MarkType[,] board, int winCount, int x, int y)
    {
        var mark = board[y, x];
        if (mark == MarkType.Empty)
            return null;

        var height = board.GetLength(0);
        var width = board.GetLength(1);

        (int dx, int dy)[] directions = [(1, 0), (0, 1), (1, 1), (1, -1)];

        foreach (var (dx, dy) in directions)
        {
            var count = 1;

            for (var i = 1; i < winCount; i++)
            {
                var nx = x + dx * i;
                var ny = y + dy * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (board[ny, nx] != mark)
                    break;
                count++;
            }

            for (var i = 1; i < winCount; i++)
            {
                var nx = x - dx * i;
                var ny = y - dy * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (board[ny, nx] != mark)
                    break;
                count++;
            }

            if (count >= winCount)
                return mark;
        }

        return null;
    }

    private static int Minimax(
        MarkType[,] board,
        int winCount,
        int depth,
        int maxDepth,
        bool isMaximizing,
        MarkType botMark,
        int alpha,
        int beta,
        int lastX,
        int lastY,
        int currentScore)
    {
        var winner = CheckWinAt(board, winCount, lastX, lastY);
        if (winner.HasValue)
        {
            return winner.Value == botMark ? 1000 - depth : -1000 + depth;
        }

        if (depth >= maxDepth)
            return currentScore;

        var height = board.GetLength(0);
        var width = board.GetLength(1);

        if (isMaximizing)
        {
            var maxEval = int.MinValue;
            for (var y = 0; y < height; y++)
            {
                for (var x = 0; x < width; x++)
                {
                    if (board[y, x] != MarkType.Empty)
                        continue;

                    var delta = EvalDeltaBeforeMove(board, botMark, winCount, x, y, botMark);
                    board[y, x] = botMark;

                    var eval = Minimax(
                        board, winCount, depth + 1, maxDepth, false, botMark,
                        alpha, beta, x, y, currentScore + delta);

                    board[y, x] = MarkType.Empty;

                    maxEval = Math.Max(maxEval, eval);
                    alpha = Math.Max(alpha, eval);
                    if (beta <= alpha)
                        return maxEval;
                }
            }

            return maxEval == int.MinValue ? 0 : maxEval;
        }
        else
        {
            var opponentMark = GetOpponent(botMark);
            var minEval = int.MaxValue;

            for (var y = 0; y < height; y++)
            {
                for (var x = 0; x < width; x++)
                {
                    if (board[y, x] != MarkType.Empty)
                        continue;

                    var delta = EvalDeltaBeforeMove(board, botMark, winCount, x, y, opponentMark);
                    board[y, x] = opponentMark;

                    var eval = Minimax(
                        board, winCount, depth + 1, maxDepth, true, botMark,
                        alpha, beta, x, y, currentScore + delta);

                    board[y, x] = MarkType.Empty;

                    minEval = Math.Min(minEval, eval);
                    beta = Math.Min(beta, eval);
                    if (beta <= alpha)
                        return minEval;
                }
            }

            return minEval == int.MaxValue ? 0 : minEval;
        }
    }

    private static int EvalDeltaBeforeMove(
        MarkType[,] board,
        MarkType botMark,
        int winCount,
        int x,
        int y,
        MarkType moveMark)
    {
        var height = board.GetLength(0);
        var width = board.GetLength(1);
        (int dx, int dy)[] directions = [(1, 0), (0, 1), (1, 1), (1, -1)];

        var delta = 0;

        foreach (var (dx, dy) in directions)
        {
            for (var offset = 0; offset < winCount; offset++)
            {
                var startX = x - dx * offset;
                var startY = y - dy * offset;
                var endX = startX + dx * (winCount - 1);
                var endY = startY + dy * (winCount - 1);

                if (startX < 0 || startY < 0 || endX < 0 || endY < 0 ||
                    startX >= width || startY >= height || endX >= width || endY >= height)
                    continue;

                var botCount = 0;
                var oppCount = 0;

                for (var i = 0; i < winCount; i++)
                {
                    var cx = startX + dx * i;
                    var cy = startY + dy * i;
                    var cell = board[cy, cx];

                    if (cell == botMark)
                        botCount++;
                    else if (cell != MarkType.Empty)
                        oppCount++;
                }

                var oldScore = oppCount == 0 ? botCount * botCount
                    : botCount == 0 ? -(oppCount * oppCount)
                    : 0;

                int newScore;
                if (moveMark == botMark)
                {
                    newScore = oppCount == 0 ? (botCount + 1) * (botCount + 1) : 0;
                }
                else
                {
                    newScore = botCount == 0 ? -((oppCount + 1) * (oppCount + 1)) : 0;
                }

                delta += newScore - oldScore;
            }
        }

        return delta;
    }

    private static int EvaluateBoard(MarkType[,] board, MarkType botMark, int winCount)
    {
        var opponentMark = GetOpponent(botMark);
        var botScore = CountThreats(board, botMark, winCount);
        var opponentScore = CountThreats(board, opponentMark, winCount);
        return botScore - opponentScore;
    }

    private static int CountThreats(MarkType[,] board, MarkType mark, int winCount)
    {
        var height = board.GetLength(0);
        if (height == 0)
            return 0;
        var width = board.GetLength(1);

        var score = 0;

        for (var y = 0; y < height; y++)
        {
            for (var x = 0; x < width; x++)
            {
                score += CheckLineThreat(board, x, y, 1, 0, mark, winCount);
                score += CheckLineThreat(board, x, y, 0, 1, mark, winCount);
                score += CheckLineThreat(board, x, y, 1, 1, mark, winCount);
                score += CheckLineThreat(board, x, y, 1, -1, mark, winCount);
            }
        }

        return score;
    }

    private static int CheckLineThreat(
        MarkType[,] board,
        int startX,
        int startY,
        int dx,
        int dy,
        MarkType mark,
        int winCount)
    {
        var height = board.GetLength(0);
        var width = board.GetLength(1);
        var last = winCount - 1;

        var endX = startX + dx * last;
        var endY = startY + dy * last;

        if (endX < 0 || endY < 0 || endX >= width || endY >= height)
            return 0;

        var count = 0;

        for (var i = 0; i < winCount; i++)
        {
            var cx = startX + dx * i;
            var cy = startY + dy * i;
            var cell = board[cy, cx];

            if (cell == mark)
                count++;
            else if (cell != MarkType.Empty)
                return 0;
        }

        return count * count;
    }
}
