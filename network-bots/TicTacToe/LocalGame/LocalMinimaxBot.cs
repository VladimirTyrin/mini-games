namespace MiniGameNetworkBot.TicTacToe.LocalGame;

public sealed class LocalMinimaxBot : ILocalBot
{
    private readonly int _maxDepth;

    public string Name => $"Minimax(d={_maxDepth})";

    public LocalMinimaxBot(int maxDepth = 2)
    {
        _maxDepth = maxDepth;
    }

    public (int X, int Y) GetMove(GameEngine engine)
    {
        var moves = engine.GetAvailableMoves();
        if (moves.Count == 0)
            return (0, 0);

        // Check for immediate win
        foreach (var (x, y) in moves)
        {
            if (TacticsEngine.IsWinningMove(engine, x, y, engine.CurrentPlayer))
                return (x, y);
        }

        // Check for immediate block
        var opponent = engine.CurrentPlayer == Mark.X ? Mark.O : Mark.X;
        foreach (var (x, y) in moves)
        {
            if (TacticsEngine.IsWinningMove(engine, x, y, opponent))
                return (x, y);
        }

        var bestMove = moves[0];
        var bestScore = int.MinValue;

        foreach (var (x, y) in moves)
        {
            var clone = engine.Clone();
            clone.Place(x, y);
            var score = -Negamax(clone, _maxDepth - 1, int.MinValue, int.MaxValue);

            if (score > bestScore)
            {
                bestScore = score;
                bestMove = (x, y);
            }
        }

        return bestMove;
    }

    private int Negamax(GameEngine engine, int depth, int alpha, int beta)
    {
        if (engine.IsGameOver)
        {
            if (engine.Winner == null)
                return 0;
            return engine.Winner == engine.CurrentPlayer ? 1000 + depth : -1000 - depth;
        }

        if (depth <= 0)
            return Evaluate(engine);

        var moves = engine.GetAvailableMoves();
        if (moves.Count == 0)
            return 0;

        var bestScore = int.MinValue;

        foreach (var (x, y) in moves)
        {
            var clone = engine.Clone();
            clone.Place(x, y);
            var score = -Negamax(clone, depth - 1, -beta, -alpha);

            bestScore = Math.Max(bestScore, score);
            alpha = Math.Max(alpha, score);

            if (alpha >= beta)
                break;
        }

        return bestScore;
    }

    private int Evaluate(GameEngine engine)
    {
        var myMark = engine.CurrentPlayer;
        var oppMark = myMark == Mark.X ? Mark.O : Mark.X;

        return CountThreats(engine, myMark) - CountThreats(engine, oppMark);
    }

    private int CountThreats(GameEngine engine, Mark mark)
    {
        var score = 0;
        int[] dx = [1, 0, 1, 1];
        int[] dy = [0, 1, 1, -1];

        for (var y = 0; y < engine.Height; y++)
        {
            for (var x = 0; x < engine.Width; x++)
            {
                for (var d = 0; d < 4; d++)
                {
                    var count = 0;
                    var empty = 0;

                    for (var i = 0; i < engine.WinCount; i++)
                    {
                        var nx = x + dx[d] * i;
                        var ny = y + dy[d] * i;

                        if (nx < 0 || ny < 0 || nx >= engine.Width || ny >= engine.Height)
                            break;

                        var cell = engine.GetCell(nx, ny);
                        if (cell == mark)
                            count++;
                        else if (cell == Mark.Empty)
                            empty++;
                        else
                            break;
                    }

                    if (count + empty >= engine.WinCount && count > 0)
                        score += count * count;
                }
            }
        }

        return score;
    }

}
