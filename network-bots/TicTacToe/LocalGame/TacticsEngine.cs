namespace MiniGameNetworkBot.TicTacToe.LocalGame;

public static class TacticsEngine
{
    private static readonly int[] Dx = [1, 0, 1, 1];
    private static readonly int[] Dy = [0, 1, 1, -1];

    public static (int X, int Y)? GetTacticalMove(IBoardView board, Mark myMark)
    {
        var oppMark = myMark == Mark.X ? Mark.O : Mark.X;
        var availableMoves = board.GetAvailableMoves();
        var cells = board.ToArray();
        var winCount = board.WinCount;

        if (availableMoves.Count == 0)
            return null;

        // 1. Win
        foreach (var (x, y) in availableMoves)
        {
            if (IsWinningMove(cells, winCount, x, y, myMark))
                return (x, y);
        }

        // 2. Block win
        foreach (var (x, y) in availableMoves)
        {
            if (IsWinningMove(cells, winCount, x, y, oppMark))
                return (x, y);
        }

        // 3. Create open four (4 with 2 open ends - guaranteed win)
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesOpenN(cells, board.Width, board.Height, winCount, x, y, myMark, winCount - 1))
                return (x, y);
        }

        // 4. Block opponent's open four
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesOpenN(cells, board.Width, board.Height, winCount, x, y, oppMark, winCount - 1))
                return (x, y);
        }

        // 5. Block opponent's four (with at least 1 open end)
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesNInRow(cells, board.Width, board.Height, winCount, x, y, oppMark, winCount - 1))
                return (x, y);
        }

        // 6. Block opponent's double threat FIRST
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesDoubleThreat(cells, winCount, x, y, oppMark, availableMoves))
                return (x, y);
        }

        // 7. Create our double threat
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesDoubleThreat(cells, winCount, x, y, myMark, availableMoves))
                return (x, y);
        }

        // 8. Create open three (leads to open four next move)
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesOpenN(cells, board.Width, board.Height, winCount, x, y, myMark, winCount - 2))
                return (x, y);
        }

        // 9. Block opponent's open three
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesOpenN(cells, board.Width, board.Height, winCount, x, y, oppMark, winCount - 2))
                return (x, y);
        }

        return null;
    }

    public static bool IsWinningMove(IBoardView board, int x, int y, Mark mark)
    {
        return IsWinningMove(board.Width, board.Height, board.WinCount, board.GetCell, x, y, mark);
    }

    public static bool IsWinningMove(Mark[,] cells, int winCount, int x, int y, Mark mark)
    {
        var height = cells.GetLength(0);
        var width = cells.GetLength(1);
        return IsWinningMove(width, height, winCount, (cx, cy) => cells[cy, cx], x, y, mark);
    }

    private static bool IsWinningMove(int width, int height, int winCount, Func<int, int, Mark> getCell, int x, int y, Mark mark)
    {
        for (var d = 0; d < 4; d++)
        {
            var count = 1;

            for (var i = 1; i < winCount; i++)
            {
                var nx = x + Dx[d] * i;
                var ny = y + Dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (getCell(nx, ny) != mark)
                    break;
                count++;
            }

            for (var i = 1; i < winCount; i++)
            {
                var nx = x - Dx[d] * i;
                var ny = y - Dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (getCell(nx, ny) != mark)
                    break;
                count++;
            }

            if (count >= winCount)
                return true;
        }

        return false;
    }

    private static bool CreatesOpenN(Mark[,] cells, int width, int height, int winCount, int x, int y, Mark mark, int requiredCount)
    {
        for (var d = 0; d < 4; d++)
        {
            var count = 1;
            var openEnds = 0;

            var posEnd = 1;
            for (var i = 1; i < winCount; i++)
            {
                var nx = x + Dx[d] * i;
                var ny = y + Dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (cells[ny, nx] != mark)
                    break;
                count++;
                posEnd = i + 1;
            }

            var checkX = x + Dx[d] * posEnd;
            var checkY = y + Dy[d] * posEnd;
            if (checkX >= 0 && checkY >= 0 && checkX < width && checkY < height &&
                cells[checkY, checkX] == Mark.Empty)
            {
                openEnds++;
            }

            var negEnd = 1;
            for (var i = 1; i < winCount; i++)
            {
                var nx = x - Dx[d] * i;
                var ny = y - Dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (cells[ny, nx] != mark)
                    break;
                count++;
                negEnd = i + 1;
            }

            checkX = x - Dx[d] * negEnd;
            checkY = y - Dy[d] * negEnd;
            if (checkX >= 0 && checkY >= 0 && checkX < width && checkY < height &&
                cells[checkY, checkX] == Mark.Empty)
            {
                openEnds++;
            }

            if (count >= requiredCount && openEnds >= 2)
                return true;
        }

        return false;
    }

    private static bool CreatesNInRow(Mark[,] cells, int width, int height, int winCount, int x, int y, Mark mark, int requiredCount)
    {
        for (var d = 0; d < 4; d++)
        {
            var count = 1;

            for (var i = 1; i < winCount; i++)
            {
                var nx = x + Dx[d] * i;
                var ny = y + Dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (cells[ny, nx] != mark)
                    break;
                count++;
            }

            for (var i = 1; i < winCount; i++)
            {
                var nx = x - Dx[d] * i;
                var ny = y - Dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (cells[ny, nx] != mark)
                    break;
                count++;
            }

            if (count >= requiredCount)
                return true;
        }

        return false;
    }

    private static bool CreatesDoubleThreat(Mark[,] cells, int winCount, int x, int y, Mark mark, List<(int X, int Y)> availableMoves)
    {
        cells[y, x] = mark;

        var winningMoves = 0;
        foreach (var (mx, my) in availableMoves)
        {
            if (mx == x && my == y)
                continue;
            if (cells[my, mx] != Mark.Empty)
                continue;

            if (IsWinningMove(cells, winCount, mx, my, mark))
                winningMoves++;

            if (winningMoves >= 2)
            {
                cells[y, x] = Mark.Empty;
                return true;
            }
        }

        cells[y, x] = Mark.Empty;
        return false;
    }
}
