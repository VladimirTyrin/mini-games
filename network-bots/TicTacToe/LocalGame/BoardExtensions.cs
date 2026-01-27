namespace MiniGameNetworkBot.TicTacToe.LocalGame;

public static class BoardExtensions
{
    public static float[] GetBoardStateForNetwork(this IBoardView board)
    {
        return GetBoardStateForNetwork(board, board.CurrentPlayer);
    }

    public static float[] GetBoardStateForNetwork(this IBoardView board, Mark perspective)
    {
        var state = new float[3 * board.Height * board.Width];
        var myMark = perspective;
        var oppMark = perspective == Mark.X ? Mark.O : Mark.X;

        for (var y = 0; y < board.Height; y++)
        {
            for (var x = 0; x < board.Width; x++)
            {
                var idx = y * board.Width + x;
                var cell = board.GetCell(x, y);

                if (cell == myMark)
                    state[idx] = 1f;
                else if (cell == oppMark)
                    state[board.Height * board.Width + idx] = 1f;
                else
                    state[2 * board.Height * board.Width + idx] = 1f;
            }
        }

        return state;
    }
}
