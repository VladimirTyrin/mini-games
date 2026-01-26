namespace MiniGameNetworkBot.TicTacToe.LocalGame;

public sealed class HybridNeuralLocalBot : ILocalBot
{
    private readonly PolicyValueNetwork _network;
    private readonly int _winCount;

    public string Name => "HybridNeural";

    public HybridNeuralLocalBot(PolicyValueNetwork network, int winCount = 5)
    {
        _network = network;
        _winCount = winCount;
    }

    public (int X, int Y) GetMove(GameEngine engine)
    {
        var myMark = engine.CurrentPlayer;
        var oppMark = myMark == Mark.X ? Mark.O : Mark.X;
        var availableMoves = engine.GetAvailableMoves();

        if (availableMoves.Count == 0)
            return (0, 0);

        // 1. Check for immediate win
        foreach (var (x, y) in availableMoves)
        {
            if (IsWinningMove(engine, x, y, myMark))
                return (x, y);
        }

        // 2. Block opponent's immediate win
        foreach (var (x, y) in availableMoves)
        {
            if (IsWinningMove(engine, x, y, oppMark))
                return (x, y);
        }

        // 3. Create open threat
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesOpenThreat(engine, x, y, myMark))
                return (x, y);
        }

        // 4. Block opponent's open threat
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesOpenThreat(engine, x, y, oppMark))
                return (x, y);
        }

        // 5. Block double threat (opponent move that creates 2+ winning moves)
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesDoubleThreat(engine, x, y, oppMark, availableMoves))
                return (x, y);
        }

        // 6. Use neural network for strategic move
        var state = engine.GetBoardState(myMark);
        var (policy, _) = _network.Predict(state);

        var bestMove = availableMoves[0];
        var bestProb = float.MinValue;

        foreach (var (x, y) in availableMoves)
        {
            var prob = policy[y * engine.Width + x];
            if (prob > bestProb)
            {
                bestProb = prob;
                bestMove = (x, y);
            }
        }

        return bestMove;
    }

    private bool IsWinningMove(GameEngine engine, int x, int y, Mark mark)
    {
        int[] dx = [1, 0, 1, 1];
        int[] dy = [0, 1, 1, -1];

        for (var d = 0; d < 4; d++)
        {
            var count = 1;

            for (var i = 1; i < _winCount; i++)
            {
                var nx = x + dx[d] * i;
                var ny = y + dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= engine.Width || ny >= engine.Height)
                    break;
                if (engine.GetCell(nx, ny) != mark)
                    break;
                count++;
            }

            for (var i = 1; i < _winCount; i++)
            {
                var nx = x - dx[d] * i;
                var ny = y - dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= engine.Width || ny >= engine.Height)
                    break;
                if (engine.GetCell(nx, ny) != mark)
                    break;
                count++;
            }

            if (count >= _winCount)
                return true;
        }

        return false;
    }

    private bool CreatesOpenThreat(GameEngine engine, int x, int y, Mark mark)
    {
        int[] dx = [1, 0, 1, 1];
        int[] dy = [0, 1, 1, -1];

        for (var d = 0; d < 4; d++)
        {
            var count = 1;
            var openEnds = 0;

            var posEnd = 1;
            for (var i = 1; i < _winCount; i++)
            {
                var nx = x + dx[d] * i;
                var ny = y + dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= engine.Width || ny >= engine.Height)
                    break;
                if (engine.GetCell(nx, ny) != mark)
                    break;
                count++;
                posEnd = i + 1;
            }

            var checkX = x + dx[d] * posEnd;
            var checkY = y + dy[d] * posEnd;
            if (checkX >= 0 && checkY >= 0 && checkX < engine.Width && checkY < engine.Height &&
                engine.GetCell(checkX, checkY) == Mark.Empty)
            {
                openEnds++;
            }

            var negEnd = 1;
            for (var i = 1; i < _winCount; i++)
            {
                var nx = x - dx[d] * i;
                var ny = y - dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= engine.Width || ny >= engine.Height)
                    break;
                if (engine.GetCell(nx, ny) != mark)
                    break;
                count++;
                negEnd = i + 1;
            }

            checkX = x - dx[d] * negEnd;
            checkY = y - dy[d] * negEnd;
            if (checkX >= 0 && checkY >= 0 && checkX < engine.Width && checkY < engine.Height &&
                engine.GetCell(checkX, checkY) == Mark.Empty)
            {
                openEnds++;
            }

            if (count >= _winCount - 1 && openEnds >= 2)
                return true;
        }

        return false;
    }

    private bool CreatesDoubleThreat(GameEngine engine, int x, int y, Mark mark, List<(int X, int Y)> availableMoves)
    {
        // Simulate placing the mark
        var clone = engine.Clone();
        clone.Place(x, y);

        // Count how many winning moves this creates
        var winningMoves = 0;
        foreach (var (mx, my) in availableMoves)
        {
            if (mx == x && my == y)
                continue;
            if (clone.GetCell(mx, my) != Mark.Empty)
                continue;

            if (IsWinningMove(clone, mx, my, mark))
                winningMoves++;

            if (winningMoves >= 2)
                return true;
        }

        return false;
    }
}
