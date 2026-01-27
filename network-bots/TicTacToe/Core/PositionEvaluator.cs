namespace MiniGameNetworkBot.TicTacToe.Core;

public static class PositionEvaluator
{
    private static readonly int[] Dx = [1, 0, 1, 1];
    private static readonly int[] Dy = [0, 1, 1, -1];

    public static int Evaluate(Mark[,] cells, int winCount, Mark perspective)
    {
        var height = cells.GetLength(0);
        var width = cells.GetLength(1);
        var opponent = perspective == Mark.X ? Mark.O : Mark.X;

        var myScore = CountThreats(cells, width, height, winCount, perspective);
        var oppScore = CountThreats(cells, width, height, winCount, opponent);

        return myScore - oppScore;
    }

    public static int EvaluateMove(Mark[,] cells, int winCount, int x, int y, Mark mark)
    {
        var height = cells.GetLength(0);
        var width = cells.GetLength(1);
        var score = 0;

        cells[y, x] = mark;

        for (var d = 0; d < 4; d++)
        {
            var (count, openEnds) = CountLineAt(cells, width, height, winCount, x, y, d, mark);

            if (count >= winCount)
            {
                cells[y, x] = Mark.Empty;
                return 10000;
            }

            if (count == winCount - 1)
            {
                if (openEnds >= 2)
                    score += 1000;
                else if (openEnds == 1)
                    score += 100;
            }
            else if (count == winCount - 2)
            {
                if (openEnds >= 2)
                    score += 50;
                else if (openEnds == 1)
                    score += 10;
            }
            else if (count >= 2)
            {
                score += count * openEnds;
            }
        }

        var centerX = width / 2;
        var centerY = height / 2;
        var distToCenter = Math.Abs(x - centerX) + Math.Abs(y - centerY);
        score += Math.Max(0, 5 - distToCenter);

        cells[y, x] = Mark.Empty;
        return score;
    }

    private static int CountThreats(Mark[,] cells, int width, int height, int winCount, Mark mark)
    {
        var score = 0;

        for (var y = 0; y < height; y++)
        {
            for (var x = 0; x < width; x++)
            {
                if (cells[y, x] != mark)
                    continue;

                for (var d = 0; d < 4; d++)
                {
                    var (count, openEnds) = CountLineFromStart(cells, width, height, winCount, x, y, d, mark);

                    if (count >= winCount)
                        score += 10000;
                    else if (count == winCount - 1 && openEnds >= 2)
                        score += 500;
                    else if (count == winCount - 1 && openEnds == 1)
                        score += 50;
                    else if (count == winCount - 2 && openEnds >= 2)
                        score += 25;
                    else if (count >= 2 && openEnds > 0)
                        score += count * count;
                }
            }
        }

        return score;
    }

    private static (int Count, int OpenEnds) CountLineAt(Mark[,] cells, int width, int height, int winCount, int x, int y, int d, Mark mark)
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
        if (checkX >= 0 && checkY >= 0 && checkX < width && checkY < height && cells[checkY, checkX] == Mark.Empty)
            openEnds++;

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
        if (checkX >= 0 && checkY >= 0 && checkX < width && checkY < height && cells[checkY, checkX] == Mark.Empty)
            openEnds++;

        return (count, openEnds);
    }

    private static (int Count, int OpenEnds) CountLineFromStart(Mark[,] cells, int width, int height, int winCount, int startX, int startY, int d, Mark mark)
    {
        var count = 0;
        var openBefore = false;
        var openAfter = false;

        var beforeX = startX - Dx[d];
        var beforeY = startY - Dy[d];
        if (beforeX >= 0 && beforeY >= 0 && beforeX < width && beforeY < height)
        {
            if (cells[beforeY, beforeX] == mark)
                return (0, 0);
            openBefore = cells[beforeY, beforeX] == Mark.Empty;
        }

        for (var i = 0; i < winCount; i++)
        {
            var nx = startX + Dx[d] * i;
            var ny = startY + Dy[d] * i;
            if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                break;
            if (cells[ny, nx] != mark)
            {
                if (cells[ny, nx] == Mark.Empty)
                    openAfter = true;
                break;
            }
            count++;
        }

        if (count == 0)
            return (0, 0);

        var afterX = startX + Dx[d] * count;
        var afterY = startY + Dy[d] * count;
        if (afterX >= 0 && afterY >= 0 && afterX < width && afterY < height && cells[afterY, afterX] == Mark.Empty)
            openAfter = true;

        var openEnds = (openBefore ? 1 : 0) + (openAfter ? 1 : 0);
        return (count, openEnds);
    }
}
