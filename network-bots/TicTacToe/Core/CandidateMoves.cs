namespace MiniGameNetworkBot.TicTacToe.Core;

/// <summary>
/// Move generation for the C# bots: empty cells within two cells of a mark (the centre on an
/// empty board). A search heuristic, not a rule: the core accepts any empty cell.
/// </summary>
public static class CandidateMoves
{
    public static List<(int X, int Y)> Around(Mark[,] cells)
    {
        var height = cells.GetLength(0);
        var width = cells.GetLength(1);
        var moves = new List<(int X, int Y)>();
        var seen = new bool[height, width];
        var anyMark = false;

        for (var y = 0; y < height; y++)
        {
            for (var x = 0; x < width; x++)
            {
                if (cells[y, x] == Mark.Empty)
                    continue;
                anyMark = true;

                for (var dy = -2; dy <= 2; dy++)
                {
                    for (var dx = -2; dx <= 2; dx++)
                    {
                        var nx = x + dx;
                        var ny = y + dy;
                        if (nx < 0 || ny < 0 || nx >= width || ny >= height || seen[ny, nx] || cells[ny, nx] != Mark.Empty)
                            continue;
                        seen[ny, nx] = true;
                        moves.Add((nx, ny));
                    }
                }
            }
        }

        return anyMark ? moves : [(width / 2, height / 2)];
    }
}
